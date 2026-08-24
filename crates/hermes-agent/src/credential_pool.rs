//! Deterministic credential-pool selection and rotation from
//! `agent/credential_pool.py`.
//!
//! This slice keeps auth-store persistence orchestration, OAuth refresh,
//! environment seeding, leases, and cross-process locking out of the core
//! state machine. It does include the source-compatible row serialization and
//! borrowed-secret disk boundary so callers can safely load and persist an
//! entry once those higher-level seams are ported.

use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use chrono::{DateTime, NaiveDateTime};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Pool entry status values persisted by the Python implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialStatus {
    Ok,
    Exhausted,
    Dead,
}

impl CredentialStatus {
    fn from_source(value: &str) -> Option<Self> {
        match value {
            "ok" => Some(Self::Ok),
            "exhausted" => Some(Self::Exhausted),
            "dead" => Some(Self::Dead),
            _ => None,
        }
    }

    fn as_source(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Exhausted => "exhausted",
            Self::Dead => "dead",
        }
    }
}

/// Selection strategies supported by the deterministic core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolStrategy {
    FillFirst,
    RoundRobin,
    LeastUsed,
}

/// A loaded credential-pool row and the fields used by selection/rotation.
#[derive(Debug, Clone, PartialEq)]
pub struct PooledCredential {
    pub provider: String,
    pub id: String,
    pub label: String,
    pub auth_type: String,
    pub priority: i32,
    pub source: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Source-compatible persisted status spelling.
    pub last_status: Option<String>,
    /// Compatibility view used by the in-memory selection core.
    pub status: Option<CredentialStatus>,
    pub last_status_at: Option<f64>,
    pub last_error_code: Option<u16>,
    pub last_error_reason: Option<String>,
    pub last_error_message: Option<String>,
    pub last_error_reset_at: Option<f64>,
    pub base_url: Option<String>,
    pub expires_at: Option<String>,
    pub expires_at_ms: Option<i64>,
    pub last_refresh: Option<String>,
    pub inference_base_url: Option<String>,
    pub agent_key: Option<String>,
    pub agent_key_expires_at: Option<String>,
    pub failure_reason: Option<String>,
    pub request_count: u64,
    /// JSON-only metadata retained by the upstream model's `_EXTRA_KEYS`.
    pub extra: Map<String, Value>,
}

impl PooledCredential {
    /// Construct a fresh API-key-style entry with source defaults.
    pub fn new(provider: &str, id: &str, access_token: &str, priority: i32) -> Self {
        Self {
            provider: provider.into(),
            id: id.into(),
            label: id.into(),
            auth_type: "api_key".into(),
            priority,
            source: "manual".into(),
            access_token: access_token.into(),
            refresh_token: None,
            last_status: None,
            status: None,
            last_status_at: None,
            last_error_code: None,
            last_error_reason: None,
            last_error_message: None,
            last_error_reset_at: None,
            base_url: None,
            expires_at: None,
            expires_at_ms: None,
            last_refresh: None,
            inference_base_url: None,
            agent_key: None,
            agent_key_expires_at: None,
            failure_reason: None,
            request_count: 0,
            extra: Map::new(),
        }
    }

    /// Rehydrate one credential-pool row from the source's dictionary shape.
    ///
    /// PARITY: agent/credential_pool.py `PooledCredential.from_dict`.
    pub fn from_dict(provider: &str, payload: &Map<String, Value>) -> Self {
        let access_token = payload
            .get("access_token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let last_status = payload
            .get("last_status")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let status = last_status
            .as_deref()
            .and_then(CredentialStatus::from_source);
        let mut extra = Map::new();
        for key in EXTRA_KEYS {
            if let Some(value) = payload.get(*key).filter(|value| !value.is_null()) {
                extra.insert((*key).to_owned(), value.clone());
            }
        }

        Self {
            provider: provider.to_owned(),
            id: payload
                .get("id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .unwrap_or_else(generated_credential_id),
            label: payload
                .get("label")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .or_else(|| {
                    payload
                        .get("source")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .unwrap_or_else(|| provider.to_owned()),
            auth_type: normalize_pool_auth_type(
                provider,
                &access_token,
                payload
                    .get("auth_type")
                    .and_then(Value::as_str)
                    .unwrap_or("api_key"),
            ),
            priority: payload
                .get("priority")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok())
                .unwrap_or(0),
            source: payload
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("manual")
                .to_owned(),
            access_token,
            refresh_token: optional_string(payload, "refresh_token"),
            last_status,
            status,
            last_status_at: source_timestamp(payload.get("last_status_at")),
            last_error_code: payload
                .get("last_error_code")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok()),
            last_error_reason: optional_string(payload, "last_error_reason"),
            last_error_message: optional_string(payload, "last_error_message"),
            last_error_reset_at: payload.get("last_error_reset_at").and_then(Value::as_f64),
            base_url: optional_string(payload, "base_url"),
            expires_at: optional_string(payload, "expires_at"),
            expires_at_ms: payload.get("expires_at_ms").and_then(Value::as_i64),
            last_refresh: optional_string(payload, "last_refresh"),
            inference_base_url: optional_string(payload, "inference_base_url"),
            agent_key: optional_string(payload, "agent_key"),
            agent_key_expires_at: optional_string(payload, "agent_key_expires_at"),
            failure_reason: optional_string(payload, "failure_reason"),
            request_count: payload
                .get("request_count")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            extra,
        }
    }

    /// Rehydrate one credential-pool row from a JSON object, failing open to
    /// source defaults for non-object values.
    pub fn from_json(provider: &str, payload: &Value) -> Self {
        payload.as_object().map_or_else(
            || Self::from_dict(provider, &Map::new()),
            |object| Self::from_dict(provider, object),
        )
    }

    /// Serialize one row to the source's dictionary shape.
    ///
    /// PARITY: agent/credential_pool.py `PooledCredential.to_dict`.
    pub fn to_dict(&self) -> Map<String, Value> {
        let mut result = Map::new();
        insert_string(&mut result, "id", &self.id);
        insert_string(&mut result, "label", &self.label);
        insert_string(&mut result, "auth_type", &self.auth_type);
        result.insert("priority".into(), Value::from(self.priority));
        insert_string(&mut result, "source", &self.source);
        insert_string(&mut result, "access_token", &self.access_token);
        insert_optional_string_if_some(&mut result, "refresh_token", self.refresh_token.as_deref());

        let status = self.last_status.clone().or_else(|| {
            self.status
                .map(CredentialStatus::as_source)
                .map(str::to_owned)
        });
        insert_optional_string(&mut result, "last_status", status.as_deref());
        result.insert(
            "last_status_at".into(),
            self.last_status_at.map_or(Value::Null, Value::from),
        );
        result.insert(
            "last_error_code".into(),
            self.last_error_code.map_or(Value::Null, Value::from),
        );
        insert_optional_string(
            &mut result,
            "last_error_reason",
            self.last_error_reason.as_deref(),
        );
        insert_optional_string(
            &mut result,
            "last_error_message",
            self.last_error_message.as_deref(),
        );
        result.insert(
            "last_error_reset_at".into(),
            self.last_error_reset_at.map_or(Value::Null, Value::from),
        );
        insert_optional_string_if_some(&mut result, "base_url", self.base_url.as_deref());
        insert_optional_string_if_some(&mut result, "expires_at", self.expires_at.as_deref());
        if let Some(expires_at_ms) = self.expires_at_ms {
            result.insert("expires_at_ms".into(), Value::from(expires_at_ms));
        }
        insert_optional_string_if_some(&mut result, "last_refresh", self.last_refresh.as_deref());
        insert_optional_string_if_some(
            &mut result,
            "inference_base_url",
            self.inference_base_url.as_deref(),
        );
        insert_optional_string_if_some(&mut result, "agent_key", self.agent_key.as_deref());
        insert_optional_string_if_some(
            &mut result,
            "agent_key_expires_at",
            self.agent_key_expires_at.as_deref(),
        );
        insert_optional_string_if_some(
            &mut result,
            "failure_reason",
            self.failure_reason.as_deref(),
        );
        result.insert("request_count".into(), Value::from(self.request_count));
        for (key, value) in &self.extra {
            if !value.is_null() {
                result.insert(key.clone(), value.clone());
            }
        }
        sanitize_borrowed_credential_payload(result, &self.provider)
    }

    pub fn to_json(&self) -> Value {
        Value::Object(self.to_dict())
    }

    /// Return the runtime credential, including Nous's invoke-JWT guard.
    ///
    /// PARITY: agent/credential_pool.py `runtime_api_key`.
    pub fn runtime_api_key(&self) -> String {
        if self.provider == "nous" {
            let scope = self.extra.get("scope");
            for (token, expires_at) in [
                (
                    self.agent_key.as_deref(),
                    self.agent_key_expires_at.as_deref(),
                ),
                (
                    self.access_token.as_str().into(),
                    self.expires_at.as_deref(),
                ),
            ] {
                let Some(token) = token.filter(|value| !value.trim().is_empty()) else {
                    continue;
                };
                if nous_invoke_jwt_is_usable(token, scope, expires_at) {
                    return token.trim().to_owned();
                }
            }
            return String::new();
        }
        self.access_token.clone()
    }

    /// Return the provider runtime base URL.
    ///
    /// PARITY: agent/credential_pool.py `runtime_base_url`.
    pub fn runtime_base_url(&self) -> Option<&str> {
        if self.provider == "nous" {
            self.inference_base_url
                .as_deref()
                .or(self.base_url.as_deref())
        } else {
            self.base_url.as_deref()
        }
    }

    fn set_status(&mut self, status: CredentialStatus) {
        self.status = Some(status);
        self.last_status = Some(status.as_source().to_owned());
    }
}

const EXTRA_KEYS: &[&str] = &[
    "token_type",
    "scope",
    "client_id",
    "portal_base_url",
    "obtained_at",
    "expires_in",
    "agent_key_id",
    "agent_key_expires_in",
    "agent_key_reused",
    "agent_key_obtained_at",
    "tls",
    "secret_source",
    "secret_fingerprint",
    "failure_reason",
];

fn optional_string(payload: &Map<String, Value>, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn source_timestamp(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(value)) => parse_absolute_timestamp(value),
        _ => None,
    }
}

fn parse_absolute_timestamp(value: &str) -> Option<f64> {
    let raw = value.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(numeric) = raw.parse::<f64>() {
        return (numeric > 0.0).then_some(if numeric > 1_000_000_000_000.0 {
            numeric / 1000.0
        } else {
            numeric
        });
    }
    DateTime::parse_from_rfc3339(raw)
        .map(|timestamp| timestamp.timestamp_millis() as f64 / 1000.0)
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|timestamp| timestamp.and_utc().timestamp_millis() as f64 / 1000.0)
        })
}

fn normalize_pool_auth_type(provider: &str, token: &str, auth_type: &str) -> String {
    if provider == "anthropic" && token.starts_with("sk-ant-oat") {
        "oauth".into()
    } else if auth_type.is_empty() {
        "api_key".into()
    } else {
        auth_type.into()
    }
}

fn insert_string(result: &mut Map<String, Value>, key: &str, value: &str) {
    result.insert(key.into(), Value::String(value.into()));
}

fn insert_optional_string(result: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    result.insert(
        key.into(),
        value.map_or(Value::Null, |value| Value::String(value.into())),
    );
}

fn insert_optional_string_if_some(result: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        result.insert(key.into(), Value::String(value.into()));
    }
}

static FALLBACK_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn generated_credential_id() -> String {
    // PARITY: uuid.uuid4().hex[:6]. The standard library fallback keeps this
    // model dependency-light while retaining the six-hex-character contract.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos() as u64);
    let sequence = FALLBACK_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:06x}", (now ^ sequence) & 0x00ff_ffff)
}

fn decode_jwt_claims(token: &str) -> Option<Map<String, Value>> {
    if token.matches('.').count() != 2 {
        return None;
    }
    let payload = token.split('.').nth(1)?;
    let mut encoded = payload.to_owned();
    encoded.push_str(&"=".repeat((4 - encoded.len() % 4) % 4));
    let decoded = URL_SAFE.decode(encoded.as_bytes()).ok()?;
    serde_json::from_slice::<Value>(&decoded)
        .ok()?
        .as_object()
        .cloned()
}

fn scope_values(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(value)) => value
            .replace(',', " ")
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect(),
        Some(Value::Array(values)) => values
            .iter()
            .flat_map(|value| scope_values(Some(value)))
            .collect(),
        _ => Vec::new(),
    }
}

fn nous_invoke_jwt_is_usable(
    token: &str,
    stored_scope: Option<&Value>,
    expires_at: Option<&str>,
) -> bool {
    let Some(claims) = decode_jwt_claims(token) else {
        return false;
    };
    let mut scopes = scope_values(stored_scope);
    scopes.extend(scope_values(claims.get("scope")));
    scopes.extend(scope_values(claims.get("scp")));
    if !scopes.iter().any(|scope| scope == "inference:invoke") {
        return false;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |duration| duration.as_secs_f64());
    if let Some(exp) = claims.get("exp").and_then(Value::as_f64) {
        return exp > now + 120.0;
    }
    expires_at
        .and_then(parse_absolute_timestamp)
        .is_some_and(|expiry| expiry > now + 120.0)
}

/// Derive a human-readable label from standard JWT identity claims.
///
/// PARITY: agent/credential_pool.py `label_from_token`.
pub fn label_from_token(token: &str, fallback: &str) -> String {
    let Some(claims) = decode_jwt_claims(token) else {
        return fallback.to_owned();
    };
    ["email", "preferred_username", "upn"]
        .iter()
        .find_map(|key| {
            claims
                .get(*key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| fallback.to_owned())
}

fn normalize_key(key: &str) -> String {
    let mut normalized = String::with_capacity(key.len() + 4);
    let mut previous_is_lower_or_digit = false;
    for character in key.chars() {
        if character.is_ascii_uppercase() && previous_is_lower_or_digit {
            normalized.push('_');
        }
        if character == '-' || character == '.' {
            normalized.push('_');
        } else {
            normalized.push(character.to_ascii_lowercase());
        }
        previous_is_lower_or_digit = character.is_ascii_lowercase() || character.is_ascii_digit();
    }
    normalized
}

fn is_secret_payload_key(key: &str) -> bool {
    const SAFE: &[&str] = &[
        "secret_fingerprint",
        "secret_source",
        "token_type",
        "scope",
        "client_id",
        "agent_key_id",
        "agent_key_expires_at",
        "agent_key_expires_in",
        "agent_key_reused",
        "agent_key_obtained_at",
        "expires_at",
        "expires_at_ms",
        "expires_in",
        "last_refresh",
        "last_status",
        "last_status_at",
        "last_error_code",
        "last_error_reason",
        "last_error_message",
        "last_error_reset_at",
    ];
    if SAFE.contains(&key) {
        return false;
    }
    const SECRET: &[&str] = &[
        "access_token",
        "refresh_token",
        "agent_key",
        "api_key",
        "apikey",
        "api_token",
        "auth_token",
        "authorization",
        "bearer_token",
        "client_secret",
        "credential",
        "credentials",
        "id_token",
        "oauth_token",
        "private_key",
        "secret_key",
        "session_token",
        "password",
        "secret",
        "token",
        "tokens",
    ];
    SECRET.contains(&key)
        || [
            "_api_key",
            "_api_token",
            "_access_token",
            "_auth_token",
            "_refresh_token",
            "_bearer_token",
            "_client_secret",
            "_id_token",
            "_oauth_token",
            "_private_key",
            "_session_token",
            "_secret_key",
            "_password",
            "_secret",
            "_token",
            "_key",
        ]
        .iter()
        .any(|suffix| key.ends_with(suffix))
}

fn is_borrowed_credential_source(source: &str, provider: &str) -> bool {
    let source = source.trim().to_ascii_lowercase();
    if source.is_empty() || source == "manual" || source.starts_with("manual:") {
        return false;
    }
    !matches!(
        (
            provider.trim().to_ascii_lowercase().as_str(),
            source.as_str()
        ),
        ("anthropic", "hermes_pkce")
            | ("minimax-oauth", "oauth")
            | ("nous", "device_code")
            | ("openai-codex", "device_code")
            | ("xai-oauth", "device_code")
    )
}

fn fingerprint_value(value: &Value) -> Option<String> {
    let text = match value {
        Value::Null => return None,
        Value::String(value) => value.clone(),
        _ => serde_json::to_string(value).ok()?,
    };
    if text.is_empty() {
        return None;
    }
    let digest = Sha256::digest(text.as_bytes());
    let short = digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Some(format!("sha256:{short}"))
}

fn credential_secret_fingerprint(payload: &Map<String, Value>) -> Option<String> {
    for key in [
        "agent_key",
        "access_token",
        "refresh_token",
        "api_key",
        "token",
        "secret",
    ] {
        if let Some(fingerprint) = payload.get(key).and_then(fingerprint_value) {
            return Some(fingerprint);
        }
    }
    for (key, value) in payload {
        if is_secret_payload_key(&normalize_key(key)) {
            if let Some(fingerprint) = fingerprint_value(value) {
                return Some(fingerprint);
            }
        }
    }
    payload
        .get("secret_fingerprint")
        .and_then(Value::as_str)
        .filter(|value| value.starts_with("sha256:"))
        .map(ToOwned::to_owned)
}

fn sanitize_borrowed_credential_payload(
    payload: Map<String, Value>,
    provider: &str,
) -> Map<String, Value> {
    if !is_borrowed_credential_source(
        payload
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        provider,
    ) {
        return payload;
    }
    let fingerprint = credential_secret_fingerprint(&payload);
    let mut sanitized = Map::new();
    for (key, value) in payload {
        if !is_secret_payload_key(&normalize_key(&key)) {
            sanitized.insert(key, value);
        }
    }
    if let Some(fingerprint) = fingerprint {
        sanitized.insert("secret_fingerprint".into(), Value::String(fingerprint));
    }
    sanitized
}

/// Error metadata supplied to `mark_exhausted_and_rotate`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PoolErrorContext {
    pub reason: Option<String>,
    pub message: Option<String>,
    pub reset_at: Option<f64>,
}

/// In-memory credential pool state.
#[derive(Debug, Clone, PartialEq)]
pub struct CredentialPool {
    provider: String,
    entries: Vec<PooledCredential>,
    strategy: PoolStrategy,
    current_id: Option<String>,
    unmatched_rotation_streak: usize,
}

const EXHAUSTED_TTL_401_SECONDS: f64 = 5.0 * 60.0;
const EXHAUSTED_TTL_429_SECONDS: f64 = 60.0 * 60.0;
const EXHAUSTED_TTL_DEFAULT_SECONDS: f64 = 60.0 * 60.0;
const EXHAUSTED_TTL_SOLE_CREDENTIAL_SECONDS: f64 = 60.0;
const FAILURE_REASON_BILLING: &str = "billing";

const TERMINAL_AUTH_REASONS: &[&str] = &[
    "token_invalidated",
    "token_revoked",
    "invalid_token",
    "invalid_grant",
    "unauthorized_client",
    "refresh_token_reused",
];

impl CredentialPool {
    /// Create a pool sorted by the source priority order.
    ///
    /// PARITY: agent/credential_pool.py `CredentialPool.__init__`.
    pub fn new(provider: &str, mut entries: Vec<PooledCredential>, strategy: PoolStrategy) -> Self {
        entries.sort_by_key(|entry| entry.priority);
        Self {
            provider: provider.into(),
            entries,
            strategy,
            current_id: None,
            unmatched_rotation_streak: 0,
        }
    }

    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Return whether the pool has rows, including rows currently exhausted.
    ///
    /// PARITY: agent/credential_pool.py `has_credentials`.
    pub fn has_credentials(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Return a snapshot of the loaded rows.
    pub fn entries(&self) -> Vec<PooledCredential> {
        self.entries.clone()
    }

    /// Return the currently selected row without changing selection state.
    ///
    /// PARITY: agent/credential_pool.py `current`.
    pub fn current(&self) -> Option<PooledCredential> {
        self.current_id
            .as_deref()
            .and_then(|id| self.entries.iter().find(|entry| entry.id == id))
            .cloned()
    }

    /// Return the current row, or the first available row if nothing is
    /// selected. Expired cooldowns are not rewritten by a peek.
    ///
    /// PARITY: agent/credential_pool.py `peek`.
    pub fn peek(&self, now: f64) -> Option<PooledCredential> {
        if let Some(current) = self.current() {
            return Some(current);
        }
        self.available_indices_readonly(now)
            .first()
            .and_then(|index| self.entries.get(*index))
            .cloned()
    }

    /// Return whether any entry is available at `now`.
    ///
    /// PARITY: agent/credential_pool.py `has_available`.
    pub fn has_available(&self, now: f64) -> bool {
        !self.available_indices_readonly(now).is_empty()
    }

    /// Select a credential according to the configured strategy.
    ///
    /// `now` is explicit so Rust callers can test cooldown boundaries without
    /// patching process-global wall-clock state.
    ///
    /// PARITY: agent/credential_pool.py `_select_unlocked` / `select`.
    pub fn select(&mut self, now: f64) -> Option<PooledCredential> {
        let available = self.available_indices(now, true);
        self.select_from_available(available)
    }

    /// Mark the failed credential and return the next available credential.
    ///
    /// Credential identity takes precedence over the current cursor. An
    /// unmatched identity rotates without quarantining an innocent key, while
    /// a matched key quarantines every duplicate row backed by that key.
    ///
    /// PARITY: agent/credential_pool.py lines 2031-2165.
    #[allow(clippy::too_many_arguments)]
    pub fn mark_exhausted_and_rotate(
        &mut self,
        status_code: Option<u16>,
        error_context: Option<&PoolErrorContext>,
        api_key_hint: Option<&str>,
        credential_id: Option<&str>,
        failure_reason: Option<&str>,
        now: f64,
    ) -> Option<PooledCredential> {
        let identity_supplied = credential_id.is_some() || api_key_hint.is_some();
        let selected_index = credential_id
            .and_then(|id| self.entries.iter().position(|entry| entry.id == id))
            .or_else(|| {
                api_key_hint.and_then(|key| {
                    self.entries
                        .iter()
                        .position(|entry| entry.access_token == key)
                })
            });

        let Some(selected_index) = selected_index.or_else(|| {
            if identity_supplied {
                None
            } else {
                self.current_id
                    .as_deref()
                    .and_then(|id| self.entries.iter().position(|entry| entry.id == id))
                    .or_else(|| self.available_indices(now, false).first().copied())
            }
        }) else {
            if !identity_supplied {
                self.current_id = None;
                return None;
            }

            self.unmatched_rotation_streak += 1;
            let available = self.available_indices(now, false);
            if self.unmatched_rotation_streak > available.len().max(1) {
                self.unmatched_rotation_streak = 0;
                self.current_id = None;
                return None;
            }
            self.current_id = None;
            let next = self.select(now);
            if next.is_some() && self.available_indices_readonly(now).len() == 1 {
                self.unmatched_rotation_streak = 0;
                self.current_id = None;
                return None;
            }
            return next;
        };

        self.unmatched_rotation_streak = 0;
        let failed_key = self.entries[selected_index].access_token.clone();
        let duplicate_key = identity_supplied && !failed_key.is_empty();
        for index in 0..self.entries.len() {
            if index != selected_index
                && (!duplicate_key || self.entries[index].access_token != failed_key)
            {
                continue;
            }
            self.mark_entry(index, status_code, error_context, failure_reason, now);
        }
        self.current_id = None;
        self.select(now)
    }

    fn mark_entry(
        &mut self,
        index: usize,
        status_code: Option<u16>,
        error_context: Option<&PoolErrorContext>,
        failure_reason: Option<&str>,
        now: f64,
    ) {
        let entry = &mut self.entries[index];
        let reason = error_context
            .and_then(|context| context.reason.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let terminal = status_code == Some(401)
            && reason.as_deref().is_some_and(|value| {
                TERMINAL_AUTH_REASONS
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(value))
            });
        entry.set_status(if terminal {
            CredentialStatus::Dead
        } else {
            CredentialStatus::Exhausted
        });
        entry.last_status_at = Some(now);
        entry.last_error_code = status_code;
        entry.last_error_reason = reason;
        entry.last_error_message = error_context
            .and_then(|context| context.message.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        entry.last_error_reset_at = error_context.and_then(|context| context.reset_at);
        entry.failure_reason = failure_reason
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
    }

    fn select_from_available(&mut self, available: Vec<usize>) -> Option<PooledCredential> {
        let index = match self.strategy {
            PoolStrategy::FillFirst | PoolStrategy::RoundRobin => *available.first()?,
            PoolStrategy::LeastUsed => *available
                .iter()
                .min_by_key(|index| self.entries[**index].request_count)?,
        };

        if self.strategy == PoolStrategy::LeastUsed && available.len() > 1 {
            self.entries[index].request_count = self.entries[index].request_count.saturating_add(1);
        }

        if self.strategy == PoolStrategy::RoundRobin && available.len() > 1 {
            let selected = self.entries.remove(index);
            let selected_id = selected.id.clone();
            self.entries.push(selected);
            for (priority, entry) in self.entries.iter_mut().enumerate() {
                entry.priority = priority as i32;
            }
            self.current_id = Some(selected_id);
            return self.current();
        }

        self.current_id = Some(self.entries[index].id.clone());
        self.entries.get(index).cloned()
    }

    fn available_indices(&mut self, now: f64, clear_expired: bool) -> Vec<usize> {
        let sole_credential = self
            .entries
            .iter()
            .filter(|entry| entry.status != Some(CredentialStatus::Dead))
            .count()
            <= 1;
        let mut available = Vec::new();
        for index in 0..self.entries.len() {
            let entry = &mut self.entries[index];
            if (entry.auth_type == "api_key" && entry.access_token.trim().is_empty())
                || entry.status == Some(CredentialStatus::Dead)
            {
                continue;
            }
            if entry.status == Some(CredentialStatus::Exhausted) {
                if let Some(until) = exhausted_until(entry, sole_credential) {
                    if now < until {
                        continue;
                    }
                }
                if clear_expired {
                    clear_failure_state(entry);
                }
            }
            available.push(index);
        }
        available
    }

    fn available_indices_readonly(&self, now: f64) -> Vec<usize> {
        let sole_credential = self
            .entries
            .iter()
            .filter(|entry| entry.status != Some(CredentialStatus::Dead))
            .count()
            <= 1;
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                if (entry.auth_type == "api_key" && entry.access_token.trim().is_empty())
                    || entry.status == Some(CredentialStatus::Dead)
                {
                    return None;
                }
                if entry.status == Some(CredentialStatus::Exhausted)
                    && exhausted_until(entry, sole_credential).is_some_and(|until| now < until)
                {
                    return None;
                }
                Some(index)
            })
            .collect()
    }
}

fn clear_failure_state(entry: &mut PooledCredential) {
    entry.set_status(CredentialStatus::Ok);
    entry.last_status_at = None;
    entry.last_error_code = None;
    entry.last_error_reason = None;
    entry.last_error_message = None;
    entry.last_error_reset_at = None;
    entry.failure_reason = None;
}

fn exhausted_until(entry: &PooledCredential, sole_credential: bool) -> Option<f64> {
    if entry.status != Some(CredentialStatus::Exhausted) {
        return None;
    }
    if let Some(reset_at) = entry.last_error_reset_at {
        return Some(reset_at);
    }
    let status_at = entry.last_status_at?;
    let base = match entry.last_error_code {
        Some(401) => EXHAUSTED_TTL_401_SECONDS,
        Some(429) => EXHAUSTED_TTL_429_SECONDS,
        _ => EXHAUSTED_TTL_DEFAULT_SECONDS,
    };
    let billing = entry.last_error_code == Some(402)
        || entry.failure_reason.as_deref() == Some(FAILURE_REASON_BILLING);
    let ttl = if sole_credential && !billing {
        base.min(EXHAUSTED_TTL_SOLE_CREDENTIAL_SECONDS)
    } else {
        base
    };
    Some(status_at + ttl)
}
