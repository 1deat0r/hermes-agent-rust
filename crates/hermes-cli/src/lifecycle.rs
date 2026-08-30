//! Hermes lifecycle dispatch for first-party observers and plugins.
//!
//! PARITY: `hermes_cli/lifecycle.py` @ b9aa928 (whole module, lines 1-63).
//!
//! Upstream resolves three dependencies at call time — `hermes_cli.
//! observability`, `hermes_cli.plugins`, and `agent.relay_runtime` — inside
//! `try`/`except` blocks that fail open to the next stage. Those modules are
//! not ported yet, so this leaf mirrors the source's call-time imports with
//! installable seams:
//!
//! SEAMS (documented):
//!  * [`LifecycleObserver`] stands in for `observability.observe_lifecycle`
//!    / `handles_hook`. A `Result::Err` return models the exception the
//!    source catches; an uninstalled slot models the source's `ImportError`
//!    arm — the same warning, then fail-open to the next stage (upstream
//!    puts the import inside its `try`).
//!  * [`LifecyclePlugins`] stands in for the `plugins` hook manager.
//!    Upstream does *not* catch exceptions from `plugins.invoke_hook`, so
//!    that seam reports no failure channel — plugin errors propagate. Its
//!    import cannot fail upstream (core module), so an uninstalled slot is
//!    the empty-registry state and returns no results.
//!  * [`RelayCoordinator`] stands in for `relay_runtime.SESSION_COORDINATOR`
//!    and `current_profile_key`. Upstream evaluates both inside one
//!    `try`/`except`, so a `Result::Err` from either method feeds the same
//!    "Core Relay session finalization failed" warning; an uninstalled slot
//!    models the `from agent import relay_runtime` `ImportError` arm.
//!
//! The crate that owns the real implementations (CLI entry point, plugin
//! registry, relay runtime) installs them once at startup with the
//! `set_lifecycle_*` accessors, exactly as upstream's import system would
//! resolve them.

use parking_lot::RwLock;
use serde_json::{Map, Value};
use std::sync::Arc;

/// Keyword arguments carried through a lifecycle dispatch (upstream's
/// `**kwargs`).
pub type LifecycleKwargs = Map<String, Value>;

/// PARITY: `hermes_cli.observability.observe_lifecycle` / `handles_hook`.
pub trait LifecycleObserver: Send + Sync {
    /// Notify the built-in observers of one lifecycle event. `Err` models the
    /// raised exception the source catches.
    fn observe_lifecycle(&self, hook_name: &str, kwargs: &LifecycleKwargs) -> Result<(), String>;
    /// Return whether a built-in observer consumes `hook_name`. `Err` models
    /// an inspection-time exception.
    fn handles_hook(&self, hook_name: &str) -> Result<bool, String>;
}

/// PARITY: `hermes_cli.plugins.invoke_hook` / `has_hook` on the plugin
/// manager.
pub trait LifecyclePlugins: Send + Sync {
    /// Invoke plugin hooks; returns the collected results. Upstream performs
    /// no exception handling here, so there is deliberately no error channel.
    fn invoke_hook(&self, hook_name: &str, kwargs: &LifecycleKwargs) -> Vec<Value>;
    /// Return whether a plugin consumes `hook_name`.
    fn has_hook(&self, hook_name: &str) -> bool;
}

/// PARITY: `agent.relay_runtime.SESSION_COORDINATOR` /
/// `current_profile_key`.
pub trait RelayCoordinator: Send + Sync {
    /// The currently active Relay profile key. `Err` models the resolution
    /// exception upstream catches inside the finalization `try` block.
    fn current_profile_key(&self) -> Result<String, String>;
    /// Hard-close one core-owned Relay conversation. `Err` models the
    /// exception the source catches.
    fn finalize_conversation(&self, profile_key: &str, session_id: &str) -> Result<(), String>;
}

static OBSERVER: RwLock<Option<Arc<dyn LifecycleObserver>>> = RwLock::new(None);
static PLUGINS: RwLock<Option<Arc<dyn LifecyclePlugins>>> = RwLock::new(None);
static RELAY: RwLock<Option<Arc<dyn RelayCoordinator>>> = RwLock::new(None);

/// Install (or clear with `None`) the built-in observability seam.
pub fn set_lifecycle_observer(observer: Option<Arc<dyn LifecycleObserver>>) {
    *OBSERVER.write() = observer;
}

/// Install (or clear with `None`) the plugin hook-manager seam.
pub fn set_lifecycle_plugins(plugins: Option<Arc<dyn LifecyclePlugins>>) {
    *PLUGINS.write() = plugins;
}

/// Install (or clear with `None`) the Relay session-coordinator seam.
pub fn set_relay_coordinator(relay: Option<Arc<dyn RelayCoordinator>>) {
    *RELAY.write() = relay;
}

fn observer() -> Option<Arc<dyn LifecycleObserver>> {
    OBSERVER.read().clone()
}

fn plugins() -> Option<Arc<dyn LifecyclePlugins>> {
    PLUGINS.read().clone()
}

fn relay() -> Option<Arc<dyn RelayCoordinator>> {
    RELAY.read().clone()
}

/// Notify first-party observers, then invoke compatibility plugin hooks.
///
/// PARITY: `invoke_hook` (upstream lines 11-22). The observer stage is
/// fail-open (`except Exception: logger.warning(...)`); the plugin stage is
/// not, matching the source's unguarded `plugins.invoke_hook` call.
pub fn invoke_hook(hook_name: &str, kwargs: &LifecycleKwargs) -> Vec<Value> {
    match observer() {
        Some(builtin) => {
            if let Err(error) = builtin.observe_lifecycle(hook_name, kwargs) {
                log::warn!("Built-in observability hook failed: {error}");
            }
        }
        None => log::warn!("Built-in observability hook failed: observability seam not installed"),
    }
    match plugins() {
        Some(manager) => manager.invoke_hook(hook_name, kwargs),
        None => Vec::new(),
    }
}

/// Return whether a first-party observer or plugin consumes a hook.
///
/// PARITY: `has_hook` (upstream lines 25-37). An observer that handles the
/// hook answers `True` without consulting plugins; an inspection failure is
/// fail-open, falling through to the plugin seam.
pub fn has_hook(hook_name: &str) -> bool {
    let builtin_handles = match observer() {
        Some(builtin) => match builtin.handles_hook(hook_name) {
            Ok(handles) => Some(handles),
            Err(error) => {
                log::warn!("Unable to inspect built-in observability hooks: {error}");
                None
            }
        },
        None => {
            log::warn!(
                "Unable to inspect built-in observability hooks: observability seam not installed"
            );
            None
        }
    };
    if builtin_handles == Some(true) {
        return true;
    }
    plugins().is_some_and(|manager| manager.has_hook(hook_name))
}

/// Notify observers and hard-close one core-owned Relay conversation.
///
/// PARITY: `finalize_session` (upstream lines 40-63). Order is fixed:
/// builtin observer → core Relay finalization → plugin export, each of the
/// first two fail-open. The core step only runs for a truthy session id
/// (`str(kwargs.get("session_id") or "")`).
pub fn finalize_session(kwargs: &LifecycleKwargs) -> Vec<Value> {
    match observer() {
        Some(builtin) => {
            if let Err(error) = builtin.observe_lifecycle("on_session_finalize", kwargs) {
                log::warn!("Built-in observability hook failed: {error}");
            }
        }
        None => log::warn!("Built-in observability hook failed: observability seam not installed"),
    }

    let session_id = session_id_of(kwargs);
    if !session_id.is_empty() {
        match relay() {
            Some(coordinator) => {
                let outcome = match coordinator.current_profile_key() {
                    Ok(profile_key) => coordinator.finalize_conversation(&profile_key, &session_id),
                    Err(error) => Err(error),
                };
                if let Err(error) = outcome {
                    log::warn!("Core Relay session finalization failed: {error}");
                }
            }
            None => log::warn!("Core Relay session finalization failed: relay seam not installed"),
        }
    }

    match plugins() {
        Some(manager) => manager.invoke_hook("on_session_finalize", kwargs),
        None => Vec::new(),
    }
}

/// PARITY: `str(kwargs.get("session_id") or "")` (upstream line 49) —
/// Python's `or` falls to `""` for any falsy value (`None`, `""`, `0`,
/// `False`, empty containers), and `str()` stringifies the survivors.
fn session_id_of(kwargs: &LifecycleKwargs) -> String {
    match kwargs.get("session_id") {
        Some(value) if python_truthy(value) => python_str(value),
        _ => String::new(),
    }
}

/// Python truthiness for the JSON shapes this seam can carry.
fn python_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_none_or(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Python `str()` for scalar JSON shapes. Container shapes render compactly
/// instead of Python's spaced repr — unreachable upstream, where a session id
/// is always a string; floats use the shortest round-trip form except that a
/// zero fraction keeps `.0` like `str(5.0) == "5.0"`.
fn python_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::Null => "None".to_string(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 {
                    format!("{f:.1}")
                } else {
                    format!("{f}")
                }
            } else {
                n.to_string()
            }
        }
        other => other.to_string(),
    }
}
