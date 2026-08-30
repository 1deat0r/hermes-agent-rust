//! Configured per-provider request and stale timeouts.
//!
//! PARITY: `hermes_cli/timeouts.py` @ b9aa928 (whole module, lines 1-83).
//!
//! Upstream resolves the config itself through
//! `hermes_cli.config.load_config_readonly()`; this crate stays a dependency
//! leaf, so the resolution is layered: [`get_provider_request_timeout`] reads
//! the process config path, [`get_provider_request_timeout_at_path`] reads an
//! explicit file, and [`get_provider_request_timeout_at`] works on an already
//! loaded document. The `providers` section is user-defined and carries no
//! `DEFAULT_CONFIG` keys, so reading the raw file is equivalent to the source's
//! merged read for these two knobs.

use serde_json::{Map, Value};
use std::path::Path;

/// PARITY: `_coerce_timeout` (upstream lines 4-11).
///
/// `float(raw)` accepts ints, floats, bools (`float(True) == 1.0`) and numeric
/// strings; anything else — including a mapping or list — is a `TypeError`/
/// `ValueError` arm and yields `None`, as does a non-positive result.
pub fn coerce_timeout(raw: Option<&Value>) -> Option<f64> {
    let value = match raw? {
        Value::Bool(value) => f64::from(*value),
        Value::Number(number) => number.as_f64()?,
        Value::String(text) => text.trim().parse::<f64>().ok()?,
        _ => return None,
    };
    (value > 0.0).then_some(value)
}

/// PARITY: `_get_model_config` (upstream lines 71-83): no model means no
/// per-model section, and a non-mapping `models` or per-model value degrades to
/// "no model config" so the provider knob applies.
fn model_config<'a>(
    provider_config: &'a Map<String, Value>,
    model: Option<&str>,
) -> Option<&'a Map<String, Value>> {
    let model = model.filter(|model| !model.is_empty())?;
    match provider_config.get("models") {
        Some(Value::Object(models)) => match models.get(model) {
            Some(Value::Object(model_config)) => Some(model_config),
            // `model_config.get(model, {})` on a mapping `models` yields {} for
            // an absent key and the raw value otherwise; a non-mapping value
            // fails the isinstance guard and returns None.
            Some(_) => None,
            None => Some(empty_map()),
        },
        None => Some(empty_map()),
        Some(_) => None,
    }
}

/// A shared empty mapping so the navigation helpers can return borrowed
/// "no section" values without allocating.
fn empty_map() -> &'static Map<String, Value> {
    static EMPTY: std::sync::LazyLock<Map<String, Value>> = std::sync::LazyLock::new(Map::new);
    &EMPTY
}

/// PARITY: the shared `providers.<id>` navigation in both getters (upstream
/// lines 23-29 and 50-56): a non-mapping root or `providers` section behaves as
/// `{}`, while a non-mapping provider entry aborts to `None`.
fn provider_config<'a>(
    config: &'a Map<String, Value>,
    provider_id: &str,
) -> Option<&'a Map<String, Value>> {
    if provider_id.is_empty() {
        return None;
    }
    let providers = match config.get("providers") {
        Some(Value::Object(providers)) => providers,
        None => empty_map(),
        Some(_) => empty_map(),
    };
    match providers.get(provider_id) {
        Some(Value::Object(provider)) => Some(provider),
        Some(_) => None,
        None => Some(empty_map()),
    }
}

/// Return a configured provider request timeout in seconds, if any.
///
/// PARITY: `get_provider_request_timeout` (upstream lines 14-44), resolving the
/// config from the process Hermes home.
pub fn get_provider_request_timeout(provider_id: &str, model: Option<&str>) -> Option<f64> {
    get_provider_request_timeout_at_path(&hermes_constants::get_config_path(), provider_id, model)
}

/// Explicit-file form of [`get_provider_request_timeout`]. A missing or
/// malformed file is the source's failed config import: no timeout.
pub fn get_provider_request_timeout_at_path(
    path: &Path,
    provider_id: &str,
    model: Option<&str>,
) -> Option<f64> {
    let config = read_config(path)?;
    get_provider_request_timeout_at(&config, provider_id, model)
}

/// Loaded-document form of [`get_provider_request_timeout`].
pub fn get_provider_request_timeout_at(
    config: &Map<String, Value>,
    provider_id: &str,
    model: Option<&str>,
) -> Option<f64> {
    let provider = provider_config(config, provider_id)?;
    if let Some(model_config) = model_config(provider, model) {
        if let Some(timeout) = coerce_timeout(model_config.get("timeout_seconds")) {
            return Some(timeout);
        }
    }
    coerce_timeout(provider.get("request_timeout_seconds"))
}

/// Return a configured non-stream stale timeout in seconds, if any.
///
/// PARITY: `get_provider_stale_timeout` (upstream lines 20-68). Note the
/// source quirk that both levels use the same `stale_timeout_seconds` key name.
pub fn get_provider_stale_timeout(provider_id: &str, model: Option<&str>) -> Option<f64> {
    get_provider_stale_timeout_at_path(&hermes_constants::get_config_path(), provider_id, model)
}

/// Explicit-file form of [`get_provider_stale_timeout`].
pub fn get_provider_stale_timeout_at_path(
    path: &Path,
    provider_id: &str,
    model: Option<&str>,
) -> Option<f64> {
    let config = read_config(path)?;
    get_provider_stale_timeout_at(&config, provider_id, model)
}

/// Loaded-document form of [`get_provider_stale_timeout`].
pub fn get_provider_stale_timeout_at(
    config: &Map<String, Value>,
    provider_id: &str,
    model: Option<&str>,
) -> Option<f64> {
    let provider = provider_config(config, provider_id)?;
    if let Some(model_config) = model_config(provider, model) {
        if let Some(timeout) = coerce_timeout(model_config.get("stale_timeout_seconds")) {
            return Some(timeout);
        }
    }
    coerce_timeout(provider.get("stale_timeout_seconds"))
}

fn read_config(path: &Path) -> Option<Map<String, Value>> {
    let text = std::fs::read_to_string(path).ok()?;
    let yaml = hermes_utils::fast_safe_load(&text).ok()?;
    match serde_json::to_value(yaml).ok()? {
        Value::Object(config) => Some(config),
        _ => Some(Map::new()),
    }
}
