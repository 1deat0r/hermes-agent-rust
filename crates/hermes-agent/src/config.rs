//! Read-only configuration discovery for the agent layer.
//!
//! The raw loader mirrors `hermes_cli.config.load_config_readonly`
//! (upstream `hermes_cli/config.py` lines 2968–2978). The explicit-defaults
//! loader below additionally mirrors the generic portions of the CLI merge
//! path: recursive defaults merging, environment expansion, and legacy key
//! normalization. Managed overlays, migrations, and write-back remain above
//! this crate's boundary.
//!
//! The root map is retained as the `pool_config` input expected by
//! [`crate::credential_pool::pool_strategy_from_config`], while custom
//! providers use the existing compatibility normalizer from
//! [`crate::credential_pool::get_compatible_custom_providers`].
use crate::credential_pool::get_compatible_custom_providers;
use hermes_constants::get_config_path;
use hermes_utils::fast_safe_load;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::UNIX_EPOCH;

pub(crate) fn json_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(value)) => *value,
        Some(Value::Number(value)) => value
            .as_i64()
            .map(|n| n != 0)
            .or_else(|| value.as_u64().map(|n| n != 0))
            .or_else(|| value.as_f64().map(|n| n != 0.0))
            .unwrap_or(true),
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Array(value)) => !value.is_empty(),
        Some(Value::Object(value)) => !value.is_empty(),
    }
}

pub(crate) fn deep_merge(
    base: &Map<String, Value>,
    override_map: &Map<String, Value>,
) -> Map<String, Value> {
    let mut result = base.clone();
    for (key, value) in override_map {
        if let Some(base_value) = result.get(key) {
            if let (Some(base_object), Value::Object(override_object)) =
                (base_value.as_object(), value)
            {
                result.insert(
                    key.clone(),
                    Value::Object(deep_merge(base_object, override_object)),
                );
                continue;
            }
            if base_value.is_object() && value.is_null() {
                // PARITY: A null section in YAML does not erase a mapping
                // default (`_deep_merge`, config.py:2456–2457).
                continue;
            }
        }
        result.insert(key.clone(), value.clone());
    }
    result
}

fn source_reference(inner: &str) -> bool {
    let Some(colon) = inner.find(':') else {
        return false;
    };
    let prefix = &inner[..colon];
    !prefix.is_empty()
        && prefix.as_bytes()[0].is_ascii_lowercase()
        && prefix.as_bytes().iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_' || *byte == b'-'
        })
}

fn for_each_env_reference<F>(value: &Value, visit: &mut F)
where
    F: FnMut(&str),
{
    match value {
        Value::String(value) => {
            let mut rest = value.as_str();
            while let Some(start) = rest.find("${") {
                let after_start = &rest[start + 2..];
                let Some(end) = after_start.find('}') else {
                    break;
                };
                let inner = after_start[..end].trim();
                if let Some(name) = inner.strip_prefix("env:") {
                    let name = name.trim();
                    if !name.is_empty() {
                        visit(name);
                    }
                } else if !source_reference(inner) {
                    visit(inner);
                }
                rest = &after_start[end + 1..];
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                for_each_env_reference(value, visit);
            }
        }
        Value::Array(values) => {
            for value in values {
                for_each_env_reference(value, visit);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn env_ref_snapshot(value: &Value) -> HashMap<String, Option<String>> {
    let mut snapshot = HashMap::new();
    for_each_env_reference(value, &mut |name| {
        snapshot.insert(name.to_owned(), env::var(name).ok());
    });
    snapshot
}

fn expand_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find('}') else {
            output.push_str(&rest[start..]);
            return output;
        };
        let raw = &rest[start..start + end + 3];
        let inner = after_start[..end].trim();
        let replacement = if let Some(name) = inner.strip_prefix("env:") {
            let name = name.trim();
            if name.is_empty() {
                None
            } else {
                env::var(name).ok()
            }
        } else if source_reference(inner) {
            None
        } else {
            env::var(inner).ok()
        };
        output.push_str(replacement.as_deref().unwrap_or(raw));
        rest = &after_start[end + 1..];
    }
    output.push_str(rest);
    output
}

pub(crate) fn expand_env_vars(value: &Value) -> Value {
    match value {
        Value::String(value) => Value::String(expand_string(value)),
        Value::Object(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), expand_env_vars(value)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(expand_env_vars).collect()),
        Value::Null | Value::Bool(_) | Value::Number(_) => value.clone(),
    }
}

pub(crate) fn normalize_root_model_keys(mut config: Map<String, Value>) -> Map<String, Value> {
    let model_input = config.get("model").cloned();
    let model_has_alias = model_input
        .as_ref()
        .and_then(Value::as_object)
        .map(|model| {
            json_truthy(model.get("api_base"))
                || json_truthy(model.get("model"))
                || json_truthy(model.get("name"))
        })
        .unwrap_or(false);
    let has_root = ["provider", "base_url", "context_length", "api_base"]
        .iter()
        .any(|key| json_truthy(config.get(*key)));
    if !has_root && !model_has_alias {
        return config;
    }

    let mut model = match model_input {
        Some(Value::Object(model)) => model,
        Some(value) if json_truthy(Some(&value)) => {
            let mut model = Map::new();
            model.insert("default".into(), value);
            model
        }
        _ => Map::new(),
    };
    for key in ["provider", "base_url", "context_length"] {
        let root_value = config.get(key).cloned();
        if json_truthy(root_value.as_ref()) && !json_truthy(model.get(key)) {
            model.insert(key.to_owned(), root_value.expect("truthy root value"));
        }
        config.remove(key);
    }

    let root_api_base = config.get("api_base").cloned();
    for alias in [root_api_base, model.get("api_base").cloned()] {
        if json_truthy(alias.as_ref()) && !json_truthy(model.get("base_url")) {
            model.insert("base_url".into(), alias.expect("truthy api_base"));
        }
    }
    config.remove("api_base");
    model.remove("api_base");

    if !json_truthy(model.get("default")) {
        let alias = model
            .get("model")
            .cloned()
            .filter(|value| json_truthy(Some(value)))
            .or_else(|| {
                model
                    .get("name")
                    .cloned()
                    .filter(|value| json_truthy(Some(value)))
            });
        if let Some(alias) = alias {
            model.insert("default".into(), alias);
        }
    }
    if json_truthy(model.get("default")) {
        model.remove("model");
        model.remove("name");
    }
    config.insert("model".into(), Value::Object(model));
    config
}

fn normalize_max_turns_config(mut config: Map<String, Value>) -> Map<String, Value> {
    let mut agent = config
        .get("agent")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let had_root = config.contains_key("max_turns");
    let had_agent = agent.contains_key("max_turns");
    if had_root && !had_agent {
        let value = config.get("max_turns").cloned().expect("root key exists");
        agent.insert("max_turns".into(), value);
    }
    config.insert("agent".into(), Value::Object(agent));
    config.remove("max_turns");
    config
}

fn normalize_user_max_turns(mut config: Map<String, Value>) -> Map<String, Value> {
    if !config.contains_key("max_turns") {
        return config;
    }
    let mut agent = config
        .get("agent")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if agent.get("max_turns").map(Value::is_null).unwrap_or(true) {
        let value = config.get("max_turns").cloned().expect("root key exists");
        agent.insert("max_turns".into(), value);
    }
    config.insert("agent".into(), Value::Object(agent));
    config.remove("max_turns");
    config
}

/// Load defaults plus the process-default user config path.
///
/// `defaults` is caller-supplied because the full CLI default catalog lives
/// above this crate. This function intentionally does not discover managed
/// directories or persist migrations.
pub fn load_merged_config_snapshot(defaults: &Map<String, Value>) -> MergedConfigSnapshot {
    load_merged_config_snapshot_at(&get_config_path(), defaults)
}

/// Load defaults plus a user config at an explicit path.
///
/// This is the unmanaged compatibility entry point. Callers that already
/// resolved a managed-scope overlay should use
/// [`load_merged_config_snapshot_with_overlay_at`].
pub fn load_merged_config_snapshot_at(
    path: &Path,
    defaults: &Map<String, Value>,
) -> MergedConfigSnapshot {
    load_merged_config_snapshot_inner(path, defaults, &Map::new(), None)
}

/// Load defaults plus a user config and an explicit managed overlay.
///
/// The user document is expanded before the overlay, then the overlay is
/// expanded independently and merged last. This preserves the source
/// precedence rule that a user-controlled `${VAR}` cannot shadow a managed
/// literal. The overlay is an input seam: directory discovery, permissions,
/// migrations, and write-back remain owned by the higher CLI layer.
///
/// The cache is valid only while the raw file signature, defaults map, overlay
/// map, and current values of all referenced environment variables are
/// unchanged. A malformed revision reuses the raw loader's last-known-good
/// snapshot before applying the same overlay.
///
/// PARITY: `_load_config_impl` managed merge (upstream `hermes_cli/config.py`
/// lines 3308–3423) layered over the existing merged config normalization.
/// Upstream folds the managed file's `(mtime_ns, size)` into the combined
/// cache signature (lines 3322–3342); this port additionally compares the
/// parsed overlay itself, so a managed edit that changes no value keeps
/// serving the cached snapshot instead of re-merging.
pub fn load_merged_config_snapshot_with_overlay_at(
    path: &Path,
    defaults: &Map<String, Value>,
    managed_overlay: &Map<String, Value>,
) -> MergedConfigSnapshot {
    load_merged_config_snapshot_inner(path, defaults, managed_overlay, None)
}

/// Load defaults, a user config, and the managed `config.yaml` inside an
/// explicit managed-scope directory.
///
/// PARITY: `_load_config_impl` + `managed_scope.load_managed_config()`
/// (upstream `hermes_cli/config.py` lines 3319–3326 and 3413–3423,
/// `hermes_cli/managed_scope.py` lines 118–128). Only `<managed_scope>/config.yaml`
/// participates — there is no drop-in directory — and an absent or malformed
/// managed file contributes no overlay, so the user config survives intact.
pub fn load_merged_config_snapshot_with_managed_scope_at(
    path: &Path,
    defaults: &Map<String, Value>,
    managed_scope: &Path,
) -> MergedConfigSnapshot {
    let overlay = crate::managed_scope::load_managed_config_from(managed_scope);
    load_merged_config_snapshot_inner(path, defaults, &overlay, Some(managed_scope))
}

/// Load defaults plus the process-default user config path and the managed
/// scope resolved by [`crate::managed_scope::get_managed_dir`].
///
/// PARITY: `_load_config_impl`'s `managed_scope.get_managed_dir()` lookup
/// (upstream `hermes_cli/config.py` lines 3322–3326).
pub fn load_merged_config_snapshot_with_managed_scope(
    defaults: &Map<String, Value>,
) -> MergedConfigSnapshot {
    let path = get_config_path();
    match crate::managed_scope::get_managed_dir() {
        Some(managed_dir) => {
            load_merged_config_snapshot_with_managed_scope_at(&path, defaults, &managed_dir)
        }
        None => load_merged_config_snapshot_inner(&path, defaults, &Map::new(), None),
    }
}

fn load_merged_config_snapshot_inner(
    path: &Path,
    defaults: &Map<String, Value>,
    managed_overlay: &Map<String, Value>,
    managed_dir: Option<&Path>,
) -> MergedConfigSnapshot {
    let raw = load_config_snapshot_at(path);
    let signature = raw.signature;
    let path = raw.path.clone();
    let managed_dir = managed_dir.map(Path::to_path_buf);
    let user_config = normalize_user_max_turns(raw.pool_config.clone());
    let mut config = deep_merge(defaults, &user_config);
    config = normalize_max_turns_config(config);
    config = normalize_root_model_keys(config);

    let mut env_snapshot = env_ref_snapshot(&Value::Object(config.clone()));
    env_snapshot.extend(env_ref_snapshot(&Value::Object(managed_overlay.clone())));

    let mut cache = MERGED_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(entry) = cache.get(&path) {
        if entry.signature == signature
            && entry.defaults == *defaults
            && entry.managed_overlay == *managed_overlay
            && entry.managed_dir == managed_dir
            && entry.env_snapshot == env_snapshot
        {
            return entry.snapshot.clone();
        }
    }

    let expanded_user = match expand_env_vars(&Value::Object(config)) {
        Value::Object(config) => config,
        _ => unreachable!("object expansion remains an object"),
    };
    let expanded = match expand_env_vars(&Value::Object(managed_overlay.clone())) {
        Value::Object(managed) => deep_merge(&expanded_user, &managed),
        _ => unreachable!("object expansion remains an object"),
    };
    let snapshot =
        MergedConfigSnapshot::from_config(path.clone(), managed_dir.clone(), signature, expanded);
    cache.insert(
        path,
        MergedCacheEntry {
            signature,
            defaults: defaults.clone(),
            managed_overlay: managed_overlay.clone(),
            managed_dir: managed_dir.clone(),
            env_snapshot,
            snapshot: snapshot.clone(),
        },
    );
    snapshot
}

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

/// Clone-safe effective configuration produced by merging caller defaults with
/// the explicit user file.
///
/// PARITY: `_deep_merge`, `_expand_env_vars`, `_normalize_root_model_keys`,
/// and `_normalize_max_turns_config` (upstream `hermes_cli/config.py`
/// lines 2435–2460, 2486–2591, 2746–2858).
#[derive(Debug, Clone, PartialEq)]
pub struct MergedConfigSnapshot {
    pub path: PathBuf,
    /// The managed-scope directory whose `config.yaml` was folded into this
    /// snapshot, or `None` when no managed overlay was applied.
    ///
    /// PARITY: `managed_scope.get_managed_dir()` / `load_managed_config()`
    /// (upstream `hermes_cli/config.py` lines 3322–3326 and 3420).
    pub managed_dir: Option<PathBuf>,
    pub signature: Option<ConfigSignature>,
    pub pool_config: Map<String, Value>,
    pub model_config: Option<Map<String, Value>>,
    pub custom_providers: Vec<Value>,
}

impl MergedConfigSnapshot {
    fn from_config(
        path: PathBuf,
        managed_dir: Option<PathBuf>,
        signature: Option<ConfigSignature>,
        config: Map<String, Value>,
    ) -> Self {
        let model_config = config.get("model").and_then(Value::as_object).cloned();
        let custom_providers = get_compatible_custom_providers(&config);
        Self {
            path,
            managed_dir,
            signature,
            pool_config: config,
            model_config,
            custom_providers,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the managed-scope directory this snapshot folded, when any.
    pub fn managed_dir(&self) -> Option<&Path> {
        self.managed_dir.as_deref()
    }

    pub fn signature(&self) -> Option<ConfigSignature> {
        self.signature
    }

    pub fn pool_config(&self) -> &Map<String, Value> {
        &self.pool_config
    }

    pub fn model_config(&self) -> Option<&Map<String, Value>> {
        self.model_config.as_ref()
    }

    pub fn custom_providers(&self) -> &[Value] {
        &self.custom_providers
    }
}

#[derive(Debug, Clone)]
struct MergedCacheEntry {
    signature: Option<ConfigSignature>,
    defaults: Map<String, Value>,
    managed_overlay: Map<String, Value>,
    managed_dir: Option<PathBuf>,
    env_snapshot: HashMap<String, Option<String>>,
    snapshot: MergedConfigSnapshot,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    signature: Option<ConfigSignature>,
    snapshot: ConfigSnapshot,
    valid: bool,
}

static MERGED_CACHE: LazyLock<Mutex<HashMap<PathBuf, MergedCacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static SNAPSHOT_CACHE: LazyLock<Mutex<HashMap<PathBuf, CacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn snapshot_cache() -> &'static Mutex<HashMap<PathBuf, CacheEntry>> {
    &SNAPSHOT_CACHE
}

pub(crate) fn file_signature(path: &Path) -> Option<ConfigSignature> {
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

/// Walk nested config mappings by key path, returning the terminal value.
///
/// PARITY: `cfg_get` (upstream `hermes_cli/config.py` lines 2886–2928):
/// a missing `cfg`, non-mapping intermediates, and missing keys fall open to
/// the default; an explicit `null` value is returned as-is because
/// `default` applies only when the key is *absent*.
pub fn cfg_get(config: &Map<String, Value>, keys: &[&str], default: Value) -> Value {
    let mut current: Value = Value::Object(config.clone());
    for key in keys {
        if !current.is_object() {
            return default;
        }
        let Some(map) = current.as_object() else {
            return default;
        };
        match map.get(*key) {
            Some(value) => current = value.clone(),
            None => return default,
        }
    }
    current
}

/// The `openrouter` section of upstream `DEFAULT_CONFIG`
/// (`hermes_cli/config_defaults.py` lines 813-817). Response caching is
/// enabled by default with a 300-second TTL, so installs without a config
/// file still send cache headers. `min_coding_score` is carried for shape
/// fidelity; the header builders do not read it.
pub fn openrouter_defaults() -> Map<String, Value> {
    let mut section = Map::new();
    section.insert("response_cache".into(), Value::Bool(true));
    section.insert(
        "response_cache_ttl".into(),
        Value::Number(serde_json::Number::from(300)),
    );
    section.insert(
        "min_coding_score".into(),
        Value::Number(serde_json::Number::from_f64(0.65).expect("finite")),
    );
    let mut root = Map::new();
    root.insert("openrouter".into(), Value::Object(section));
    root
}
