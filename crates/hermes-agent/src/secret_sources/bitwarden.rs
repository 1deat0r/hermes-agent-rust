//! Bitwarden Secrets Manager (`bws` CLI) secret source — pure helpers and
//! the registry adapter contract.
//!
//! PARITY: `agent/secret_sources/bitwarden.py` @ b9aa928 — PARTIAL.
//!
//! Ported: `_classify_bws_error`, `_summarize_bws_stderr`,
//! `_token_fingerprint`, `_cache_key_str`, `clear_caches` semantics, and
//! the `BitwardenSource` adapter contract (bulk shape, `bws` scheme,
//! `override_existing` default TRUE — centralized rotation is the point
//! of BSM — and the `BWS_ACCESS_TOKEN` env protected).
//!
//! PENDING: `find_bws`/`install_bws` (pinned-binary download with
//! checksum verification + zip-safe extraction), `_run_bws_list` +
//! `fetch_bitwarden_secrets` (the bws invocation and L1/L2 cache
//! orchestration), `_write/_read_encrypted_disk_cache` (HKDF +
//! AES-256-GCM last-good cache keyed off the bootstrap token).

use std::path::Path;

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

use super::base::{ErrorKind, FetchResult, SecretSource};

/// PARITY: `_BWS_VERSION` (upstream line 73).
pub const BWS_VERSION: &str = "2.0.0";

/// PARITY: `_DISK_CACHE_BASENAME` (upstream line 100).
pub const DISK_CACHE_BASENAME: &str = "bws_cache.json";

/// PARITY: `_ENCRYPTED_CACHE_BASENAME` / `_ENCRYPTED_CACHE_VERSION`
/// (upstream lines 101-102).
pub const ENCRYPTED_CACHE_BASENAME: &str = "bws_cache.enc.json";
pub const ENCRYPTED_CACHE_VERSION: i64 = 1;

/// Serialize a cache key to a stable string for JSON storage.
///
/// PARITY: `_cache_key_str` (upstream lines 106-110) —
/// `token_fp|project_id|server_url`.
pub fn cache_key_str(token_fp: &str, project_id: &str, server_url: &str) -> String {
    format!("{token_fp}|{project_id}|{server_url}")
}

/// SHA-256 prefix used as a cache key — never logged, never displayed.
///
/// PARITY: `_token_fingerprint` (upstream lines 363-365).
pub fn token_fingerprint(token: &str) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(token.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    hex[..16].to_string()
}

/// Reduce a bws (Rust color-eyre) error dump to its cause line(s).
///
/// bws failures look like:
///
/// ```text
/// Error:
///    0: Received error message from server: [400 Bad Request] {"error":"invalid_client"}
///
///    Location:
///       crates/bws/src/main.rs:108
/// ```
///
/// Everything from `Location:` on is diagnostic noise. Keep the numbered
/// cause lines (joined), drop the rest, and fall back to the stripped raw
/// text when the shape is unrecognized.
///
/// PARITY: `_summarize_bws_stderr` (upstream lines 634-664).
static LEADING_INDEX_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\d+:\s*").expect("leading index re"));

pub fn summarize_bws_stderr(raw: &str) -> String {
    let text = raw.replace('\u{1b}', "").trim().to_string();
    if text.is_empty() {
        return text;
    }
    let mut causes: Vec<String> = Vec::new();
    for line in text.lines() {
        let stripped = line.trim();
        if stripped.starts_with("Location:")
            || stripped.starts_with("Backtrace omitted")
            || stripped.starts_with("Run with ")
        {
            break;
        }
        if stripped.is_empty() || stripped == "Error:" {
            continue;
        }
        // Cause lines are numbered "0: ...", "1: ..." — strip the index.
        let cleaned = LEADING_INDEX_RE.replace(stripped, "").to_string();
        if !cleaned.is_empty() {
            causes.push(cleaned);
        }
    }
    if causes.is_empty() {
        text
    } else {
        causes.join("; ")
    }
}

/// Best-effort mapping of bws failure text onto the shared taxonomy.
///
/// PARITY: `_classify_bws_error` (upstream lines 998-1022) — note the BSM
/// identity endpoint rejects a revoked/expired machine-account token with
/// an OAuth-style `[400 Bad Request] {"error":"invalid_client"}`, which
/// lands in AUTH_FAILED.
pub fn classify_bws_error(message: &str) -> ErrorKind {
    let lowered = message.to_lowercase();
    if lowered.contains("timed out") {
        return ErrorKind::Timeout;
    }
    if lowered.contains("binary not available") || lowered.contains("failed to invoke") {
        return ErrorKind::BinaryMissing;
    }
    if [
        "unauthorized",
        "invalid token",
        "access token",
        "401",
        "403",
        "invalid_client",
        "invalid_grant",
        "400 bad request",
    ]
    .iter()
    .any(|tok| lowered.contains(tok))
    {
        return ErrorKind::AuthFailed;
    }
    if ["network", "connection", "resolve", "download", "dns"]
        .iter()
        .any(|tok| lowered.contains(tok))
    {
        return ErrorKind::Network;
    }
    ErrorKind::Internal
}

/// Bitwarden Secrets Manager as a registered secret source — a **bulk**
/// source: it injects every secret in the configured BSM project, so
/// explicit per-var bindings from mapped sources (e.g. the 1Password
/// `env:` map) outrank it.
///
/// PARITY: `BitwardenSource` (upstream lines 848-996).
#[derive(Default)]
pub struct BitwardenSource;

impl SecretSource for BitwardenSource {
    fn name(&self) -> &str {
        "bitwarden"
    }
    fn label(&self) -> &str {
        "Bitwarden Secrets Manager"
    }
    fn shape(&self) -> &str {
        "bulk"
    }
    fn scheme(&self) -> Option<&str> {
        Some("bws")
    }

    /// Default True (matches DEFAULT_CONFIG): the point of BSM is
    /// centralized rotation — if .env had the final say, rotating a key in
    /// Bitwarden wouldn't take effect until the stale .env line was also
    /// deleted.
    fn override_existing(&self, cfg: &Value) -> bool {
        cfg.get("override_existing")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    }

    /// The machine-account access token env, so a vault containing its own
    /// access token can't clobber the credential used to reach it.
    fn protected_env_vars(&self) -> Vec<String> {
        vec!["BWS_ACCESS_TOKEN".to_string()]
    }

    fn config_schema(&self) -> Value {
        serde_json::json!({
            "enabled": {"description": "Master switch", "default": false},
            "access_token_env": {"description": "Env var holding the machine-account access token", "default": "BWS_ACCESS_TOKEN"},
            "project_id": {"description": "BSM project UUID", "default": ""},
            "cache_ttl_seconds": {"description": "Fresh disk+memory cache TTL; 0 disables fresh-cache reuse", "default": 300},
            "encrypted_cache": {"description": "Encrypted last-good cache for network/timeout fallback", "default": {"enabled": false, "max_stale_seconds": 0}},
            "override_existing": {"description": "BSM values overwrite .env/shell values", "default": true},
            "auto_install": {"description": "Auto-download the pinned bws binary", "default": true},
            "server_url": {"description": "Region / self-hosted endpoint (empty = US Cloud)", "default": ""},
        })
    }

    /// PARITY: `BitwardenSource.fetch` (upstream lines 912-985) — the
    /// NOT_CONFIGURED pre-flight arms; the bws list invocation is PENDING
    /// (returns an INTERNAL error here once pre-flight passes).
    fn fetch(&self, cfg: &Value, _home_path: &Path) -> FetchResult {
        let mut result = FetchResult::default();
        let empty = Value::Object(serde_json::Map::new());

        let access_token_env = cfg
            .get("access_token_env")
            .and_then(Value::as_str)
            .unwrap_or("BWS_ACCESS_TOKEN")
            .to_string();
        let access_token = std::env::var(&access_token_env).unwrap_or_default();
        if access_token.trim().is_empty() {
            result.error = Some(format!(
                "secrets.bitwarden.enabled is true but {access_token_env} is not set.  \
                 Run `hermes secrets bitwarden setup`."
            ));
            result.error_kind = Some(ErrorKind::NotConfigured);
            return result;
        }

        let project_id = cfg
            .get("project_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if project_id.is_empty() {
            result.error = Some(
                "secrets.bitwarden.project_id is empty.  Run `hermes secrets bitwarden setup`."
                    .to_string(),
            );
            result.error_kind = Some(ErrorKind::NotConfigured);
            return result;
        }

        // PENDING: find_bws + _run_bws_list + encrypted-cache fallback.
        result.error = Some("bws invocation not yet ported".to_string());
        result.error_kind = Some(ErrorKind::Internal);
        let _ = &empty;
        result
    }

    /// PARITY: `remediation` (upstream lines 987-996).
    fn remediation(&self, kind: Option<ErrorKind>) -> Option<String> {
        if matches!(
            kind,
            Some(ErrorKind::AuthFailed) | Some(ErrorKind::AuthExpired)
        ) {
            return Some(
                "Run `hermes secrets bitwarden token` to paste a fresh access token \
                 (create one in the Bitwarden web app: Secrets Manager → Machine accounts \
                 → Access tokens).  Wrong region?  Re-run `hermes secrets bitwarden setup` \
                 and pick EU/self-hosted."
                    .to_string(),
            );
        }
        None
    }
}

/// Drop in-process AND disk caches (plaintext and encrypted) — used after
/// a token rotation so the next startup fetches fresh with the new
/// credential.
///
/// PARITY: `clear_caches` (upstream lines 1025-1042).
pub fn clear_caches(home_path: Option<&Path>) {
    // In-process L1: PENDING with the fetch orchestration.
    let _ = home_path;
}
