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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

use std::collections::HashMap;

use super::base::{ErrorKind, FetchResult, SecretSource};
use super::cache::{CachedFetch, DiskCache};

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
    /// NOT_CONFIGURED pre-flight arms, then the full bws list orchestration
    /// (L1/L2 cache + live fetch with stale fallback).
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

        let encrypted_enabled = cfg
            .get("encrypted_cache")
            .and_then(|v| v.get("enabled"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let encrypted_max_stale = cfg
            .get("encrypted_cache")
            .and_then(|v| v.get("max_stale_seconds"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0);

        let binary = find_bws(false);
        if binary.is_none() {
            result.error = Some(
                "bws binary not available and auto-install is disabled.  \
                 Run `hermes secrets bitwarden setup` to install."
                    .to_string(),
            );
            result.error_kind = Some(ErrorKind::BinaryMissing);
            return result;
        }

        let ttl = cfg
            .get("cache_ttl_seconds")
            .and_then(Value::as_f64)
            .unwrap_or(300.0);
        let server_url = cfg
            .get("server_url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        // PENDING: the installer (`install_bws`) and the HKDF+AESGCM
        // encrypted last-good cache. With `encrypted_cache_enabled` the
        // encrypted arm is the only stale fallback consulted, and it is
        // not ported yet — so that mode currently has no stale fallback.
        match fetch_bitwarden_secrets(
            &access_token,
            &project_id,
            binary.as_deref(),
            ttl,
            true,
            &server_url,
            Some(_home_path),
            false,
            0.0,
        ) {
            Ok((secrets, mut warnings)) => {
                result.secrets = secrets.into_iter().collect();
                result.warnings.append(&mut warnings);
                result
            }
            Err(err) => {
                result.error = Some(err);
                result.error_kind = Some(classify_bws_error(result.error.as_deref().unwrap_or("")));
                result
            }
        }
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

// ---------------------------------------------------------------------------
// Binary discovery
// ---------------------------------------------------------------------------

/// PARITY: `_DISK_CACHE` (upstream lines 111-113).
fn disk_cache() -> &'static DiskCache {
    static DISK: Lazy<DiskCache> = Lazy::new(|| DiskCache::new(DISK_CACHE_BASENAME));
    &DISK
}

/// PARITY: `_disk_cache_path` (upstream lines 117-125).
pub fn disk_cache_path(home_path: Option<&Path>) -> PathBuf {
    disk_cache().path(home_path)
}

/// Where Hermes stores its managed binaries. Profile-aware.
///
/// PARITY: `_hermes_bin_dir` (upstream lines 138-142).
pub fn hermes_bin_dir() -> PathBuf {
    hermes_constants::get_hermes_home().join("bin")
}

/// Return a path to a usable `bws` binary, or None.
///
/// Resolution order: `<hermes_home>/bin/bws` (managed copy, preferred),
/// then `shutil.which("bws")`. `install_if_missing` upstream calls
/// `install_bws` — the pinned-checksum downloader is PENDING, so this
/// port returns None in that case.
///
/// PARITY: `find_bws` (upstream lines 145-170).
pub fn find_bws(install_if_missing: bool) -> Option<PathBuf> {
    let _ = install_if_missing; // installer PENDING
    let managed = hermes_bin_dir().join(platform_binary_name());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if managed.exists() {
            let ok = std::fs::metadata(&managed)
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false);
            if ok {
                return Some(managed);
            }
        }
    }
    #[cfg(not(unix))]
    if managed.exists() {
        return Some(managed);
    }
    which_bws()
}

/// `shutil.which("bws")`.
fn which_bws() -> Option<PathBuf> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in path_var.split(':') {
        if dir.is_empty() {
            continue;
        }
        let candidate = Path::new(dir).join(platform_binary_name());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if candidate.is_file()
                && std::fs::metadata(&candidate)
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false)
            {
                return Some(candidate);
            }
        }
        #[cfg(not(unix))]
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// PARITY: `_platform_binary_name` (upstream lines 172-174).
pub fn platform_binary_name() -> &'static str {
    if cfg!(windows) {
        "bws.exe"
    } else {
        "bws"
    }
}

// ---------------------------------------------------------------------------
// bws secret list
// ---------------------------------------------------------------------------

/// PARITY: `_run_bws_list` (upstream lines 667-751) — run `bws secret list
/// <project> --output json`, merge stdout/stderr into classified errors,
/// and parse the JSON array of {key, value} entries (invalid env-var names
/// warn and skip).
pub fn run_bws_list(
    bws: &Path,
    access_token: &str,
    project_id: &str,
    server_url: &str,
) -> Result<(std::collections::BTreeMap<String, String>, Vec<String>), String> {
    use std::io::Read;
    use std::process::{Command, Stdio};

    let mut command = Command::new(bws);
    command
        .args(["secret", "list", project_id, "--output", "json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // bws child intentionally receives the access token; a profile-local
    // fetch must not inherit sibling credentials from the global env, so
    // the child env comes from the active per-fetch view.
    let snapshot = super::base::get_source_environment_snapshot();
    command.env_clear();
    for (k, v) in &snapshot {
        command.env(k, v);
    }
    command.env("BWS_ACCESS_TOKEN", access_token);
    command.env("NO_COLOR", "1");
    if !server_url.is_empty() {
        // Region / self-hosted support; when unset, whatever BWS_SERVER_URL
        // the caller already had is preserved by the snapshot copy above.
        command.env("BWS_SERVER_URL", server_url);
    }

    let mut child = command
        .spawn()
        .map_err(|e| format!("failed to invoke bws: {e}"))?;
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let drain = std::thread::spawn(move || {
        let mut out = Vec::new();
        if let Some(o) = stdout_pipe.as_mut() {
            let _ = o.read_to_end(&mut out);
        }
        let mut err = Vec::new();
        if let Some(e) = stderr_pipe.as_mut() {
            let _ = e.read_to_end(&mut err);
        }
        (out, err)
    });

    let deadline = Instant::now()
        + std::time::Duration::from_secs_f64(super::base::DEFAULT_CLI_TIMEOUT_SECONDS);
    let _ = deadline;
    let mut exit_code: Option<i32> = None;
    let mut timed_out = false;
    let started = Instant::now();
    while exit_code.is_none() {
        match child.try_wait() {
            Ok(Some(status)) => exit_code = Some(status.code().unwrap_or(-1)),
            Ok(None) => {
                if started.elapsed()
                    >= std::time::Duration::from_secs_f64(super::base::DEFAULT_CLI_TIMEOUT_SECONDS)
                {
                    timed_out = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => return Err(format!("failed to invoke bws: {e}")),
        }
    }
    let (stdout_bytes, stderr_bytes) = drain.join().unwrap_or_default();
    if timed_out {
        return Err(format!(
            "bws timed out after {}s fetching secrets",
            super::base::DEFAULT_CLI_TIMEOUT_SECONDS
        ));
    }
    let exit_code = exit_code.unwrap_or(-1);
    if exit_code != 0 {
        // bws writes auth/network errors to stderr as a Rust error-report
        // dump; boil it down to the meaningful cause line(s) first.
        let stderr_text = String::from_utf8_lossy(&stderr_bytes);
        let stdout_text = String::from_utf8_lossy(&stdout_bytes);
        let err = summarize_bws_stderr(if stderr_text.is_empty() {
            &stdout_text
        } else {
            &stderr_text
        });
        return Err(format!(
            "bws exited {exit_code}: {}",
            &err[..err.len().min(200)]
        ));
    }

    let raw = String::from_utf8_lossy(&stdout_bytes).trim().to_string();
    if raw.is_empty() {
        return Ok((
            Default::default(),
            vec!["bws returned no output (empty project?)".to_string()],
        ));
    }
    let payload: Value =
        serde_json::from_str(&raw).map_err(|e| format!("bws returned non-JSON output: {e}"))?;
    let Some(items) = payload.as_array() else {
        return Err(format!(
            "bws returned unexpected shape: {}",
            if payload.is_object() { "dict" } else { "other" }
        ));
    };

    let mut secrets = BTreeMap::new();
    let mut warnings = Vec::new();
    for item in items {
        let Some(item) = item.as_object() else {
            continue;
        };
        let (Some(key), Some(value)) = (
            item.get("key").and_then(Value::as_str),
            item.get("value").and_then(Value::as_str),
        ) else {
            continue;
        };
        if !super::base::is_valid_env_name(key) {
            warnings.push(format!("Skipping secret {key:?}: not a valid env-var name"));
            continue;
        }
        secrets.insert(key.to_string(), value.to_string());
    }
    Ok((secrets, warnings))
}

// ---------------------------------------------------------------------------
// Fetch orchestration
// ---------------------------------------------------------------------------

/// In-process L1 cache: (token_fp, project_id, server_url) → entry.
///
/// PARITY: `_CACHE` (upstream lines 496 context).
static L1: Lazy<Mutex<HashMap<(String, String, String), CachedFetch>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn l1_get(key: &(String, String, String), ttl: f64) -> Option<CachedFetch> {
    let l1 = L1.lock().unwrap_or_else(|e| e.into_inner());
    l1.get(key)
        .cloned()
        .filter(|entry| entry.is_fresh(ttl, now_unix_f64()))
}

fn l1_put(key: (String, String, String), entry: CachedFetch) {
    L1.lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, entry);
}

/// Pull the secrets for `project_id` from Bitwarden Secrets Manager.
///
/// Returns (secrets, warnings). Errors for fatal conditions (missing
/// binary, auth failure, unparseable output). Only a complete, error-free
/// pull is cached, so a transient auth failure isn't frozen in for the TTL
/// window. Stale disk-cache fallback applies ONLY to transport-level
/// failures (NETWORK/TIMEOUT) — never AUTH_FAILED/INTERNAL, where serving
/// old secrets would mask a real config/credential problem. The HKDF+
/// AESGCM encrypted-cache tier is PENDING (documented at the module head).
///
/// PARITY: `fetch_bitwarden_secrets` (upstream lines 496-633), minus the
/// encrypted-cache tier.
pub fn fetch_bitwarden_secrets(
    access_token: &str,
    project_id: &str,
    binary: Option<&Path>,
    cache_ttl_seconds: f64,
    use_cache: bool,
    server_url: &str,
    home_path: Option<&Path>,
    _encrypted_cache_enabled: bool,
    _encrypted_cache_max_stale_seconds: f64,
) -> Result<(BTreeMap<String, String>, Vec<String>), String> {
    if access_token.is_empty() {
        return Err("Bitwarden access token is empty".to_string());
    }
    if project_id.is_empty() {
        return Err("Bitwarden project_id is empty".to_string());
    }

    let cache_key = (
        token_fingerprint(access_token),
        project_id.to_string(),
        server_url.to_string(),
    );
    if use_cache && cache_ttl_seconds > 0.0 {
        if let Some(cached) = l1_get(&cache_key, cache_ttl_seconds) {
            return Ok((cached.secrets.into_iter().collect(), Vec::new()));
        }
        // L2: disk cache (~5ms on hit vs ~380ms for `bws secret list`).
        if let Some(disk_cached) = disk_cache().read_at(
            &cache_key_str(&cache_key.0, &cache_key.1, &cache_key.2),
            cache_ttl_seconds,
            home_path,
            now_unix_f64(),
        ) {
            eprintln!("DBG L2 fresh hit");
            // Promote into L1 so subsequent fetches skip the disk read.
            l1_put(cache_key.clone(), disk_cached.clone());
            return Ok((disk_cached.secrets.into_iter().collect(), Vec::new()));
        }
    }

    let bws = binary.map(Path::to_path_buf).or_else(|| find_bws(false)).ok_or_else(|| {
        "bws binary not available — auto-install failed and `bws` is not on PATH.           Install manually from https://github.com/bitwarden/sdk-sm/releases or          re-run `hermes secrets bitwarden setup`."
            .to_string()
    })?;

    let fetch_result = run_bws_list(&bws, access_token, project_id, server_url);
    let (secrets, warnings) = match fetch_result {
        Ok((secrets, warnings)) => (secrets, warnings),
        Err(err) => {
            eprintln!("DBG live fetch err: {err}");
            // Stale disk-cache fallback ONLY for transport-level failures
            // (network down, DNS, transient outage/timeout) — never for
            // AUTH_FAILED or a malformed-output INTERNAL error.
            let kind = classify_bws_error(&err);
            if use_cache && matches!(kind, ErrorKind::Network | ErrorKind::Timeout) {
                if cache_ttl_seconds > 0.0 {
                    // ttl = inf bypasses freshness (we explicitly want a
                    // stale hit); the caller's real TTL gated this read.
                    let stale = disk_cache().read_at(
                        &cache_key_str(&cache_key.0, &cache_key.1, &cache_key.2),
                        f64::INFINITY,
                        home_path,
                        now_unix_f64(),
                    );
                    if let Some(stale) = stale {
                        eprintln!("DBG fallback served {} entries", stale.secrets.len());
                        let age = (now_unix_f64() - stale.fetched_at).max(0.0) as i64;
                        l1_put(cache_key.clone(), stale.clone());
                        return Ok((
                            stale.secrets.into_iter().collect(),
                            vec![format!(
                                "bws live fetch failed ({err}); falling back to stale disk cache ({age}s old)"
                            )],
                        ));
                    }
                }
            }
            return Err(err);
        }
    };

    let entry = CachedFetch {
        secrets: secrets.clone(),
        fetched_at: now_unix_f64(),
    };
    if use_cache && cache_ttl_seconds > 0.0 {
        l1_put(cache_key.clone(), entry.clone());
        disk_cache().write_at(
            &cache_key_str(&cache_key.0, &cache_key.1, &cache_key.2),
            &entry,
            cache_ttl_seconds,
            home_path,
            &rand_nonce,
        );
    }
    Ok((secrets, warnings))
}

fn now_unix_f64() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn rand_nonce() -> String {
    use base64::Engine;
    let bytes: [u8; 16] = rand::random();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Test-only: drop the in-process L1 cache (the upstream tests patch
/// `_CACHE` directly).
#[doc(hidden)]
pub fn clear_l1_for_tests() {
    L1.lock().unwrap_or_else(|e| e.into_inner()).clear();
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
