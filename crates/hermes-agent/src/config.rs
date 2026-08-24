//! Read-only outer configuration discovery for the agent layer.
//!
//! This is the narrow, fail-open equivalent of the raw config read used by
//! `hermes_cli.config.load_config_readonly` (upstream `hermes_cli/config.py`
//! lines 2968–2978). It deliberately does not apply defaults, overlays,
//! migrations, or write-back. The root map is retained as the `pool_config`
//! input expected by [`crate::credential_pool::pool_strategy_from_config`],
//! while custom providers use the existing compatibility normalizer from
//! [`crate::credential_pool::get_compatible_custom_providers`].

use crate::credential_pool::get_compatible_custom_providers;
use hermes_constants::get_config_path;
use hermes_utils::fast_safe_load;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::UNIX_EPOCH;

/// Filesystem signature used to decide whether a cached snapshot is stale.
///
/// The first component is modification time in nanoseconds since the Unix
/// epoch and the second is the file size. This mirrors the source loader's
/// cheap path/signature invalidation boundary without hashing config content.
pub type ConfigSignature = (u128, u64);

/// Clone-safe read-only view of the outer config needed by agent consumers.
///
/// A snapshot owns all parsed values and can therefore be retained or cloned
/// without borrowing the loader cache. Invalid, unreadable, and non-mapping
/// config files produce an empty map and no derived sections unless a valid
/// snapshot for the same path already exists, in which case it is retained.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigSnapshot {
    /// The path used for this load, resolved by [`get_config_path`] for the
    /// process-default entry point.
    pub path: PathBuf,
    /// `(mtime_ns, size)` when metadata was available, otherwise `None`.
    pub signature: Option<ConfigSignature>,
    /// The parsed root map, passed as `pool_config` to pool strategy helpers.
    pub pool_config: Map<String, Value>,
    /// A cloned `model` map when the root contains an object at that key.
    pub model_config: Option<Map<String, Value>>,
    /// Legacy/keyed custom providers normalized to one list-shaped view.
    pub custom_providers: Vec<Value>,
}

impl ConfigSnapshot {
    fn empty(path: PathBuf, signature: Option<ConfigSignature>) -> Self {
        Self {
            path,
            signature,
            pool_config: Map::new(),
            model_config: None,
            custom_providers: Vec::new(),
        }
    }

    /// Return the requested/resolved config path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the current `(mtime_ns, size)` signature, when available.
    pub fn signature(&self) -> Option<ConfigSignature> {
        self.signature
    }

    /// Return the root map used as credential-pool configuration.
    pub fn pool_config(&self) -> &Map<String, Value> {
        &self.pool_config
    }

    /// Return the optional cloned model map.
    pub fn model_config(&self) -> Option<&Map<String, Value>> {
        self.model_config.as_ref()
    }

    /// Return normalized custom-provider entries.
    pub fn custom_providers(&self) -> &[Value] {
        &self.custom_providers
    }
}
#[derive(Debug, Clone)]
struct CacheEntry {
    signature: Option<ConfigSignature>,
    snapshot: ConfigSnapshot,
    valid: bool,
}

static SNAPSHOT_CACHE: LazyLock<Mutex<HashMap<PathBuf, CacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn snapshot_cache() -> &'static Mutex<HashMap<PathBuf, CacheEntry>> {
    &SNAPSHOT_CACHE
}

fn file_signature(path: &Path) -> Option<ConfigSignature> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let mtime_ns = modified.duration_since(UNIX_EPOCH).ok()?.as_nanos();
    Some((mtime_ns, metadata.len()))
}
fn parse_snapshot(path: &Path, signature: Option<ConfigSignature>) -> Option<ConfigSnapshot> {
    let path = path.to_path_buf();
    let mut snapshot = ConfigSnapshot::empty(path.clone(), signature);
    let text = std::fs::read_to_string(&path).ok()?;
    let yaml = fast_safe_load(&text).ok()?;
    let json = serde_json::to_value(yaml).ok()?;
    let root = json.as_object()?;

    snapshot.model_config = root.get("model").and_then(Value::as_object).cloned();
    snapshot.custom_providers = get_compatible_custom_providers(root);
    snapshot.pool_config = root.clone();
    Some(snapshot)
}

/// Load the process-default config path from [`hermes_constants::get_config_path`].
///
/// PARITY: `hermes_cli.config.load_config_readonly` (upstream
/// `hermes_cli/config.py` lines 2968–2978), limited to raw map discovery.
pub fn load_config_snapshot() -> ConfigSnapshot {
    load_config_snapshot_at(&get_config_path())
}

/// Load a read-only config snapshot from an explicit path.
/// The cache is keyed by path and reused only while its `(mtime_ns, size)`
/// signature is unchanged. If a previously valid snapshot exists, a later
/// malformed, non-map, or unreadable revision serves that last-known-good
/// value; an invalid path with no valid history fails open to empty.
///
/// PARITY: `hermes_cli.config.load_config_readonly` (upstream
/// `hermes_cli/config.py` lines 2968–2978) and
/// `get_compatible_custom_providers` (lines 1532–1578).
pub fn load_config_snapshot_at(path: &Path) -> ConfigSnapshot {
    let path = path.to_path_buf();
    let signature = file_signature(&path);
    let mut cache = snapshot_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let previous = cache.get(&path).cloned();
    if let Some(entry) = previous.as_ref() {
        if entry.signature == signature {
            return entry.snapshot.clone();
        }
    }

    if let Some(snapshot) = parse_snapshot(&path, signature) {
        cache.insert(
            path,
            CacheEntry {
                signature,
                snapshot: snapshot.clone(),
                valid: true,
            },
        );
        return snapshot;
    }

    if let Some(entry) = previous.filter(|entry| entry.valid) {
        return entry.snapshot;
    }

    let snapshot = ConfigSnapshot::empty(path.clone(), signature);
    cache.insert(
        path,
        CacheEntry {
            signature,
            snapshot: snapshot.clone(),
            valid: false,
        },
    );
    snapshot
}
