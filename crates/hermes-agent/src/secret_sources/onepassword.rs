//! 1Password (`op` CLI) secret source.
//!
//! PARITY: `agent/secret_sources/onepassword.py` @ b9aa928 (whole module).
//!
//! Resolve provider credentials from 1Password `op://vault/item/field`
//! references at process startup so they don't live in plaintext in
//! `~/.hermes/.env`. Each reference is resolved with a single
//! `op read -- <reference>` call. Hermes never authenticates on the
//! user's behalf — it shells out to an already-trusted, already-
//! authenticated CLI. Failures NEVER block startup: a missing binary,
//! expired auth, a bad reference, or a permission error each surface a
//! one-line warning and Hermes continues.
//!
//! Cache mechanics are shared with the other backends via the `_cache`
//! substrate: successful, complete pulls are cached in-process and on disk
//! under `<hermes_home>/cache/op_cache.json`. The disk file holds only
//! resolved secret *values*; auth material is fingerprinted (SHA-256
//! prefix), never stored.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64_URL;
use base64::Engine;
use once_cell::sync::Lazy;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::base::{
    get_source_env_var, get_source_environment_snapshot, ErrorKind, FetchResult, SecretSource,
};
use super::cache::{CachedFetch, DiskCache};

/// How long to wait for a single `op read`, in seconds.
///
/// PARITY: `_OP_RUN_TIMEOUT` (upstream line 63).
pub const OP_RUN_TIMEOUT: f64 = 30.0;

/// Default env var the official `op` CLI reads for service-account auth.
///
/// PARITY: `_DEFAULT_TOKEN_ENV` (upstream line 66).
pub const DEFAULT_TOKEN_ENV: &str = "OP_SERVICE_ACCOUNT_TOKEN";

/// Env vars the `op` child actually needs — a minimal allowlisted env
/// rather than a copy of the full post-dotenv environ (tighter blast
/// radius). `OP_SESSION_*` and the token are added dynamically.
///
/// PARITY: `_OP_ENV_ALLOWLIST` (upstream lines 72-91).
pub const OP_ENV_ALLOWLIST: [&str; 15] = [
    "PATH",
    "HOME",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "SystemRoot",
    "TMPDIR",
    "TMP",
    "TEMP",
    "XDG_CONFIG_HOME",
    "XDG_RUNTIME_DIR",
    "OP_ACCOUNT",
    "OP_CONNECT_HOST",
    "OP_CONNECT_TOKEN",
    // Lets a user skip op's desktop-app integration probe (which can hang
    // with no timeout on a wedged desktop container).
    "OP_LOAD_DESKTOP_APP_SETTINGS",
];

/// PARITY: `_DISK_CACHE_BASENAME` (upstream line 118).
pub const DISK_CACHE_BASENAME: &str = "op_cache.json";

static STATE: Lazy<Mutex<L1Cache>> = Lazy::new(|| Mutex::new(L1Cache::default()));

#[derive(Default)]
struct L1Cache {
    /// (auth_fp, account, home, refs_fp) → entry. The key folds in the
    /// home so a HERMES_HOME switch inside one process can't return
    /// another profile's secrets from L1.
    entries: HashMap<(String, String, String, String), CachedFetch>,
}

/// PARITY: `_DISK_CACHE` (upstream lines 130-134).
fn disk_cache() -> &'static DiskCache {
    static DISK: Lazy<DiskCache> = Lazy::new(|| DiskCache::new(DISK_CACHE_BASENAME));
    &DISK
}

/// Serialize a cache key for on-disk storage, omitting home_path — the
/// disk file is already partitioned by home (it lives under
/// `<home>/cache/`).
///
/// PARITY: `_disk_key_str` (upstream lines 121-129).
fn disk_key_str(cache_key: &(String, String, String, String)) -> String {
    format!("{}|{}|{}", cache_key.0, cache_key.1, cache_key.3)
}

/// PARITY: `_disk_cache_path` (upstream lines 137-139).
pub fn disk_cache_path(home_path: Option<&Path>) -> PathBuf {
    disk_cache().path(home_path)
}

/// Return `(valid_refs, warnings)` from an `env` mapping.
///
/// A reference is kept only if its target env-var name is a valid POSIX
/// name and the value is a stripped `op://…` reference. Everything else
/// produces a warning and is dropped (never fatal).
///
/// PARITY: `_validate_references` (upstream lines 147-173).
pub fn validate_references(references: Option<&Value>) -> (BTreeMap<String, String>, Vec<String>) {
    let mut valid = BTreeMap::new();
    let mut warnings = Vec::new();
    let Some(map) = references.and_then(Value::as_object) else {
        return (valid, warnings);
    };
    for (name, ref_value) in map {
        if !super::base::is_valid_env_name(name) {
            warnings.push(format!("Skipping {name:?}: not a valid env-var name"));
            continue;
        }
        let Some(ref_str) = ref_value.as_str() else {
            warnings.push(format!("Skipping {name:?}: reference is not a string"));
            continue;
        };
        let cleaned = ref_str.trim();
        if !cleaned.starts_with("op://") {
            warnings.push(format!(
                "Skipping {name:?}: {ref_value:?} is not an op:// secret reference"
            ));
            continue;
        }
        valid.insert(name.clone(), cleaned.to_string());
    }
    (valid, warnings)
}

/// SHA-256 prefix over the auth material `op` would use: the
/// service-account token, `OP_ACCOUNT`, the Connect host/token, and *all*
/// `OP_SESSION_*` vars. Signing out and into a different identity changes
/// the cache key, so a value cached under a previous identity is never
/// served under a new one. Never logged or displayed.
///
/// PARITY: `_auth_fingerprint` (upstream lines 176-198).
fn auth_fingerprint(token_env: &str) -> String {
    let token_value = get_source_env_var(token_env).unwrap_or_default();
    let account = get_source_env_var("OP_ACCOUNT").unwrap_or_default();
    let connect_host = get_source_env_var("OP_CONNECT_HOST").unwrap_or_default();
    let connect_token = get_source_env_var("OP_CONNECT_TOKEN").unwrap_or_default();
    let mut parts = vec![
        format!("token={token_value}"),
        format!("account={account}"),
        format!("connect_host={connect_host}"),
        format!("connect_token={connect_token}"),
    ];
    let mut session_keys: Vec<String> = get_source_environment_snapshot()
        .into_keys()
        .filter(|k| k.starts_with("OP_SESSION_"))
        .collect();
    session_keys.sort();
    for key in session_keys {
        parts.push(format!(
            "{key}={}",
            get_source_env_var(&key).unwrap_or_default()
        ));
    }
    let material = parts.join("\n");
    let digest = Sha256::digest(material.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    hex[..16].to_string()
}

/// SHA-256 prefix over the configured name→reference mapping.
///
/// PARITY: `_refs_fingerprint` (upstream lines 201-208).
fn refs_fingerprint(references: &BTreeMap<String, String>) -> String {
    let material: Vec<String> = references
        .iter()
        .map(|(name, reference)| format!("{name}={reference}"))
        .collect();
    let digest = Sha256::digest(material.join("\n").as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    hex[..16].to_string()
}

/// Resolve a usable `op` binary, or None.
///
/// When `binary_path` is set it is used verbatim and PATH is NOT consulted
/// — pinning an absolute path avoids trusting whatever `op` shows up first
/// on PATH. A pinned-but-missing path returns None rather than silently
/// falling back.
///
/// PARITY: `find_op` (upstream lines 211-227).
pub fn find_op(binary_path: &str) -> Option<PathBuf> {
    if !binary_path.is_empty() {
        let pinned = PathBuf::from(binary_path);
        #[cfg(unix)]
        let executable = {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(&pinned)
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        };
        #[cfg(not(unix))]
        let executable = true;
        if pinned.exists() && executable {
            return Some(pinned);
        }
        return None;
    }
    // shutil.which("op").
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in path_var.split(':') {
        if dir.is_empty() {
            continue;
        }
        let candidate = Path::new(dir).join("op");
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

/// Remove ANSI control sequences and trim, for safe message surfacing.
///
/// PARITY: `_scrub` (upstream lines 230-236) — the shared scrubber plus
/// any stray lone ESC.
fn scrub(text: &str) -> String {
    // The shared base scrubber strips CSI/OSC (incl. unterminated); drop
    // any stray lone ESC too.
    super::base::scrub_ansi(Some(text))
        .replace('\u{1b}', "")
        .trim()
        .to_string()
}

/// Build a minimal allowlisted environment for the `op` child process.
///
/// PARITY: `_op_child_env` (upstream lines 239-258). `op` reads
/// OP_SERVICE_ACCOUNT_TOKEN regardless of which env var the user
/// configured, so the value normalizes to that name here.
fn op_child_env(token_value: &str) -> HashMap<String, String> {
    let snapshot = get_source_environment_snapshot();
    let mut env = HashMap::new();
    for key in OP_ENV_ALLOWLIST {
        if let Some(val) = snapshot.get(key) {
            env.insert((*key).to_string(), val.clone());
        }
    }
    // Desktop / interactive session credentials.
    for (key, val) in &snapshot {
        if key.starts_with("OP_SESSION_") {
            env.insert(key.clone(), val.clone());
        }
    }
    if !token_value.is_empty() {
        env.insert(
            "OP_SERVICE_ACCOUNT_TOKEN".to_string(),
            token_value.to_string(),
        );
    }
    env.insert("NO_COLOR".to_string(), "1".to_string());
    env
}

/// Resolve a single `op://` reference to its value.
///
/// Errors on any failure — including returncode 0 with empty output, which
/// would otherwise silently clobber a good .env/shell credential with "".
///
/// PARITY: `_run_op_read` (upstream lines 261-317).
fn run_op_read(
    op: &Path,
    reference: &str,
    account: &str,
    token_value: &str,
) -> Result<String, String> {
    use std::io::Read;
    use std::process::{Command, Stdio};

    let mut command = Command::new(op);
    command.arg("read");
    if !account.is_empty() {
        command.args(["--account", account]);
    }
    // `--` terminates option parsing so a reference can never be mis-parsed
    // as an `op` flag.
    command.arg("--").arg(reference);
    command
        .env_clear()
        .envs(op_child_env(token_value))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|e| format!("failed to invoke op: {e}"))?;
    let deadline = Instant::now() + std::time::Duration::from_secs_f64(OP_RUN_TIMEOUT);
    let mut code: Option<i32> = None;
    let mut timed_out = false;
    while code.is_none() {
        match child.try_wait() {
            Ok(Some(status)) => code = Some(status.code().unwrap_or(-1)),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => return Err(format!("failed to invoke op: {e}")),
        }
    }
    if timed_out {
        return Err(format!(
            "op read timed out after {OP_RUN_TIMEOUT}s for {reference:?}"
        ));
    }
    let mut stdout_buf = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_end(&mut stdout_buf);
    }
    let mut stderr_buf = Vec::new();
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_end(&mut stderr_buf);
    }
    let exit_code = code.unwrap_or(-1);
    if exit_code != 0 {
        let err = scrub(&String::from_utf8_lossy(&stderr_buf));
        let err: String = err.chars().take(200).collect();
        if !err.is_empty() {
            return Err(format!("op read failed for {reference:?}: {err}"));
        }
        return Err(format!("op read exited {exit_code} for {reference:?}"));
    }

    // `op` appends a trailing newline; strip only that so a value with
    // intentional internal/edge spaces survives. Empty/whitespace-only is
    // treated as empty: applying it would silently clobber a good
    // .env/shell credential with effectively nothing.
    let value = String::from_utf8_lossy(&stdout_buf)
        .trim_end_matches(['\r', '\n'])
        .to_string();
    if value.trim().is_empty() {
        return Err(format!("op read returned an empty value for {reference:?}"));
    }
    Ok(value)
}

/// Resolve `references` (name → `op://…`) to `(secrets, warnings)`.
///
/// Raises only when no `op` binary is available — a fatal "can't fetch
/// anything" condition. Per-reference failures are collected as warnings
/// and the reference is dropped. Only a complete, error-free pull is
/// cached, so a transient auth failure isn't frozen in for the TTL window.
///
/// PARITY: `fetch_onepassword_secrets` (upstream lines 321-391).
pub fn fetch_onepassword_secrets(
    references: &BTreeMap<String, String>,
    account: &str,
    token_env: &str,
    binary: Option<&Path>,
    binary_path: &str,
    use_cache: bool,
    cache_ttl_seconds: f64,
    home_path: Option<&Path>,
) -> Result<(BTreeMap<String, String>, Vec<String>), String> {
    let (valid, mut warnings) = validate_references(Some(
        &serde_json::to_value(references).unwrap_or(Value::Null),
    ));
    if valid.is_empty() {
        return Ok((BTreeMap::new(), warnings));
    }

    let token_value = get_source_env_var(token_env)
        .unwrap_or_default()
        .trim()
        .to_string();
    let home_str = home_path
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let cache_key = (
        auth_fingerprint(token_env),
        account.to_string(),
        home_str,
        refs_fingerprint(&valid),
    );

    if use_cache {
        let cached = STATE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entries
            .get(&cache_key)
            .cloned();
        if let Some(cached) = cached.filter(|c| c.is_fresh(cache_ttl_seconds, now_unix())) {
            return Ok((cached.secrets.into_iter().collect(), warnings));
        }
        if let Some(disk_cached) = disk_cache().read_at(
            &disk_key_str(&cache_key),
            cache_ttl_seconds,
            home_path,
            now_unix(),
        ) {
            // Promote into L1 so later fetches in this process skip the disk.
            STATE
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entries
                .insert(cache_key.clone(), disk_cached.clone());
            return Ok((disk_cached.secrets.into_iter().collect(), warnings));
        }
    }

    let op = match binary {
        Some(op) => Some(op.to_path_buf()),
        None => find_op(binary_path),
    };
    let Some(op) = op else {
        return Err("op CLI not found.  Install the 1Password CLI \
             (https://developer.1password.com/docs/cli/get-started/) or set \
             secrets.onepassword.binary_path to its absolute location."
            .to_string());
    };

    let mut secrets = BTreeMap::new();
    let mut read_errors = 0;
    for name in valid.keys() {
        match run_op_read(&op, &valid[name], account, &token_value) {
            Ok(value) => {
                secrets.insert(name.clone(), value);
            }
            Err(err) => {
                warnings.push(err);
                read_errors += 1;
            }
        }
    }

    if use_cache && read_errors == 0 && !secrets.is_empty() {
        let entry = CachedFetch {
            secrets: secrets.clone(),
            fetched_at: now_unix(),
        };
        STATE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entries
            .insert(cache_key.clone(), entry.clone());
        let nonce = rand_nonce();
        disk_cache().write_at(
            &disk_key_str(&cache_key),
            &entry,
            cache_ttl_seconds,
            home_path,
            &|| B64_URL.encode(nonce),
        );
    }

    Ok((secrets, warnings))
}

fn now_unix() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn rand_nonce() -> [u8; 16] {
    rand::random()
}

/// 1Password as a registered secret source — a **mapped** source: the user
/// explicitly binds each env var to an `op://` reference, so its claims
/// outrank bulk sources on contested vars.
///
/// PARITY: `OnePasswordSource` (upstream lines 497-638).
#[derive(Default)]
pub struct OnePasswordSource;

impl SecretSource for OnePasswordSource {
    fn name(&self) -> &str {
        "onepassword"
    }
    fn label(&self) -> &str {
        "1Password"
    }
    fn shape(&self) -> &str {
        "mapped"
    }
    fn scheme(&self) -> Option<&str> {
        Some("op")
    }

    /// Default True: an explicit VAR→op:// binding is the strongest user
    /// intent there is — leaving a stale .env line in place should not
    /// silently defeat it.
    fn override_existing(&self, cfg: &Value) -> bool {
        cfg.get("override_existing")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    }

    /// The source's own bootstrap-auth var (the service-account token env)
    /// so a resolved secret can never clobber the credential used to auth.
    fn protected_env_vars(&self) -> Vec<String> {
        vec![DEFAULT_TOKEN_ENV.to_string()]
    }

    fn config_schema(&self) -> Value {
        serde_json::json!({
            "enabled": {"description": "Master switch", "default": false},
            "env": {"description": "Map of ENV_VAR -> op://vault/item/field reference", "default": {}},
            "account": {"description": "op --account shorthand (empty = default account)", "default": ""},
            "service_account_token_env": {"description": "Env var holding the service-account token (unset = desktop/interactive session)", "default": DEFAULT_TOKEN_ENV},
            "binary_path": {"description": "Pin the op binary (empty = resolve via PATH)", "default": ""},
            "cache_ttl_seconds": {"description": "Disk+memory cache TTL; 0 disables", "default": 300},
            "override_existing": {"description": "Resolved values overwrite .env/shell values", "default": true},
        })
    }

    fn fetch(&self, cfg: &Value, home_path: &Path) -> FetchResult {
        let mut result = FetchResult::default();
        let empty_map = serde_json::Map::new();
        let env_map = cfg
            .get("env")
            .and_then(Value::as_object)
            .unwrap_or(&empty_map);
        let env_value = Value::Object(env_map.clone());
        let (valid, mut warnings) = validate_references(Some(&env_value));
        result.warnings.append(&mut warnings);
        if valid.is_empty() {
            if result.warnings.is_empty() {
                result.error = Some(
                    "secrets.onepassword.enabled is true but the env: map is empty. \
                     Add ENV_VAR: op://vault/item/field entries."
                        .to_string(),
                );
                result.error_kind = Some(ErrorKind::NotConfigured);
            }
            return result;
        }

        let binary_path = cfg
            .get("binary_path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let binary = find_op(&binary_path);
        result.binary_path = binary.clone();
        let Some(binary) = binary else {
            if !binary_path.is_empty() {
                result.error = Some(format!(
                    "secrets.onepassword.binary_path ({binary_path:?}) is not an executable op binary."
                ));
            } else {
                result.error = Some(
                    "secrets.onepassword.enabled is true but the op CLI was not found on PATH. \
                     Install it (https://developer.1password.com/docs/cli/get-started/) \
                     or set secrets.onepassword.binary_path."
                        .to_string(),
                );
            }
            result.error_kind = Some(ErrorKind::BinaryMissing);
            return result;
        };

        let ttl = cfg
            .get("cache_ttl_seconds")
            .and_then(Value::as_f64)
            .unwrap_or(300.0);

        let token_env = cfg
            .get("service_account_token_env")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_TOKEN_ENV);
        let account = cfg.get("account").and_then(Value::as_str).unwrap_or("");

        match fetch_onepassword_secrets(
            &valid,
            account,
            token_env,
            Some(&binary),
            &binary_path,
            true,
            ttl,
            Some(home_path),
        ) {
            Ok((secrets, mut fetch_warnings)) => {
                result.secrets = secrets.into_iter().collect();
                result.warnings.append(&mut fetch_warnings);
                result
            }
            Err(err) => {
                result.error = Some(err.clone());
                result.error_kind = Some(classify_op_error(&err));
                result
            }
        }
    }

    fn remediation(&self, kind: Option<ErrorKind>) -> Option<String> {
        match kind {
            Some(ErrorKind::AuthFailed) | Some(ErrorKind::AuthExpired) => Some(
                "Run `hermes secrets onepassword token` to paste a fresh \
                 service-account token (OP_SERVICE_ACCOUNT_TOKEN), or `op signin` \
                 for an interactive session."
                    .to_string(),
            ),
            Some(ErrorKind::BinaryMissing) => Some(
                "Install the 1Password CLI \
                 (https://developer.1password.com/docs/cli/get-started/) or set \
                 secrets.onepassword.binary_path."
                    .to_string(),
            ),
            _ => None,
        }
    }
}

/// Best-effort mapping of op failure text onto the shared taxonomy.
///
/// PARITY: `_classify_op_error` (upstream lines 640-663).
pub fn classify_op_error(message: &str) -> ErrorKind {
    let lowered = message.to_lowercase();
    if lowered.contains("timed out") {
        return ErrorKind::Timeout;
    }
    if lowered.contains("not found on path")
        || lowered.contains("not an executable")
        || lowered.contains("failed to invoke")
    {
        return ErrorKind::BinaryMissing;
    }
    if [
        "unauthorized",
        "not signed in",
        "session expired",
        "authentication",
        "401",
        "403",
    ]
    .iter()
    .any(|tok| lowered.contains(tok))
    {
        return ErrorKind::AuthFailed;
    }
    if lowered.contains("empty value") {
        return ErrorKind::EmptyValue;
    }
    if ["network", "connection", "resolve host", "dns"]
        .iter()
        .any(|tok| lowered.contains(tok))
    {
        return ErrorKind::Network;
    }
    ErrorKind::Internal
}

/// Drop in-process AND disk caches — used after a token rotation so the
/// next startup resolves fresh with the new credential.
///
/// PARITY: `clear_caches` (upstream lines 666-673).
pub fn clear_caches(home_path: Option<&Path>) {
    STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entries
        .clear();
    disk_cache().clear(home_path);
}
