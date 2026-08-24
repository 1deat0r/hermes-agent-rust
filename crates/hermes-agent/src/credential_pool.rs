//! Credential-pool selection, rotation, and source-compatible row helpers
//! from `agent/credential_pool.py`.
//!
//! This slice keeps auth-store persistence orchestration, OAuth refresh,
//! leases, and cross-process locking out of the core state machine. It
//! includes the source-compatible row serialization, borrowed-secret disk
//! boundary, and explicit environment/singleton seeding seams so the higher
//! auth/config layer can compose the full loader without an upward crate
//! dependency.

use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use rand::seq::IndexedRandom;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use hermes_constants::OPENROUTER_BASE_URL;

/// Authentication type values persisted by the Python implementation.
pub const AUTH_TYPE_API_KEY: &str = "api_key";
pub const AUTH_TYPE_OAUTH: &str = "oauth";

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

/// Selection strategies supported by the pool core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolStrategy {
    FillFirst,
    RoundRobin,
    LeastUsed,
    Random,
}

/// Provider metadata required by the environment seeding boundary.
///
/// The upstream `ProviderConfig` lives in the higher CLI/auth layer. Keeping
/// this small input type here preserves the bottom-up crate dependency while
/// allowing that layer to supply the exact registry values later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCredentialConfig {
    pub auth_type: String,
    pub api_key_env_vars: Vec<String>,
    pub base_url_env_var: Option<String>,
    pub inference_base_url: String,
}

impl ProviderCredentialConfig {
    pub fn api_key(
        api_key_env_vars: impl IntoIterator<Item = impl Into<String>>,
        base_url_env_var: Option<impl Into<String>>,
        inference_base_url: impl Into<String>,
    ) -> Self {
        Self {
            auth_type: AUTH_TYPE_API_KEY.into(),
            api_key_env_vars: api_key_env_vars.into_iter().map(Into::into).collect(),
            base_url_env_var: base_url_env_var.map(Into::into),
            inference_base_url: inference_base_url.into(),
        }
    }
}

/// Runtime inputs visible to `_seed_from_env`.
///
/// `dotenv` wins over `process` for normal values. `secret_scope` models the
/// upstream active secret resolver used when `.env` contains an unresolved
/// `op://` reference. The source label map is metadata only; it never grants
/// permission to persist the raw secret. `suppressed_sources` is supplied by
/// the auth layer because suppression state is owned outside this crate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvironmentSnapshot {
    pub dotenv: BTreeMap<String, String>,
    pub process: BTreeMap<String, String>,
    pub secret_scope: BTreeMap<String, String>,
    pub secret_sources: BTreeMap<String, String>,
    pub suppressed_sources: BTreeSet<String>,
}

impl EnvironmentSnapshot {
    /// Capture the current process environment while using a caller-supplied
    /// parsed `.env` map. This keeps tests deterministic and lets the higher
    /// config layer control its existing dotenv cache.
    pub fn from_process(dotenv: BTreeMap<String, String>) -> Self {
        Self {
            dotenv,
            process: std::env::vars().collect(),
            ..Self::default()
        }
    }

    /// Load `.env` from disk and capture the current process environment.
    pub fn from_dotenv_path(path: &Path) -> io::Result<Self> {
        Ok(Self::from_process(load_env_file(path)?))
    }

    fn preferred_value(&self, key: &str) -> String {
        let dotenv_value = self
            .dotenv
            .get(key)
            .map(String::as_str)
            .unwrap_or_default()
            .trim();
        let scoped_value = self
            .secret_scope
            .get(key)
            .or_else(|| self.process.get(key))
            .map(String::as_str)
            .unwrap_or_default()
            .trim();
        if dotenv_value.starts_with("op://") && !scoped_value.is_empty() {
            return scoped_value.to_owned();
        }
        if !dotenv_value.is_empty() {
            dotenv_value.to_owned()
        } else {
            scoped_value.to_owned()
        }
    }
}

/// Explicit inputs for the full `load_pool` composition boundary.
///
/// The owning auth/config layer constructs this view after resolving provider
/// singletons and configuration. Borrowing the values keeps this crate
/// independent of those higher-level representations while avoiding a wide
/// positional function signature.
pub struct PoolLoadInputs<'a> {
    pub provider_config: Option<&'a ProviderCredentialConfig>,
    pub pool_config: Option<&'a Map<String, Value>>,
    pub snapshot: &'a EnvironmentSnapshot,
    pub singleton_state: Option<&'a Map<String, Value>>,
    pub custom_providers: &'a [Value],
    pub model_config: Option<&'a Map<String, Value>>,
    pub profile_path: &'a Path,
    pub global_path: Option<&'a Path>,
}

/// Result of one source-seeding pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeedResult {
    pub changed: bool,
    pub active_sources: BTreeSet<String>,
}

/// Seed provider-owned singleton state without importing the higher auth
/// store into this crate.
///
/// PARITY: `agent/credential_pool.py._seed_from_singletons` (2453–2835),
/// currently the `anthropic`, `nous`, `copilot`, `qwen-oauth`, `minimax-oauth`,
/// `openai-codex`, and `xai-oauth` branches. The caller supplies the
/// already-resolved provider object and source suppression set; `None` means
/// the provider state/resolver result was absent, while
/// `Some(empty/object-without-runtime)` preserves each source branch's
/// fail-open behavior.
/// For `anthropic`, the resolved map also carries
/// `provider_explicitly_configured`, `api_key_path_explicit`, and the
/// `hermes_pkce`/`claude_code` credential-file results so this crate does not
/// import the CLI or adapter layers.
/// For `copilot`, the resolved map carries the final exchanged `api_token`,
/// resolver `source`, and optional enterprise endpoint; the CLI token
/// resolution and network exchange stay in the higher auth layer.
pub fn seed_from_singletons(
    provider: &str,
    entries: &mut Vec<PooledCredential>,
    state: Option<&Map<String, Value>>,
    suppressed_sources: &BTreeSet<String>,
) -> SeedResult {
    let mut result = SeedResult::default();

    if provider == "anthropic" {
        let Some(state) = state else {
            return result;
        };
        if !state
            .get("provider_explicitly_configured")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return result;
        }

        if state
            .get("api_key_path_explicit")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let original_len = entries.len();
            entries.retain(|entry| entry.source != "hermes_pkce" && entry.source != "claude_code");
            result.changed = entries.len() != original_len;
            return result;
        }

        for source in ["hermes_pkce", "claude_code"] {
            let Some(source_state) = state.get(source).and_then(Value::as_object) else {
                continue;
            };
            let Some(access_token) = source_state
                .get("accessToken")
                .and_then(Value::as_str)
                .filter(|token| !token.is_empty())
            else {
                continue;
            };
            if suppressed_sources.contains(source) {
                continue;
            }

            result.active_sources.insert(source.into());
            let mut payload = Map::new();
            payload.insert("source".into(), Value::String(source.into()));
            payload.insert("auth_type".into(), Value::String(AUTH_TYPE_OAUTH.into()));
            payload.insert(
                "access_token".into(),
                Value::String(access_token.to_owned()),
            );
            if let Some(value) = source_state
                .get("refreshToken")
                .filter(|value| !value.is_null())
            {
                payload.insert("refresh_token".into(), value.clone());
            }
            if let Some(expires_at_ms) = source_state.get("expiresAt").and_then(|value| {
                value
                    .as_i64()
                    .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
            }) {
                payload.insert("expires_at_ms".into(), Value::from(expires_at_ms));
            }
            payload.insert(
                "label".into(),
                Value::String(label_from_token(access_token, source)),
            );
            result.changed |= upsert_entry(entries, provider, source, &payload);
        }
        return result;
    }

    if provider == "copilot" {
        // PARITY: `_seed_from_singletons` (2605–2681) checks every known
        // Copilot source before spawning `gh auth token` or exchanging a token.
        // The resolved input map is the bottom-up equivalent of that adapter
        // boundary, so an all-suppressed pool load remains a no-op here.
        let all_sources = [
            "gh_cli",
            "env:COPILOT_GITHUB_TOKEN",
            "env:GH_TOKEN",
            "env:GITHUB_TOKEN",
        ];
        if all_sources
            .iter()
            .all(|source| suppressed_sources.contains(*source))
        {
            return result;
        }

        let Some(state) = state else {
            return result;
        };
        let Some(api_token) = state
            .get("api_token")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
        else {
            return result;
        };
        let Some(resolver_source) = state
            .get("source")
            .and_then(Value::as_str)
            .filter(|source| !source.is_empty())
        else {
            return result;
        };
        // The upstream resolver uses the exact CLI description for the gh
        // subprocess and the environment variable name for env resolution.
        let source = if resolver_source == "gh auth token" {
            "gh_cli".to_owned()
        } else {
            format!("env:{resolver_source}")
        };
        if suppressed_sources.contains(&source) {
            return result;
        }

        result.active_sources.insert(source.clone());
        let base_url = state
            .get("enterprise_base_url")
            .and_then(Value::as_str)
            .filter(|url| !url.is_empty())
            .unwrap_or("https://api.githubcopilot.com");
        let mut payload = Map::new();
        payload.insert("source".into(), Value::String(source.clone()));
        payload.insert("auth_type".into(), Value::String(AUTH_TYPE_API_KEY.into()));
        payload.insert("access_token".into(), Value::String(api_token.to_owned()));
        payload.insert("base_url".into(), Value::String(base_url.into()));
        payload.insert("label".into(), Value::String(resolver_source.to_owned()));
        result.changed = upsert_entry(entries, provider, &source, &payload);
        return result;
    }

    if provider == "qwen-oauth" {
        let Some(state) = state else {
            return result;
        };
        let token = state
            .get("api_key")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if token.is_empty() {
            return result;
        }
        let source = state
            .get("source")
            .and_then(Value::as_str)
            .filter(|source| !source.is_empty())
            .unwrap_or("qwen-cli");
        if suppressed_sources.contains(source) {
            return result;
        }
        result.active_sources.insert(source.into());
        let mut payload = Map::new();
        payload.insert("source".into(), Value::String(source.into()));
        payload.insert("auth_type".into(), Value::String(AUTH_TYPE_OAUTH.into()));
        payload.insert("access_token".into(), Value::String(token.into()));
        payload.insert(
            "label".into(),
            Value::String(
                state
                    .get("auth_file")
                    .and_then(Value::as_str)
                    .filter(|label| !label.is_empty())
                    .unwrap_or(source)
                    .into(),
            ),
        );
        for key in ["expires_at_ms", "base_url"] {
            if let Some(value) = state.get(key).filter(|value| !value.is_null()) {
                payload.insert(key.into(), value.clone());
            }
        }
        result.changed = upsert_entry(entries, provider, source, &payload);
        return result;
    }

    if provider == "minimax-oauth" {
        let Some(state) = state else {
            return result;
        };
        let Some(access_token) = state
            .get("access_token")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
        else {
            return result;
        };
        let source = "oauth";
        if suppressed_sources.contains(source) {
            return result;
        }

        result.active_sources.insert(source.into());
        let fallback_label = label_from_token(access_token, source);
        let label = state
            .get("label")
            .and_then(Value::as_str)
            .filter(|label| !label.is_empty())
            .unwrap_or(&fallback_label);
        let base_url = state
            .get("inference_base_url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim_end_matches('/');
        let mut payload = Map::new();
        payload.insert("source".into(), Value::String(source.into()));
        payload.insert("auth_type".into(), Value::String(AUTH_TYPE_OAUTH.into()));
        payload.insert(
            "access_token".into(),
            Value::String(access_token.to_owned()),
        );
        if let Some(value) = state.get("refresh_token").filter(|value| !value.is_null()) {
            payload.insert("refresh_token".into(), value.clone());
        }
        if let Some(expires_at) = state.get("expires_at").and_then(Value::as_str) {
            if let Some(expires_at_ms) = parse_iso_timestamp_millis(expires_at) {
                payload.insert("expires_at_ms".into(), Value::from(expires_at_ms));
            }
        }
        payload.insert("base_url".into(), Value::String(base_url.into()));
        payload.insert("label".into(), Value::String(label.into()));
        result.changed = upsert_entry(entries, provider, source, &payload);
        return result;
    }

    if provider == "openai-codex" {
        let source = "device_code";
        if suppressed_sources.contains(source) {
            return result;
        }
        let Some(state) = state else {
            return result;
        };
        let Some(tokens) = state.get("tokens").and_then(Value::as_object) else {
            return result;
        };
        let Some(access_token) = tokens
            .get("access_token")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
        else {
            return result;
        };

        result.active_sources.insert(source.into());
        let custom_label = state
            .get("label")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|label| !label.is_empty());
        let fallback_label = label_from_token(access_token, source);
        let mut payload = Map::new();
        payload.insert("source".into(), Value::String(source.into()));
        payload.insert("auth_type".into(), Value::String(AUTH_TYPE_OAUTH.into()));
        payload.insert(
            "access_token".into(),
            Value::String(access_token.to_owned()),
        );
        if let Some(value) = tokens.get("refresh_token").filter(|value| !value.is_null()) {
            payload.insert("refresh_token".into(), value.clone());
        }
        payload.insert(
            "base_url".into(),
            Value::String("https://chatgpt.com/backend-api/codex".into()),
        );
        if let Some(value) = state.get("last_refresh").filter(|value| !value.is_null()) {
            payload.insert("last_refresh".into(), value.clone());
        }
        payload.insert(
            "label".into(),
            Value::String(custom_label.unwrap_or(&fallback_label).into()),
        );
        result.changed = upsert_entry(entries, provider, source, &payload);
        return result;
    }

    if provider == "xai-oauth" {
        let source = "device_code";
        let Some(state) = state else {
            return result;
        };
        let Some(tokens) = state.get("tokens").and_then(Value::as_object) else {
            return result;
        };
        let Some(access_token) = tokens
            .get("access_token")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
        else {
            return result;
        };
        if suppressed_sources.contains(source) {
            return result;
        }

        result.active_sources.insert(source.into());
        let mut payload = Map::new();
        payload.insert("source".into(), Value::String(source.into()));
        payload.insert("auth_type".into(), Value::String(AUTH_TYPE_OAUTH.into()));
        payload.insert(
            "access_token".into(),
            Value::String(access_token.to_owned()),
        );
        if let Some(value) = tokens.get("refresh_token").filter(|value| !value.is_null()) {
            payload.insert("refresh_token".into(), value.clone());
        }
        payload.insert(
            "base_url".into(),
            Value::String("https://api.x.ai/v1".into()),
        );
        if let Some(value) = state.get("last_refresh").filter(|value| !value.is_null()) {
            payload.insert("last_refresh".into(), value.clone());
        }
        payload.insert(
            "label".into(),
            Value::String(label_from_token(access_token, source)),
        );
        result.changed = upsert_entry(entries, provider, source, &payload);
        return result;
    }

    if provider != "nous" {
        return result;
    }
    let Some(state) = state else {
        return result;
    };
    let has_runtime_material = ["access_token", "agent_key"].iter().any(|key| {
        state
            .get(*key)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    });
    if !has_runtime_material {
        let original_len = entries.len();
        entries
            .retain(|entry| entry.source != "device_code" && entry.source != "manual:device_code");
        result.changed = entries.len() != original_len;
        return result;
    }
    if suppressed_sources.contains("device_code") {
        return result;
    }

    result.active_sources.insert("device_code".into());
    let access_token = state
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let custom_label = state
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let seeded_label = if custom_label.is_empty() {
        label_from_token(access_token, "device_code")
    } else {
        custom_label.to_owned()
    };
    let mut payload = Map::new();
    payload.insert("source".into(), Value::String("device_code".into()));
    payload.insert("auth_type".into(), Value::String(AUTH_TYPE_OAUTH.into()));
    payload.insert("label".into(), Value::String(seeded_label));
    for key in [
        "access_token",
        "refresh_token",
        "expires_at",
        "token_type",
        "scope",
        "client_id",
        "portal_base_url",
        "inference_base_url",
        "agent_key",
        "agent_key_expires_at",
        "obtained_at",
        "expires_in",
        "agent_key_id",
        "agent_key_expires_in",
        "agent_key_reused",
        "agent_key_obtained_at",
        "tls",
    ] {
        if let Some(value) = state.get(key).filter(|value| !value.is_null()) {
            payload.insert(key.into(), value.clone());
        }
    }
    result.changed = upsert_entry(entries, provider, "device_code", &payload);
    result
}

/// Parse the Hermes `.env` file into assignment values.
///
/// PARITY: `hermes_cli.config.load_env` and `_parse_env_value` (3668–3724).
/// Reads are intentionally lossy UTF-8, preserve everything after the first
/// `=`, accept `export KEY=...`, and fail only for filesystem read errors.
pub fn load_env_file(path: &Path) -> io::Result<BTreeMap<String, String>> {
    let raw = match fs::read(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(error),
    };
    let text = String::from_utf8_lossy(&raw);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut values = BTreeMap::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let assignment = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, raw_value)) = assignment.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        values.insert(key.to_owned(), parse_env_value(raw_value));
    }
    Ok(values)
}

fn parse_env_value(raw_value: &str) -> String {
    let value = raw_value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        let quoted = &value[1..value.len() - 1];
        let mut parsed = String::with_capacity(quoted.len());
        let mut chars = quoted.chars();
        while let Some(character) = chars.next() {
            if character == '\\' {
                if let Some(next) = chars.next() {
                    if matches!(next, '"' | '\\') {
                        parsed.push(next);
                    } else {
                        parsed.push(character);
                        parsed.push(next);
                    }
                    continue;
                }
            }
            parsed.push(character);
        }
        return parsed;
    }
    if value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'') {
        return value[1..value.len() - 1].to_owned();
    }
    value.to_owned()
}

/// Resolve one credential value with the source's `.env` precedence.
///
/// PARITY: `agent/credential_pool.py.get_env_prefer_dotenv` (2841–2869).
pub fn get_env_prefer_dotenv(key: &str, snapshot: &EnvironmentSnapshot) -> String {
    snapshot.preferred_value(key)
}

/// Seed API-key credentials from `.env`/process environment.
///
/// PARITY: `agent/credential_pool.py._seed_from_env` (2849–2974). The
/// provider registry, secret scope, and source-suppression lookup are passed
/// in rather than imported upward from the CLI crate.
pub fn seed_from_env(
    provider: &str,
    entries: &mut Vec<PooledCredential>,
    config: Option<&ProviderCredentialConfig>,
    snapshot: &EnvironmentSnapshot,
) -> SeedResult {
    let mut result = SeedResult::default();

    // Copilot has a dedicated exchange path in the upstream singleton seeder;
    // generic env seeding must never overwrite that exchanged token.
    if provider == "copilot" {
        return result;
    }

    let mut upsert = |env_var: &str, token: String, base_url: String| {
        if token.is_empty() {
            return;
        }
        let source = format!("env:{env_var}");
        if snapshot.suppressed_sources.contains(&source) {
            return;
        }
        result.active_sources.insert(source.clone());
        let mut payload = Map::new();
        payload.insert("source".into(), Value::String(source.clone()));
        payload.insert("auth_type".into(), Value::String(AUTH_TYPE_API_KEY.into()));
        payload.insert("access_token".into(), Value::String(token));
        payload.insert("base_url".into(), Value::String(base_url));
        payload.insert("label".into(), Value::String(env_var.into()));
        if let Some(secret_source) = snapshot.secret_sources.get(env_var) {
            if !secret_source.trim().is_empty() {
                payload.insert(
                    "secret_source".into(),
                    Value::String(secret_source.trim().to_owned()),
                );
            }
        }
        result.changed |= upsert_entry(entries, provider, &source, &payload);
    };

    if provider == "openrouter" {
        upsert(
            "OPENROUTER_API_KEY",
            get_env_prefer_dotenv("OPENROUTER_API_KEY", snapshot),
            OPENROUTER_BASE_URL.into(),
        );
        return result;
    }

    let Some(config) = config.filter(|config| config.auth_type == AUTH_TYPE_API_KEY) else {
        return result;
    };
    let base_url_override = config
        .base_url_env_var
        .as_deref()
        .map(|key| {
            get_env_prefer_dotenv(key, snapshot)
                .trim_end_matches('/')
                .to_owned()
        })
        .unwrap_or_default();

    let env_vars: Vec<&str> = if provider == "anthropic" {
        vec![
            "ANTHROPIC_TOKEN",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "ANTHROPIC_API_KEY",
        ]
    } else {
        config.api_key_env_vars.iter().map(String::as_str).collect()
    };
    for env_var in env_vars {
        let token = get_env_prefer_dotenv(env_var, snapshot);
        if token.is_empty() {
            continue;
        }
        let mut base_url = if base_url_override.is_empty() {
            config.inference_base_url.clone()
        } else {
            base_url_override.clone()
        };
        if provider == "kimi-coding"
            && base_url_override.is_empty()
            && token.starts_with("sk-kimi-")
        {
            // PARITY: `_resolve_kimi_base_url` (585–598). Z.AI's equivalent
            // endpoint probe is an auth-layer/network seam and is intentionally
            // supplied by the future provider integration.
            base_url = "https://api.kimi.com/coding".into();
        }
        upsert(env_var, token, base_url);
    }
    result
}

/// Remove file-backed seeded rows whose source was not active this pass.
///
/// PARITY: `agent/credential_pool.py._prune_stale_seeded_entries` (2976–3008).
/// Environment rows are retained during ordinary loads so one process cannot
/// erase another process's persisted env reference merely because its own
/// environment is different.
pub fn prune_stale_seeded_entries(
    entries: &mut Vec<PooledCredential>,
    active_sources: &BTreeSet<String>,
    prune_env_sources: bool,
) -> bool {
    let original_len = entries.len();
    entries.retain(|entry| {
        if is_manual_source(&entry.source) || active_sources.contains(&entry.source) {
            return true;
        }
        let prunable = if entry.source.starts_with("env:") {
            prune_env_sources
        } else {
            is_borrowed_credential_source(&entry.source, &entry.provider)
                || entry.source == "hermes_pkce"
        };
        !prunable
    });
    entries.len() != original_len
}

/// Load, environment-seed, prune, normalize, and persist one provider pool.
///
/// This compatibility wrapper keeps the earlier environment-only boundary for
/// callers that have not yet supplied provider-owned singleton or custom
/// configuration inputs.
pub fn load_pool_with_environment_at(
    provider: &str,
    provider_config: Option<&ProviderCredentialConfig>,
    pool_config: Option<&Map<String, Value>>,
    snapshot: &EnvironmentSnapshot,
    profile_path: &Path,
    global_path: Option<&Path>,
) -> io::Result<CredentialPool> {
    load_pool_with_inputs_at(
        provider,
        PoolLoadInputs {
            provider_config,
            pool_config,
            snapshot,
            singleton_state: None,
            custom_providers: &[],
            model_config: None,
            profile_path,
            global_path,
        },
    )
}

/// Load one provider pool from explicit auth/config inputs.
///
/// PARITY: `agent/credential_pool.py.load_pool` (3084–3165). The upstream
/// function chooses the `custom:*` branch before any singleton/environment
/// seeding. This bottom-up boundary receives the already-resolved singleton
/// state, custom-provider list, model map, and suppression set through the
/// environment snapshot so the higher auth/config layer can supply them
/// without an upward crate dependency.
pub fn load_pool_with_inputs_at(
    provider: &str,
    inputs: PoolLoadInputs<'_>,
) -> io::Result<CredentialPool> {
    let provider = provider.trim().to_ascii_lowercase();
    let raw = crate::credential_store::read_credential_pool_at(
        Some(inputs.profile_path),
        inputs.global_path,
        Some(&provider),
    )?;
    let raw_entries = raw.as_array().cloned().unwrap_or_default();
    let disk_ids: BTreeSet<String> = raw_entries
        .iter()
        .filter_map(|entry| entry.get("id").and_then(Value::as_str))
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    let raw_needs_sanitization = raw_entries.iter().any(|payload| {
        payload.as_object().is_some_and(|payload| {
            sanitize_borrowed_credential_payload(payload.clone(), &provider) != *payload
        })
    });
    let mut entries: Vec<PooledCredential> = raw_entries
        .iter()
        .map(|payload| PooledCredential::from_json(&provider, payload))
        .collect();
    let mut raw_needs_auth_normalization = raw_entries.iter().any(|payload| {
        let Some(payload) = payload.as_object() else {
            return false;
        };
        let access_token = payload
            .get("access_token")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let auth_type = payload
            .get("auth_type")
            .and_then(Value::as_str)
            .unwrap_or(AUTH_TYPE_API_KEY);
        normalize_pool_auth_type(&provider, access_token, auth_type) != auth_type
    });
    if raw_needs_auth_normalization {
        // A global fallback is read-only. Only heal auth-type normalization
        // when the active profile actually owns a non-empty provider pool.
        let active_store = crate::credential_store::load_auth_store(Some(inputs.profile_path))?;
        let active_entries = active_store
            .get("credential_pool")
            .and_then(Value::as_object)
            .and_then(|pool| pool.get(&provider))
            .and_then(Value::as_array);
        raw_needs_auth_normalization = active_entries.is_some_and(|rows| !rows.is_empty());
    }

    let mut changed = raw_needs_sanitization || raw_needs_auth_normalization;
    if provider.starts_with("custom:") {
        // PARITY: `load_pool` custom branch (3115–3119). Custom pools do not
        // run singleton or environment seeders; only their config/model
        // sources are active, and stale borrowed/env references are pruned.
        let seed_result = seed_custom_pool(
            &provider,
            &mut entries,
            inputs.custom_providers,
            inputs.model_config,
            &inputs.snapshot.suppressed_sources,
        );
        changed |= seed_result.changed;
        changed |= prune_stale_seeded_entries(&mut entries, &seed_result.active_sources, true);
    } else {
        // PARITY: `load_pool` non-custom branch (3120–3138). Singleton state
        // is applied before environment state, then env rows are retained on
        // ordinary reads so one process cannot erase another process's source.
        let singleton_result = seed_from_singletons(
            &provider,
            &mut entries,
            inputs.singleton_state,
            &inputs.snapshot.suppressed_sources,
        );
        let env_result = seed_from_env(
            &provider,
            &mut entries,
            inputs.provider_config,
            inputs.snapshot,
        );
        changed |= singleton_result.changed || env_result.changed;
        let active_sources = singleton_result
            .active_sources
            .union(&env_result.active_sources)
            .cloned()
            .collect::<BTreeSet<_>>();
        // Ordinary pool loads are non-destructive for env rows: another
        // process may own the environment value even when this process does
        // not. File-backed singleton rows still prune when absent.
        changed |= prune_stale_seeded_entries(&mut entries, &active_sources, false);
        changed |= normalize_pool_priorities(&provider, &mut entries);
    }

    if changed {
        let new_ids: BTreeSet<String> = entries.iter().map(|entry| entry.id.clone()).collect();
        let removed_ids: Vec<String> = disk_ids.difference(&new_ids).cloned().collect();
        let mut persisted = entries.clone();
        persisted.sort_by_key(|entry| entry.priority);
        let payloads: Vec<Value> = persisted.iter().map(PooledCredential::to_json).collect();
        crate::credential_store::write_credential_pool_at(
            inputs.profile_path,
            &provider,
            &payloads,
            &removed_ids,
        )?;
    }
    Ok(CredentialPool::new(
        &provider,
        entries,
        pool_strategy_from_config(&provider, inputs.pool_config),
    ))
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

fn parse_iso_timestamp_millis(value: &str) -> Option<i64> {
    let raw = value.trim();
    if raw.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc3339(raw)
        .map(|timestamp| timestamp.timestamp_millis())
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .and_then(|timestamp| {
                    Local
                        .from_local_datetime(&timestamp)
                        .single()
                        .map(|timestamp| timestamp.timestamp_millis())
                })
        })
        .or_else(|| {
            NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S%.f")
                .ok()
                .and_then(|timestamp| {
                    Local
                        .from_local_datetime(&timestamp)
                        .single()
                        .map(|timestamp| timestamp.timestamp_millis())
                })
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

pub(crate) fn sanitize_borrowed_credential_payload(
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

/// Normalize a named custom provider for use in a persisted pool key.
///
/// PARITY: agent/credential_pool.py `_normalize_custom_pool_name`.
pub fn normalize_custom_pool_name(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace(' ', "-")
}

/// Look up a `custom:<name>` pool key from explicit custom-provider config.
///
/// The config loader is intentionally an outer-layer dependency. This adapter
/// takes its already-parsed list so the endpoint/name precedence remains
/// testable without coupling `hermes-agent` to the future config crate.
///
/// PARITY: agent/credential_pool.py `get_custom_provider_pool_key`.
pub fn custom_provider_pool_key(
    base_url: Option<&str>,
    provider_name: Option<&str>,
    providers: &[Value],
) -> Option<String> {
    let base_url = base_url?.trim().trim_end_matches('/');
    if base_url.is_empty() {
        return None;
    }

    let valid = providers.iter().filter_map(Value::as_object);
    if let Some(provider_name) = provider_name.filter(|name| !name.is_empty()) {
        let normalized_name = normalize_custom_pool_name(provider_name);
        for entry in providers.iter().filter_map(Value::as_object) {
            let Some(name) = entry.get("name").and_then(Value::as_str) else {
                continue;
            };
            if normalize_custom_pool_name(name) == normalized_name {
                return Some(format!("custom:{normalized_name}"));
            }
        }
    }

    for entry in valid {
        let Some(name) = entry.get("name").and_then(Value::as_str) else {
            continue;
        };
        let entry_url = entry
            .get("base_url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .trim_end_matches('/');
        if !entry_url.is_empty() && entry_url == base_url {
            return Some(format!("custom:{}", normalize_custom_pool_name(name)));
        }
    }
    None
}

/// Return the parsed custom-provider config entry for a pool key.
///
/// PARITY: agent/credential_pool.py `_get_custom_provider_config`.
pub fn custom_provider_config(pool_key: &str, providers: &[Value]) -> Option<Map<String, Value>> {
    let suffix = pool_key.strip_prefix("custom:")?;
    providers
        .iter()
        .filter_map(Value::as_object)
        .find(|entry| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| normalize_custom_pool_name(name) == suffix)
        })
        .cloned()
}

/// Seed a custom endpoint pool from explicit provider and model config.
///
/// PARITY: `agent/credential_pool.py._seed_custom_pool` (3011–3075). Config
/// loading and suppression ownership remain outside this crate; the caller
/// supplies the parsed custom-provider list, optional model map, and active
/// suppression set.
pub fn seed_custom_pool(
    pool_key: &str,
    entries: &mut Vec<PooledCredential>,
    providers: &[Value],
    model_config: Option<&Map<String, Value>>,
    suppressed_sources: &BTreeSet<String>,
) -> SeedResult {
    let mut result = SeedResult::default();

    if let Some(config) = custom_provider_config(pool_key, providers) {
        let api_key = config
            .get("api_key")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        let base_url = config
            .get("base_url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .trim_end_matches('/');
        let name = config
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        if !api_key.is_empty() {
            let source = format!("config:{name}");
            if !suppressed_sources.contains(&source) {
                result.active_sources.insert(source.clone());
                let mut payload = Map::new();
                payload.insert("source".into(), Value::String(source.clone()));
                payload.insert("auth_type".into(), Value::String(AUTH_TYPE_API_KEY.into()));
                payload.insert("access_token".into(), Value::String(api_key.into()));
                payload.insert("base_url".into(), Value::String(base_url.into()));
                payload.insert(
                    "label".into(),
                    Value::String(if name.is_empty() {
                        source.clone()
                    } else {
                        name.into()
                    }),
                );
                result.changed |= upsert_entry(entries, pool_key, &source, &payload);
            }
        }
    }

    if let Some(model) = model_config {
        let model_provider = model
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let model_base_url = model
            .get("base_url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .trim_end_matches('/');
        let model_api_key = ["api_key", "api"].iter().find_map(|key| {
            model
                .get(*key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        });
        if model_provider == "custom" && !model_base_url.is_empty() {
            if let Some(model_api_key) = model_api_key {
                if custom_provider_pool_key(Some(model_base_url), None, providers).as_deref()
                    == Some(pool_key)
                {
                    let source = "model_config";
                    if !suppressed_sources.contains(source) {
                        result.active_sources.insert(source.into());
                        let mut payload = Map::new();
                        payload.insert("source".into(), Value::String(source.into()));
                        payload.insert("auth_type".into(), Value::String(AUTH_TYPE_API_KEY.into()));
                        payload.insert(
                            "access_token".into(),
                            Value::String(model_api_key.to_owned()),
                        );
                        payload.insert("base_url".into(), Value::String(model_base_url.to_owned()));
                        payload.insert("label".into(), Value::String(source.into()));
                        result.changed |= upsert_entry(entries, pool_key, source, &payload);
                    }
                }
            }
        }
    }

    result
}

/// List non-empty custom pool keys from a parsed credential-pool mapping.
///
/// PARITY: agent/credential_pool.py `list_custom_pool_providers`.
pub fn list_custom_pool_providers(pool_data: &Map<String, Value>) -> Vec<String> {
    let mut keys = pool_data
        .iter()
        .filter_map(|(key, value)| {
            (key.starts_with("custom:") && value.as_array().is_some_and(|rows| !rows.is_empty()))
                .then_some(key.clone())
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

/// Parse the configured selection strategy, failing open to fill-first.
///
/// PARITY: agent/credential_pool.py `get_pool_strategy`.
pub fn pool_strategy_from_config(
    provider: &str,
    config: Option<&Map<String, Value>>,
) -> PoolStrategy {
    let Some(strategy) = config
        .and_then(|config| config.get("credential_pool_strategies"))
        .and_then(Value::as_object)
        .and_then(|strategies| strategies.get(provider))
        .and_then(Value::as_str)
        .map(str::trim)
        .map(str::to_ascii_lowercase)
    else {
        return PoolStrategy::FillFirst;
    };
    match strategy.as_str() {
        "round_robin" => PoolStrategy::RoundRobin,
        "least_used" => PoolStrategy::LeastUsed,
        "random" => PoolStrategy::Random,
        "fill_first" => PoolStrategy::FillFirst,
        _ => PoolStrategy::FillFirst,
    }
}

/// Check whether a loaded pool belongs to the requested runtime provider.
///
/// `None` for `pool_provider` represents an old unscoped adapter and preserves
/// the source's backward-compatible fail-open result. A present empty identity
/// fails closed. Named custom pools are accepted only when their configured
/// endpoint resolves to the same `custom:<name>` key.
///
/// PARITY: agent/credential_pool.py `credential_pool_matches_provider`.
pub fn credential_pool_matches_provider(
    pool_provider: Option<&str>,
    provider: Option<&str>,
    base_url: Option<&str>,
    providers: &[Value],
) -> bool {
    let Some(raw_pool_provider) = pool_provider else {
        return true;
    };
    let pool_provider = raw_pool_provider.trim().to_ascii_lowercase();
    let provider = provider.unwrap_or_default().trim().to_ascii_lowercase();
    if pool_provider.is_empty() || provider.is_empty() {
        return false;
    }
    if pool_provider == provider {
        return true;
    }
    if provider != "custom" || !pool_provider.starts_with("custom:") {
        return false;
    }
    custom_provider_pool_key(base_url, None, providers)
        .is_some_and(|key| key.eq_ignore_ascii_case(&pool_provider))
}

fn is_manual_source(source: &str) -> bool {
    let normalized = source.trim().to_ascii_lowercase();
    normalized == "manual" || normalized.starts_with("manual:")
}

fn next_priority(entries: &[PooledCredential]) -> i32 {
    entries
        .iter()
        .map(|entry| entry.priority)
        .max()
        .unwrap_or(-1)
        .saturating_add(1)
}

/// Upsert one source-owned row while preserving the first row's identity.
///
/// Rows are singleton-scoped by `source`. Duplicate rows after the first are
/// removed. A changed access token clears stale failure state; an unchanged
/// token preserves its cooldown exactly as the Python implementation does.
///
/// PARITY: agent/credential_pool.py `_upsert_entry`.
pub fn upsert_entry(
    entries: &mut Vec<PooledCredential>,
    provider: &str,
    source: &str,
    payload: &Map<String, Value>,
) -> bool {
    let matching_indices = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| (entry.source == source).then_some(index))
        .collect::<Vec<_>>();
    let existing_index = matching_indices.first().copied();
    let had_duplicates = matching_indices.len() > 1;
    if had_duplicates {
        let duplicate_indices = matching_indices[1..]
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        *entries = entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                (!duplicate_indices.contains(&index)).then_some(entry.clone())
            })
            .collect();
    }

    let Some(existing_index) = existing_index else {
        let mut inserted = payload.clone();
        inserted
            .entry("id")
            .or_insert_with(|| Value::String(generated_credential_id()));
        inserted
            .entry("priority")
            .or_insert_with(|| Value::from(next_priority(entries)));
        inserted
            .entry("label")
            .or_insert_with(|| Value::String(source.to_owned()));
        entries.push(PooledCredential::from_dict(provider, &inserted));
        return true;
    };

    let existing = entries[existing_index].clone();
    let token_changed = payload
        .get("access_token")
        .filter(|value| !value.is_null())
        .and_then(Value::as_str)
        .is_some_and(|token| token != existing.access_token);
    let mut updated = existing.clone();
    let mut field_changed = false;
    let mut extra_changed = false;

    for (key, value) in payload {
        if matches!(key.as_str(), "id" | "priority") || value.is_null() {
            continue;
        }
        if key == "label" && !existing.label.is_empty() {
            continue;
        }
        if EXTRA_KEYS.contains(&key.as_str()) {
            if updated.extra.get(key) != Some(value) {
                updated.extra.insert(key.clone(), value.clone());
                extra_changed = true;
            }
            if key == "failure_reason" {
                let next = value.as_str().map(ToOwned::to_owned);
                if updated.failure_reason != next {
                    updated.failure_reason = next;
                    field_changed = true;
                }
            }
            continue;
        }
        let changed = match key.as_str() {
            "label" => value.as_str().map(|next| {
                if updated.label != next {
                    updated.label = next.to_owned();
                    true
                } else {
                    false
                }
            }),
            "auth_type" => value.as_str().map(|next| {
                let normalized = normalize_pool_auth_type(provider, &updated.access_token, next);
                if updated.auth_type != normalized {
                    updated.auth_type = normalized;
                    true
                } else {
                    false
                }
            }),
            "source" => value.as_str().map(|next| {
                if updated.source != next {
                    updated.source = next.to_owned();
                    true
                } else {
                    false
                }
            }),
            "access_token" => value.as_str().map(|next| {
                if updated.access_token != next {
                    updated.access_token = next.to_owned();
                    true
                } else {
                    false
                }
            }),
            "refresh_token" => value.as_str().map(|next| {
                let next = Some(next.to_owned());
                if updated.refresh_token != next {
                    updated.refresh_token = next;
                    true
                } else {
                    false
                }
            }),
            "last_status" => value.as_str().map(|next| {
                let next_status = Some(next.to_owned());
                if updated.last_status != next_status {
                    updated.last_status = next_status;
                    updated.status = CredentialStatus::from_source(next);
                    true
                } else {
                    false
                }
            }),
            "last_status_at" => source_timestamp(Some(value)).map(|next| {
                if updated.last_status_at != Some(next) {
                    updated.last_status_at = Some(next);
                    true
                } else {
                    false
                }
            }),
            "last_error_code" => value
                .as_u64()
                .and_then(|next| u16::try_from(next).ok())
                .map(|next| {
                    if updated.last_error_code != Some(next) {
                        updated.last_error_code = Some(next);
                        true
                    } else {
                        false
                    }
                }),
            "last_error_reason" => value.as_str().map(|next| {
                let next = Some(next.to_owned());
                if updated.last_error_reason != next {
                    updated.last_error_reason = next;
                    true
                } else {
                    false
                }
            }),
            "last_error_message" => value.as_str().map(|next| {
                let next = Some(next.to_owned());
                if updated.last_error_message != next {
                    updated.last_error_message = next;
                    true
                } else {
                    false
                }
            }),
            "last_error_reset_at" => value.as_f64().map(|next| {
                if updated.last_error_reset_at != Some(next) {
                    updated.last_error_reset_at = Some(next);
                    true
                } else {
                    false
                }
            }),
            "base_url" => value.as_str().map(|next| {
                let next = Some(next.to_owned());
                if updated.base_url != next {
                    updated.base_url = next;
                    true
                } else {
                    false
                }
            }),
            "expires_at" => value.as_str().map(|next| {
                let next = Some(next.to_owned());
                if updated.expires_at != next {
                    updated.expires_at = next;
                    true
                } else {
                    false
                }
            }),
            "expires_at_ms" => value.as_i64().map(|next| {
                if updated.expires_at_ms != Some(next) {
                    updated.expires_at_ms = Some(next);
                    true
                } else {
                    false
                }
            }),
            "last_refresh" => value.as_str().map(|next| {
                let next = Some(next.to_owned());
                if updated.last_refresh != next {
                    updated.last_refresh = next;
                    true
                } else {
                    false
                }
            }),
            "inference_base_url" => value.as_str().map(|next| {
                let next = Some(next.to_owned());
                if updated.inference_base_url != next {
                    updated.inference_base_url = next;
                    true
                } else {
                    false
                }
            }),
            "agent_key" => value.as_str().map(|next| {
                let next = Some(next.to_owned());
                if updated.agent_key != next {
                    updated.agent_key = next;
                    true
                } else {
                    false
                }
            }),
            "agent_key_expires_at" => value.as_str().map(|next| {
                let next = Some(next.to_owned());
                if updated.agent_key_expires_at != next {
                    updated.agent_key_expires_at = next;
                    true
                } else {
                    false
                }
            }),
            "request_count" => value.as_u64().map(|next| {
                if updated.request_count != next {
                    updated.request_count = next;
                    true
                } else {
                    false
                }
            }),
            _ => None,
        };
        field_changed |= changed.unwrap_or(false);
    }

    let normalized_auth_type =
        normalize_pool_auth_type(provider, &updated.access_token, &updated.auth_type);
    if updated.auth_type != normalized_auth_type {
        updated.auth_type = normalized_auth_type;
        field_changed = true;
    }

    if token_changed && existing.last_status.is_some() {
        updated.last_status = None;
        updated.status = None;
        updated.last_status_at = None;
        updated.last_error_code = None;
        updated.last_error_reason = None;
        updated.last_error_message = None;
        updated.last_error_reset_at = None;
        field_changed = true;
    }

    if field_changed || extra_changed {
        entries[existing_index] = updated.clone();
        return had_duplicates || existing.to_dict() != updated.to_dict();
    }
    had_duplicates
}

/// Reassign Anthropic priorities while keeping manual entries ahead of seeded
/// credentials and preserving the source's stable source-rank tie-breakers.
///
/// PARITY: agent/credential_pool.py `_normalize_pool_priorities`.
pub fn normalize_pool_priorities(provider: &str, entries: &mut [PooledCredential]) -> bool {
    if provider != "anthropic" {
        return false;
    }
    let source_rank = [
        ("env:ANTHROPIC_TOKEN", 0usize),
        ("env:CLAUDE_CODE_OAUTH_TOKEN", 1),
        ("hermes_pkce", 2),
        ("claude_code", 3),
        ("env:ANTHROPIC_API_KEY", 4),
    ];
    let rank_for = |source: &str| {
        source_rank
            .iter()
            .find_map(|(candidate, rank)| (*candidate == source).then_some(*rank))
            .unwrap_or(source_rank.len())
    };
    let mut ordered = entries.iter().cloned().enumerate().collect::<Vec<_>>();
    ordered.sort_by(|(left_index, left), (right_index, right)| {
        let left_is_manual = is_manual_source(&left.source);
        let right_is_manual = is_manual_source(&right.source);
        let ordering = left_is_manual
            .cmp(&right_is_manual)
            .reverse()
            .then_with(|| {
                let left_rank = if left_is_manual {
                    0
                } else {
                    rank_for(&left.source)
                };
                let right_rank = if right_is_manual {
                    0
                } else {
                    rank_for(&right.source)
                };
                left_rank.cmp(&right_rank)
            })
            .then_with(|| left.priority.cmp(&right.priority))
            .then_with(|| {
                if left_is_manual {
                    std::cmp::Ordering::Equal
                } else {
                    left.label.cmp(&right.label)
                }
            });
        ordering.then_with(|| left_index.cmp(right_index))
    });
    let positions = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.id.clone(), index))
        .collect::<std::collections::HashMap<_, _>>();
    let mut changed = false;
    for (new_priority, (_, entry)) in ordered.into_iter().enumerate() {
        if entry.priority == new_priority as i32 {
            continue;
        }
        if let Some(index) = positions.get(&entry.id) {
            entries[*index].priority = new_priority as i32;
            changed = true;
        }
    }
    changed
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
            PoolStrategy::Random => *available.choose(&mut rand::rng())?,
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

pub(crate) fn exhausted_until(entry: &PooledCredential, sole_credential: bool) -> Option<f64> {
    if entry.status != Some(CredentialStatus::Exhausted) {
        return None;
    }
    if let Some(reset_at) = entry.last_error_reset_at {
        return Some(normalize_epoch_seconds(reset_at));
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

fn normalize_epoch_seconds(value: f64) -> f64 {
    if value > 1_000_000_000_000.0 {
        value / 1000.0
    } else {
        value
    }
}
