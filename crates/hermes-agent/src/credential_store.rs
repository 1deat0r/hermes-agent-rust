//! Auth-store persistence and credential-pool disk boundaries from
//! `hermes_cli/auth.py`.
//!
//! This module owns the JSON schema/migration and atomic file boundary. The
//! higher-level OAuth refresh, environment/config seeding, and cross-process
//! lock orchestration remain separate leaves, as they do in the upstream
//! module's call graph.

use crate::credential_pool::sanitize_borrowed_credential_payload;
use chrono::Utc;
use hermes_constants::{get_default_hermes_root, get_hermes_home, secure_parent_dir};
use hermes_utils::atomic_replace;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use url::Url;

/// Version written to every native auth store.
pub const AUTH_STORE_VERSION: i64 = 1;

/// Current default Nous Portal URL used by the load-time migration.
pub const DEFAULT_NOUS_PORTAL_URL: &str = "https://portal.nousresearch.com";

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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
/// This is the final borrowed-secret disk boundary. Cross-process locking and
/// cooldown recency merging are intentionally separate orchestration seams;
/// callers must serialize this read-modify-write transaction until that leaf
/// is ported.
///
/// PARITY: `hermes_cli.auth.write_credential_pool` (1650–1714).
pub fn write_credential_pool_at(
    auth_file: &Path,
    provider_id: &str,
    entries: &[Value],
    removed_ids: &[String],
) -> io::Result<PathBuf> {
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
    let mut merged = sanitized_entries;
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
