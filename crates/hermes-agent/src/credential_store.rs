//! Auth-store persistence and credential-pool disk boundaries from
//! `hermes_cli/auth.py`.
//!
//! This module owns the JSON schema/migration, per-store advisory lock, and
//! atomic file boundary. The higher-level OAuth refresh and
//! environment/config seeding remain separate leaves, as they do in the
//! upstream module's call graph.

use crate::credential_pool::{
    exhausted_until, sanitize_borrowed_credential_payload, PooledCredential,
};
use chrono::Utc;
use fs2::FileExt;
use hermes_constants::{get_default_hermes_root, get_hermes_home, secure_parent_dir};
use hermes_utils::atomic_replace;
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use url::Url;

/// Version written to every native auth store.
pub const AUTH_STORE_VERSION: i64 = 1;

/// Default cross-process auth-store lock timeout.
pub const AUTH_LOCK_TIMEOUT_SECONDS: f64 = 15.0;

/// Current default Nous Portal URL used by the load-time migration.
pub const DEFAULT_NOUS_PORTAL_URL: &str = "https://portal.nousresearch.com";

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static AUTH_LOCK_HOLDERS: RefCell<HashMap<PathBuf, usize>> = RefCell::new(HashMap::new());
}

/// An acquired per-auth-store advisory lock.
///
/// The guard is reentrant for the current thread and keyed by the canonical
/// auth-store path, while the underlying `.lock` file serializes other
/// threads/processes through `fs2`'s platform-native exclusive lock.
pub struct AuthStoreLockGuard {
    key: PathBuf,
    file: Option<File>,
}

impl Drop for AuthStoreLockGuard {
    fn drop(&mut self) {
        let outermost = AUTH_LOCK_HOLDERS.with(|holders| {
            let mut holders = holders.borrow_mut();
            let Some(depth) = holders.get_mut(&self.key) else {
                return false;
            };
            if *depth > 1 {
                *depth -= 1;
                false
            } else {
                holders.remove(&self.key);
                true
            }
        });
        if outermost {
            if let Some(file) = self.file.as_ref() {
                // PARITY: `_file_lock` ignores unlock failures during cleanup.
                let _ = file.unlock();
            }
        }
    }
}

/// Acquire the active auth-store lock with the source's default timeout.
///
/// PARITY: `hermes_cli.auth._auth_store_lock` (1187–1213).
pub fn auth_store_lock(target_path: Option<&Path>) -> io::Result<AuthStoreLockGuard> {
    auth_store_lock_with_timeout(target_path, AUTH_LOCK_TIMEOUT_SECONDS)
}

/// Acquire one auth-store lock with an explicit timeout, primarily for parity
/// tests and callers that need a bounded transaction.
pub fn auth_store_lock_with_timeout(
    target_path: Option<&Path>,
    timeout_seconds: f64,
) -> io::Result<AuthStoreLockGuard> {
    let auth_path = target_path.map_or_else(auth_file_path, Path::to_path_buf);
    let key = lock_holder_key(&auth_path);
    if let Some(guard) = AUTH_LOCK_HOLDERS.with(|holders| {
        let mut holders = holders.borrow_mut();
        let depth = holders.get_mut(&key)?;
        *depth += 1;
        Some(AuthStoreLockGuard {
            key: key.clone(),
            file: None,
        })
    }) {
        return Ok(guard);
    }

    let lock_path = auth_path.with_extension("lock");
    if let Some(parent) = lock_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    let deadline = lock_deadline(timeout_seconds);
    loop {
        match file.try_lock_exclusive() {
            Ok(()) => break,
            Err(_) if deadline.is_none_or(|deadline| Instant::now() < deadline) => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Timed out waiting for auth store lock",
                ));
            }
        }
    }

    AUTH_LOCK_HOLDERS.with(|holders| {
        holders.borrow_mut().insert(key.clone(), 1);
    });
    Ok(AuthStoreLockGuard {
        key,
        file: Some(file),
    })
}

fn lock_holder_key(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        return canonical;
    }
    if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name()) {
        if let Ok(canonical_parent) = fs::canonicalize(parent) {
            return canonical_parent.join(file_name);
        }
    }
    path.to_path_buf()
}

fn lock_deadline(timeout_seconds: f64) -> Option<Instant> {
    if timeout_seconds.is_infinite() && timeout_seconds.is_sign_positive() {
        return None;
    }
    let seconds = if timeout_seconds.is_finite() {
        timeout_seconds.max(1.0)
    } else {
        1.0
    };
    Some(Instant::now() + Duration::from_secs_f64(seconds))
}

/// Return the active `~/.hermes/auth.json` path.
///
/// PARITY: `hermes_cli.auth._auth_file_path` (1000–1018).
pub fn auth_file_path() -> PathBuf {
    get_hermes_home().join("auth.json")
}

/// Load one auth store, preserving the upstream distinction between an I/O
/// read failure and malformed JSON.
///
/// A missing file and a malformed file both produce the source's empty store;
/// an existing file that cannot be read returns the I/O error so a later
/// read-modify-write cannot silently erase credentials.
///
/// PARITY: `hermes_cli.auth._load_auth_store` (1215–1281).
pub fn load_auth_store(auth_file: Option<&Path>) -> io::Result<Map<String, Value>> {
    let path = auth_file.map_or_else(auth_file_path, Path::to_path_buf);
    if !path.exists() {
        return Ok(empty_auth_store());
    }

    let bytes = fs::read(&path)?;
    let raw = match serde_json::from_slice::<Value>(&bytes) {
        Ok(raw) => raw,
        Err(_) => {
            // PARITY: malformed JSON/UTF-8 is quarantined beside auth.json;
            // failure to create the sidecar must not turn a fail-open load
            // into a write failure.
            let corrupt_path = path.with_extension("json.corrupt");
            let _ = fs::copy(&path, corrupt_path);
            return Ok(empty_auth_store());
        }
    };

    let Value::Object(mut raw) = raw else {
        return Ok(empty_auth_store());
    };

    if raw.get("providers").is_some_and(Value::is_object)
        || raw.get("credential_pool").is_some_and(Value::is_object)
    {
        if !raw.get("providers").is_some_and(Value::is_object) {
            raw.insert("providers".into(), Value::Object(Map::new()));
        }
        if let Some(providers) = raw.get_mut("providers").and_then(Value::as_object_mut) {
            migrate_stale_nous_portal_url(providers);
        }
        return Ok(raw);
    }

    // PARITY: migration from the older PR `systems` format.
    if let Some(systems) = raw.get("systems").and_then(Value::as_object) {
        let mut migrated = empty_auth_store();
        let mut providers = Map::new();
        if let Some(nous) = systems.get("nous_portal") {
            providers.insert("nous".into(), nous.clone());
        }
        migrated.insert("providers".into(), Value::Object(providers.clone()));
        migrated.insert(
            "active_provider".into(),
            if providers.is_empty() {
                Value::Null
            } else {
                Value::String("nous".into())
            },
        );
        return Ok(migrated);
    }

    Ok(empty_auth_store())
}

/// Save one auth store atomically with owner-only file permissions.
///
/// The temporary file is created with `create_new`/`0o600` before any token
/// bytes are written, then flushed, replaced, and followed by a best-effort
/// directory fsync. The parent is tightened to `0o700` through the shared
/// path helper.
///
/// PARITY: `hermes_cli.auth._save_auth_store` (1284–1335).
pub fn save_auth_store(
    auth_store: &mut Map<String, Value>,
    target_path: Option<&Path>,
) -> io::Result<PathBuf> {
    let auth_file = target_path.map_or_else(auth_file_path, Path::to_path_buf);
    if let Some(parent) = auth_file.parent() {
        fs::create_dir_all(parent)?;
    }
    secure_parent_dir(&auth_file);

    auth_store.insert("version".into(), Value::from(AUTH_STORE_VERSION));
    auth_store.insert("updated_at".into(), Value::String(Utc::now().to_rfc3339()));
    let mut payload =
        serde_json::to_vec_pretty(&Value::Object(auth_store.clone())).map_err(io::Error::other)?;
    payload.push(b'\n');

    let file_name = auth_file.file_name().map_or_else(
        || "auth.json".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let sequence = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_path = auth_file.with_file_name(format!(
        "{file_name}.tmp.{}.{}",
        std::process::id(),
        sequence
    ));

    let write_result = (|| -> io::Result<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp_path)?;
        file.write_all(&payload)?;
        file.flush()?;
        file.sync_all()?;

        // `atomic_replace` preserves the upstream symlink-aware replacement
        // helper. Its fallback is best-effort by contract, as in utils.py.
        let _ = atomic_replace(&tmp_path, &auth_file);

        #[cfg(unix)]
        if let Some(parent) = auth_file.parent() {
            if let Ok(directory) = File::open(parent) {
                let _ = directory.sync_all();
            }
        }
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    write_result?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&auth_file, fs::Permissions::from_mode(0o600));
    }
    let _ = fs::remove_file(&tmp_path);
    Ok(auth_file)
}

/// Read a pool using explicit active/global paths.
///
/// This path-oriented adapter keeps profile fallback deterministic in parity
/// tests while the public [`read_credential_pool`] resolves the active paths
/// from the process environment.
///
/// PARITY: `hermes_cli.auth.read_credential_pool` (1536–1580).
pub fn read_credential_pool_at(
    profile_path: Option<&Path>,
    global_path: Option<&Path>,
    provider_id: Option<&str>,
) -> io::Result<Value> {
    let profile_store = match profile_path {
        Some(path) => load_auth_store(Some(path))?,
        None => empty_auth_store(),
    };
    let profile_pool = pool_from_store(&profile_store);

    // Global fallback is read-only and fail-open: a malformed/unreadable
    // global file must not make a profile process unusable.
    let global_pool = global_path
        .and_then(|path| load_auth_store(Some(path)).ok())
        .map_or_else(Map::new, |store| pool_from_store(&store));

    match provider_id {
        Some(provider) => {
            if let Some(entries) = nonempty_array(profile_pool.get(provider)) {
                return Ok(Value::Array(entries.to_vec()));
            }
            Ok(nonempty_array(global_pool.get(provider)).map_or_else(
                || Value::Array(Vec::new()),
                |entries| Value::Array(entries.to_vec()),
            ))
        }
        None => {
            let mut merged = profile_pool;
            for (provider, entries) in global_pool {
                let Some(global_entries) = entries.as_array().filter(|entries| !entries.is_empty())
                else {
                    continue;
                };
                let profile_has_entries = merged
                    .get(&provider)
                    .and_then(Value::as_array)
                    .is_some_and(|entries| !entries.is_empty());
                if !profile_has_entries {
                    merged.insert(provider, Value::Array(global_entries.clone()));
                }
            }
            Ok(Value::Object(merged))
        }
    }
}

/// Read the active credential pool, applying the profile/global fallback.
pub fn read_credential_pool(provider_id: Option<&str>) -> io::Result<Value> {
    let active = auth_file_path();
    let global = global_auth_file_path();
    read_credential_pool_at(Some(&active), global.as_deref(), provider_id)
}

/// Write one provider pool to an explicit auth-store path.
///
/// This is the final borrowed-secret disk boundary. The source's per-path
/// cross-process lock and newer/live cooldown recency merge are applied here.
///
/// PARITY: `hermes_cli.auth.write_credential_pool` (1650–1714).
pub fn write_credential_pool_at(
    auth_file: &Path,
    provider_id: &str,
    entries: &[Value],
    removed_ids: &[String],
) -> io::Result<PathBuf> {
    let _lock = auth_store_lock(Some(auth_file))?;
    let mut auth_store = load_auth_store(Some(auth_file))?;
    let pool = auth_store
        .entry("credential_pool")
        .or_insert_with(|| Value::Object(Map::new()));
    if !pool.is_object() {
        *pool = Value::Object(Map::new());
    }
    let pool_map = pool.as_object_mut().expect("pool normalized to object");

    let sanitized_entries: Vec<Value> = entries
        .iter()
        .map(|entry| sanitize_entry(entry, provider_id))
        .collect();
    let removed: HashSet<String> = removed_ids.iter().cloned().collect();
    let new_ids: HashSet<String> = sanitized_entries
        .iter()
        .filter_map(|entry| entry.get("id").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect();

    let existing = pool_map
        .get(provider_id)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let existing_by_id: Map<String, Value> = existing
        .iter()
        .filter_map(|entry| {
            entry
                .get("id")
                .and_then(Value::as_str)
                .map(|id| (id.to_owned(), entry.clone()))
        })
        .collect();
    let mut merged: Vec<Value> = sanitized_entries
        .into_iter()
        .map(|entry| {
            let disk_entry = entry
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| existing_by_id.get(id));
            merge_disk_cooldown_state(&entry, disk_entry, provider_id)
        })
        .collect();
    for entry in existing {
        let Some(id) = entry.get("id").and_then(Value::as_str) else {
            continue;
        };
        if new_ids.contains(id) || removed.contains(id) {
            continue;
        }
        merged.push(sanitize_entry(&entry, provider_id));
    }
    pool_map.insert(provider_id.to_owned(), Value::Array(merged));
    save_auth_store(&mut auth_store, Some(auth_file))
}

/// Write one provider pool to the active auth store.
pub fn write_credential_pool(
    provider_id: &str,
    entries: &[Value],
    removed_ids: Option<&[String]>,
) -> io::Result<PathBuf> {
    write_credential_pool_at(
        &auth_file_path(),
        provider_id,
        entries,
        removed_ids.unwrap_or(&[]),
    )
}

/// Return the global-root auth store path in profile mode, or `None` in
/// classic mode.
pub fn global_auth_file_path() -> Option<PathBuf> {
    let profile_home = get_hermes_home();
    let global_root = get_default_hermes_root();
    if same_path(&profile_home, &global_root) {
        None
    } else {
        Some(global_root.join("auth.json"))
    }
}

fn empty_auth_store() -> Map<String, Value> {
    let mut store = Map::new();
    store.insert("version".into(), Value::from(AUTH_STORE_VERSION));
    store.insert("providers".into(), Value::Object(Map::new()));
    store
}

fn pool_from_store(store: &Map<String, Value>) -> Map<String, Value> {
    store
        .get("credential_pool")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn nonempty_array(value: Option<&Value>) -> Option<&Vec<Value>> {
    value
        .and_then(Value::as_array)
        .filter(|entries| !entries.is_empty())
}

fn sanitize_entry(entry: &Value, provider_id: &str) -> Value {
    entry.as_object().map_or_else(
        || entry.clone(),
        |object| {
            Value::Object(sanitize_borrowed_credential_payload(
                object.clone(),
                provider_id,
            ))
        },
    )
}

const POOL_STATUS_FIELDS: &[&str] = &[
    "last_status",
    "last_status_at",
    "last_error_code",
    "last_error_reason",
    "last_error_message",
    "last_error_reset_at",
];

/// Keep a newer, still-binding disk cooldown over a stale in-memory row.
///
/// PARITY: `hermes_cli.auth._merge_disk_cooldown_state` (1593–1647).
fn merge_disk_cooldown_state(
    entry: &Value,
    disk_entry: Option<&Value>,
    provider_id: &str,
) -> Value {
    let (Some(entry_map), Some(disk_map)) =
        (entry.as_object(), disk_entry.and_then(Value::as_object))
    else {
        return entry.clone();
    };
    let disk_status = disk_map.get("last_status").and_then(Value::as_str);
    if !matches!(disk_status, Some("dead" | "exhausted")) {
        return entry.clone();
    }

    let memory_access = entry_map
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let disk_access = disk_map
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !memory_access.is_empty() && !disk_access.is_empty() && memory_access != disk_access {
        return entry.clone();
    }

    let disk_timestamp = timestamp_value(disk_map.get("last_status_at")).unwrap_or(0.0);
    let memory_timestamp = timestamp_value(entry_map.get("last_status_at")).unwrap_or(0.0);
    if disk_timestamp <= memory_timestamp {
        return entry.clone();
    }

    if disk_status == Some("exhausted") {
        let disk_credential =
            PooledCredential::from_json(provider_id, &Value::Object(disk_map.clone()));
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        if exhausted_until(&disk_credential, false).is_none_or(|until| until <= now) {
            return entry.clone();
        }
    }

    let mut merged = entry_map.clone();
    for field in POOL_STATUS_FIELDS {
        merged.insert(
            (*field).to_owned(),
            disk_map.get(*field).cloned().unwrap_or(Value::Null),
        );
    }
    Value::Object(merged)
}

fn timestamp_value(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(number)) => number.as_f64().and_then(normalize_timestamp),
        Some(Value::String(value)) => parse_timestamp(value),
        _ => None,
    }
}

fn normalize_timestamp(value: f64) -> Option<f64> {
    if value <= 0.0 {
        return None;
    }
    Some(if value > 1_000_000_000_000.0 {
        value / 1000.0
    } else {
        value
    })
}

fn parse_timestamp(value: &str) -> Option<f64> {
    let raw = value.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(numeric) = raw.parse::<f64>() {
        if numeric <= 0.0 {
            return None;
        }
        return normalize_timestamp(numeric);
    }
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|timestamp| timestamp.timestamp_millis() as f64 / 1000.0)
        .ok()
}

fn migrate_stale_nous_portal_url(providers: &mut Map<String, Value>) {
    let Some(nous) = providers.get_mut("nous").and_then(Value::as_object_mut) else {
        return;
    };
    let Some(stored) = nous.get("portal_base_url").and_then(Value::as_str) else {
        return;
    };
    if stored.trim().is_empty() {
        return;
    }
    if Url::parse(stored)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .as_deref()
        == Some("api.nousresearch.com")
    {
        nous.insert(
            "portal_base_url".into(),
            Value::String(DEFAULT_NOUS_PORTAL_URL.into()),
        );
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}
