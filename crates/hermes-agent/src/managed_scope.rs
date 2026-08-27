//! Managed scope — IT-pushed, user-immutable config and env layer.
//!
//! PARITY: `hermes_cli/managed_scope.py` @ b9aa928 (whole module, lines 1-195).
//!
//! A system-level directory (default `/etc/hermes`, root-owned and not
//! user-writable) supplies `config.yaml` and `.env` values that WIN over the
//! user's `~/.hermes/config.yaml` and `~/.hermes/.env` on a per-leaf-key
//! basis.
//!
//! This is DISTINCT from `hermes_cli.config.is_managed()` / `HERMES_MANAGED`,
//! which is a coarse package-manager write-lock. That lock blocks all
//! mutation; this layer injects specific immutable values. The two are
//! independent and may coexist.
//!
//! v1 enforcement is filesystem permissions only (upstream docstring, lines
//! 15-19); v1 is Linux/POSIX-first and [`get_managed_dir`] is the single seam
//! for adding other-platform locations later.
//!
//! DOCUMENTED DIVERGENCE: upstream logs a warning when a managed file fails
//! to parse and when the overlay application raises. The Rust port keeps the
//! fail-open result but logs through the `log` facade; upstream's
//! `exc_info=True` traceback is not reproduced.

use crate::config::{deep_merge, expand_env_vars, normalize_root_model_keys};
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// POSIX default. Other-platform locations are a deliberate v2 item; when
/// added, they belong ONLY inside [`get_managed_dir`].
///
/// PARITY: `_DEFAULT_MANAGED_DIR` (upstream lines 33).
const DEFAULT_MANAGED_DIR: &str = "/etc/hermes";

/// Managed-file caches, keyed by path with the `(mtime_ns, size)` signature
/// the entry was parsed under.
///
/// PARITY: `_CONFIG_CACHE` / `_ENV_CACHE` / `_CACHE_LOCK` (upstream
/// lines 35-38).
type ManagedCache = Mutex<HashMap<PathBuf, (u128, u64, Value)>>;

fn config_cache() -> &'static ManagedCache {
    static CACHE: OnceLock<ManagedCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn env_cache() -> &'static ManagedCache {
    static CACHE: OnceLock<ManagedCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// True when running inside the upstream Python test suite.
///
/// PARITY: `_under_pytest` (upstream lines 41-50). Used to ignore the system
/// default `/etc/hermes` during tests so a real managed scope on a
/// developer/CI box cannot leak policy into the suite. Tests that exercise
/// managed scope set `HERMES_MANAGED_DIR` explicitly, which is honored by the
/// override tier above this guard.
fn under_pytest() -> bool {
    std::env::var_os("PYTEST_CURRENT_TEST").is_some()
}

/// Resolve the managed-scope directory, or `None` when no scope is present.
///
/// PARITY: `get_managed_dir` (upstream lines 53-74). Resolution, highest
/// priority first:
///   1. `$HERMES_MANAGED_DIR` — deployment/bootstrap path override
///      (IT-only; never persisted to any `.env`), honored only when
///      non-empty (after trimming) AND the directory exists.
///   2. `/etc/hermes` — POSIX default, when it exists and not under pytest.
///
/// A non-existent directory at either tier resolves to `None` (no managed
/// scope), which is the common case and must stay cheap and side-effect-free.
pub fn get_managed_dir() -> Option<PathBuf> {
    let override_dir = std::env::var("HERMES_MANAGED_DIR").unwrap_or_default();
    let override_dir = override_dir.trim();
    if !override_dir.is_empty() {
        let path = PathBuf::from(override_dir);
        return if path.is_dir() { Some(path) } else { None };
    }
    if under_pytest() {
        return None;
    }
    let path = PathBuf::from(DEFAULT_MANAGED_DIR);
    path.is_dir().then_some(path)
}

/// Drop cached managed config/env. For tests and post-edit reloads.
///
/// PARITY: `invalidate_managed_cache` (upstream lines 77-82).
pub fn invalidate_managed_cache() {
    for cache in [config_cache(), env_cache()] {
        cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }
}

/// Shared `(mtime_ns, size)`-keyed read returning an owned parsed value.
///
/// PARITY: `_cached_read` (upstream lines 85-115). Returns `None` when the
/// file is absent or fails to parse (fail-open). A parse failure is logged
/// LOUDLY — the admin needs to know their policy is not being applied — but
/// never raises, so a malformed managed file cannot brick startup.
fn cached_read<F>(path: &Path, cache: &ManagedCache, parse: F) -> Option<Value>
where
    F: FnOnce(&str) -> Result<Value, String>,
{
    let signature = crate::config::file_signature(path)?;
    let path = path.to_path_buf();
    {
        let guard = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((mtime, size, value)) = guard.get(&path) {
            if (*mtime, *size) == signature {
                return Some(value.clone());
            }
        }
    }
    let text = std::fs::read_to_string(&path).ok()?;
    let parsed = match parse(&text) {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "managed scope: failed to parse {}: {error} — IGNORING this managed file. \
                 Admin policy from this file is NOT being applied. Fix and restart.",
                path.display()
            );
            return None;
        }
    };
    cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(path, (signature.0, signature.1, parsed.clone()));
    Some(parsed)
}

fn parse_managed_yaml(text: &str) -> Result<Value, String> {
    let yaml = hermes_utils::fast_safe_load(text).map_err(|error| error.to_string())?;
    let value = serde_json::to_value(yaml).map_err(|error| error.to_string())?;
    // PARITY: `yaml.safe_load(f) or {}` — an empty document is an empty dict.
    if value.is_null() {
        return Ok(Value::Object(Map::new()));
    }
    Ok(value)
}

/// Parsed managed `config.yaml`, or `{}` when absent or malformed (fail-open).
///
/// PARITY: `load_managed_config` (upstream lines 118-128).
pub fn load_managed_config() -> Map<String, Value> {
    let Some(managed_dir) = get_managed_dir() else {
        return Map::new();
    };
    load_managed_config_from(&managed_dir)
}

/// Managed `config.yaml` read for an explicit managed-scope directory.
///
/// This is the directory-parameterized form of [`load_managed_config`]: the
/// higher CLI layer resolves the scope once and passes the same value to
/// every consumer. Same file, same fail-open and cache semantics.
pub fn load_managed_config_from(managed_dir: &Path) -> Map<String, Value> {
    let parsed = cached_read(
        &managed_dir.join("config.yaml"),
        config_cache(),
        parse_managed_yaml,
    );
    match parsed {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

/// Parsed managed `.env` (`KEY=VALUE`), or `{}` when absent (fail-open).
///
/// PARITY: `load_managed_env` (upstream lines 131-137).
pub fn load_managed_env() -> HashMap<String, String> {
    let Some(managed_dir) = get_managed_dir() else {
        return HashMap::new();
    };
    load_managed_env_from(&managed_dir)
}

/// Managed `.env` read for an explicit managed-scope directory.
pub fn load_managed_env_from(managed_dir: &Path) -> HashMap<String, String> {
    let parsed = cached_read(&managed_dir.join(".env"), env_cache(), |text| {
        Ok(parse_env(text))
    });
    match parsed {
        Some(Value::Object(map)) => map
            .iter()
            .map(|(key, value)| {
                (
                    key.clone(),
                    value.as_str().map(str::to_string).unwrap_or_default(),
                )
            })
            .collect(),
        _ => HashMap::new(),
    }
}

/// PARITY: `_parse_env` (upstream lines 178-187): blank lines, comments, and
/// lines without `=` are skipped; the key is everything before the FIRST `=`;
/// both sides are trimmed and one layer of matching quotes is stripped.
fn parse_env(text: &str) -> Value {
    let mut out = Map::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let (key, value) = line.split_once('=').expect("`=` presence checked");
        out.insert(
            key.trim().to_string(),
            Value::String(
                value
                    .trim()
                    .trim_matches(|ch| ch == '"' || ch == '\'')
                    .to_string(),
            ),
        );
    }
    Value::Object(out)
}

/// PARITY: `_flatten_keys` (upstream lines 190-200): nested mappings recurse
/// while they are non-empty; an empty mapping is a leaf.
fn flatten_keys(map: &Map<String, Value>, prefix: &str, out: &mut BTreeSet<String>) {
    for (key, value) in map {
        let dotted = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            Value::Object(inner) if !inner.is_empty() => flatten_keys(inner, &dotted, out),
            _ => {
                out.insert(dotted);
            }
        }
    }
}

/// Dotted leaf keys pinned by the managed config (e.g. `{"model.default"}`).
///
/// PARITY: `managed_config_keys` (upstream lines 203-205).
pub fn managed_config_keys() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    flatten_keys(&load_managed_config(), "", &mut keys);
    keys
}

/// True when the exact dotted config key is pinned by the managed layer.
///
/// PARITY: `is_key_managed` (upstream lines 208-210).
pub fn is_key_managed(dotted_key: &str) -> bool {
    managed_config_keys().contains(dotted_key)
}

/// True when the env var name is pinned by the managed `.env` layer.
///
/// PARITY: `is_env_managed` (upstream lines 213-215).
pub fn is_env_managed(name: &str) -> bool {
    load_managed_env().contains_key(name)
}

/// Overlay administrator-pinned config values on top of an owned dict.
///
/// PARITY: `apply_managed_overlay` (upstream lines 139-176). The single,
/// shared way for a config loader that builds its own dict — rather than
/// going through `hermes_cli.config.load_config` — to honor managed scope:
///
///   * expand the managed config's `${VAR}` refs against the PROCESS env only
///     (never user-config-defined refs), so a user cannot shadow a managed
///     literal via a `${VAR}` they control;
///   * normalize the managed config's root `model` key so a bare
///     `model: x/y` string cannot clobber the dict shape callers expect;
///   * leaf-level deep-merge managed ON TOP, so managed wins per-leaf while
///     sibling keys stay user-controlled.
///
/// Fail-open: returns `config` unchanged when no managed scope is present —
/// managed scope must never break a caller's startup.
pub fn apply_managed_overlay(config: Map<String, Value>) -> Map<String, Value> {
    let managed = load_managed_config();
    if managed.is_empty() {
        return config;
    }
    let expanded = match expand_env_vars(&Value::Object(managed)) {
        Value::Object(map) => normalize_root_model_keys(map),
        _ => return config,
    };
    // A bare `model: x/y` string in the managed file must merge as
    // `model.default` — otherwise `deep_merge` would replace the caller's
    // `model` dict with a string and break every `cfg["model"]["..."]` read.
    // `_normalize_root_model_keys` only promotes the string when there are
    // root provider/base_url keys to migrate, so handle the bare case here.
    let expanded = match expanded.get("model") {
        Some(Value::String(model)) => {
            let mut section = Map::new();
            section.insert("default".into(), Value::String(model.clone()));
            let mut patched = expanded;
            patched.insert("model".into(), Value::Object(section));
            patched
        }
        _ => expanded,
    };
    deep_merge(&config, &expanded)
}
