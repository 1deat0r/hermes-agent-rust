// Tier: unit/mock — mirrors `tests/hermes_cli/test_lifecycle.py` (two oracle
// tests) plus the fail-open `except Exception` arms of
// `hermes_cli/lifecycle.py`. Upstream monkeypatches the `observability`,
// `plugins`, and `agent.relay_runtime` module objects; the Rust leaf stands in
// for those call-time imports with installable seams (see `lifecycle.rs`).
// The tests share process-global slots, so they serialize on a mutex.

use hermes_cli::lifecycle::{
    finalize_session, has_hook, invoke_hook, set_lifecycle_observer, set_lifecycle_plugins,
    set_relay_coordinator, LifecycleKwargs, LifecycleObserver, LifecyclePlugins, RelayCoordinator,
};
use parking_lot::{Mutex, MutexGuard};
use serde_json::{json, Map, Value};
use std::sync::{Arc, LazyLock};

/// Serialized execution for the process-global seam slots.
fn seam_lock() -> MutexGuard<'static, ()> {
    static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
    LOCK.lock()
}

type Log = Arc<Mutex<Vec<String>>>;

fn kwargs(pairs: &[(&str, Value)]) -> LifecycleKwargs {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect::<Map<String, Value>>()
}

struct FakeObserver {
    log: Log,
    observe_result: Result<(), String>,
    handles: Result<bool, String>,
}

impl LifecycleObserver for FakeObserver {
    fn observe_lifecycle(&self, hook_name: &str, _kwargs: &LifecycleKwargs) -> Result<(), String> {
        self.log.lock().push(format!("builtin:{hook_name}"));
        self.observe_result.clone()
    }
    fn handles_hook(&self, _hook_name: &str) -> Result<bool, String> {
        self.handles.clone()
    }
}

struct FakePlugins {
    log: Log,
    results: Vec<Value>,
    has: bool,
    has_calls: Arc<Mutex<usize>>,
}

impl LifecyclePlugins for FakePlugins {
    fn invoke_hook(&self, hook_name: &str, _kwargs: &LifecycleKwargs) -> Vec<Value> {
        self.log.lock().push(format!("plugin:{hook_name}"));
        self.results.clone()
    }
    fn has_hook(&self, _hook_name: &str) -> bool {
        *self.has_calls.lock() += 1;
        self.has
    }
}

struct FakeRelay {
    log: Log,
    profile: &'static str,
    result: Result<(), String>,
}

impl RelayCoordinator for FakeRelay {
    fn current_profile_key(&self) -> Result<String, String> {
        Ok(self.profile.to_string())
    }
    fn finalize_conversation(&self, profile_key: &str, session_id: &str) -> Result<(), String> {
        self.log
            .lock()
            .push(format!("core:{profile_key}:{session_id}"));
        self.result.clone()
    }
}

fn observer(
    log: &Log,
    observe_result: Result<(), String>,
    handles: Result<bool, String>,
) -> Arc<dyn LifecycleObserver> {
    Arc::new(FakeObserver {
        log: log.clone(),
        observe_result,
        handles,
    })
}

fn plugins(
    log: &Log,
    results: Vec<Value>,
    has: bool,
) -> (Arc<dyn LifecyclePlugins>, Arc<Mutex<usize>>) {
    let has_calls = Arc::new(Mutex::new(0));
    (
        Arc::new(FakePlugins {
            log: log.clone(),
            results,
            has,
            has_calls: has_calls.clone(),
        }),
        has_calls,
    )
}

fn relay(log: &Log, result: Result<(), String>) -> Arc<dyn RelayCoordinator> {
    Arc::new(FakeRelay {
        log: log.clone(),
        profile: "profile-1",
        result,
    })
}

fn entries(log: &Log) -> Vec<String> {
    log.lock().clone()
}

// Oracle: test_invoke_hook_notifies_builtin_observers_before_plugins.
#[test]
fn invoke_hook_notifies_builtin_observers_before_plugins() {
    let _guard = seam_lock();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    set_lifecycle_observer(Some(observer(&log, Ok(()), Ok(false))));
    let (manager, _) = plugins(&log, vec![json!("ok")], false);
    set_lifecycle_plugins(Some(manager));

    let result = invoke_hook(
        "on_session_start",
        &kwargs(&[("session_id", json!("session-1"))]),
    );

    assert_eq!(result, vec![json!("ok")]);
    assert_eq!(
        entries(&log),
        vec!["builtin:on_session_start", "plugin:on_session_start"]
    );

    set_lifecycle_observer(None);
    set_lifecycle_plugins(None);
}

// Oracle: test_finalize_session_closes_core_before_plugin_export.
#[test]
fn finalize_session_closes_core_before_plugin_export() {
    let _guard = seam_lock();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    set_lifecycle_observer(Some(observer(&log, Ok(()), Ok(false))));
    let (manager, _) = plugins(&log, Vec::new(), false);
    set_lifecycle_plugins(Some(manager));
    set_relay_coordinator(Some(relay(&log, Ok(()))));

    finalize_session(&kwargs(&[
        ("session_id", json!("session-1")),
        ("platform", json!("cli")),
    ]));

    assert_eq!(
        entries(&log),
        vec![
            "builtin:on_session_finalize",
            "core:profile-1:session-1",
            "plugin:on_session_finalize",
        ]
    );

    set_lifecycle_observer(None);
    set_lifecycle_plugins(None);
    set_relay_coordinator(None);
}

// Oracle: test_plugin_only_dispatch_does_not_reenter_builtin_observers.
#[test]
fn plugin_seam_dispatch_does_not_reenter_builtin_observers() {
    let _guard = seam_lock();
    let observer_log: Log = Arc::new(Mutex::new(Vec::new()));
    let plugin_log: Log = Arc::new(Mutex::new(Vec::new()));
    set_lifecycle_observer(Some(observer(&observer_log, Ok(()), Ok(false))));
    let (manager, _) = plugins(
        &plugin_log,
        vec![json!("custom"), json!({"value": 1})],
        false,
    );
    set_lifecycle_plugins(Some(manager.clone()));

    // Dispatch through the plugin seam itself, like upstream calls
    // `plugins.invoke_hook` directly.
    let results =
        LifecyclePlugins::invoke_hook(&*manager, "custom", &kwargs(&[("value", json!(1))]));

    assert_eq!(results, vec![json!("custom"), json!({"value": 1})]);
    // Upstream patches observe_lifecycle to raise AssertionError("unexpected");
    // the equivalent channel here is the observer's log staying empty.
    assert!(entries(&observer_log).is_empty());

    set_lifecycle_observer(None);
    set_lifecycle_plugins(None);
}

// Upstream `from hermes_cli.observability import ...` raising ImportError is
// the source's missing-module arm; here the slot is simply uninstalled.
#[test]
fn missing_observer_still_dispatches_plugins() {
    let _guard = seam_lock();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let (manager, _) = plugins(&log, vec![json!(1)], false);
    set_lifecycle_plugins(Some(manager));

    let result = invoke_hook("on_session_start", &kwargs(&[]));

    assert_eq!(result, vec![json!(1)]);
    assert_eq!(entries(&log), vec!["plugin:on_session_start"]);

    set_lifecycle_plugins(None);
}

// The `except Exception: logger.warning(...)` arm around observe_lifecycle.
#[test]
fn builtin_observer_failure_is_fail_open() {
    let _guard = seam_lock();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    set_lifecycle_observer(Some(observer(
        &log,
        Err("observability exploded".to_string()),
        Ok(false),
    )));
    let (manager, _) = plugins(&log, vec![json!("ok")], false);
    set_lifecycle_plugins(Some(manager));

    let result = invoke_hook("on_session_start", &kwargs(&[]));

    assert_eq!(result, vec![json!("ok")]);
    assert_eq!(
        entries(&log),
        vec!["builtin:on_session_start", "plugin:on_session_start"]
    );

    set_lifecycle_observer(None);
    set_lifecycle_plugins(None);
}

#[test]
fn has_hook_prefers_a_builtin_observer_that_handles_the_hook() {
    let _guard = seam_lock();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    set_lifecycle_observer(Some(observer(&log, Ok(()), Ok(true))));
    let (manager, has_calls) = plugins(&log, Vec::new(), false);
    set_lifecycle_plugins(Some(manager));

    assert!(has_hook("on_session_start"));
    assert_eq!(*has_calls.lock(), 0);

    set_lifecycle_observer(None);
    set_lifecycle_plugins(None);
}

#[test]
fn has_hook_falls_through_to_plugins_when_the_observer_passes() {
    let _guard = seam_lock();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    set_lifecycle_observer(Some(observer(&log, Ok(()), Ok(false))));
    let (manager, _) = plugins(&log, Vec::new(), true);
    set_lifecycle_plugins(Some(manager));

    assert!(has_hook("on_session_start"));

    set_lifecycle_observer(None);
    set_lifecycle_plugins(None);
}

// The `except Exception` arm around the observability inspection.
#[test]
fn has_hook_survives_builtin_inspection_failure() {
    let _guard = seam_lock();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    set_lifecycle_observer(Some(observer(
        &log,
        Ok(()),
        Err("cannot inspect".to_string()),
    )));
    let (manager, _) = plugins(&log, Vec::new(), false);
    set_lifecycle_plugins(Some(manager));

    assert!(!has_hook("on_session_start"));

    set_lifecycle_observer(None);
    set_lifecycle_plugins(None);
}

#[test]
fn has_hook_with_no_seams_installed_is_false() {
    let _guard = seam_lock();
    assert!(!has_hook("on_session_start"));
}

// `str(kwargs.get("session_id") or "")`: missing, empty, and Python-falsy
// values close no conversation; truthy non-strings stringify.
#[test]
fn finalize_session_requires_a_truthy_session_id() {
    let _guard = seam_lock();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let (manager, _) = plugins(&log, Vec::new(), false);
    set_lifecycle_plugins(Some(manager));
    set_relay_coordinator(Some(relay(&log, Ok(()))));

    for session_id in [None, Some(json!("")), Some(json!(0)), Some(json!(false))] {
        let mut call = Vec::new();
        if let Some(v) = session_id {
            call.push(("session_id", v));
        }
        finalize_session(&kwargs(&call));
    }

    let core_calls: Vec<_> = entries(&log)
        .into_iter()
        .filter(|e| e.starts_with("core:"))
        .collect();
    assert!(core_calls.is_empty());
    // The plugin export still runs for every finalize.
    assert_eq!(
        entries(&log)
            .iter()
            .filter(|e| e.starts_with("plugin:"))
            .count(),
        4
    );

    // A truthy non-string id stringifies, like `str(5 or "")`.
    finalize_session(&kwargs(&[("session_id", json!(5))]));
    assert_eq!(
        entries(&log)
            .iter()
            .filter(|e| *e == "core:profile-1:5")
            .count(),
        1
    );

    set_lifecycle_plugins(None);
    set_relay_coordinator(None);
}

#[test]
fn relay_finalization_failure_is_fail_open() {
    let _guard = seam_lock();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let (manager, _) = plugins(&log, vec![json!("exported")], false);
    set_lifecycle_plugins(Some(manager));
    set_relay_coordinator(Some(relay(&log, Err("relay down".to_string()))));

    let result = finalize_session(&kwargs(&[("session_id", json!("session-1"))]));

    assert_eq!(result, vec![json!("exported")]);
    assert_eq!(
        entries(&log),
        vec!["core:profile-1:session-1", "plugin:on_session_finalize"]
    );

    set_lifecycle_plugins(None);
    set_relay_coordinator(None);
}

// No relay coordinator installed mirrors `agent.relay_runtime` being absent
// from the process: the core step vanishes, observer and plugins still run.
#[test]
fn missing_relay_skips_only_the_core_step() {
    let _guard = seam_lock();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    set_lifecycle_observer(Some(observer(&log, Ok(()), Ok(false))));
    let (manager, _) = plugins(&log, Vec::new(), false);
    set_lifecycle_plugins(Some(manager));

    finalize_session(&kwargs(&[("session_id", json!("session-1"))]));

    assert_eq!(
        entries(&log),
        vec!["builtin:on_session_finalize", "plugin:on_session_finalize"]
    );

    set_lifecycle_observer(None);
    set_lifecycle_plugins(None);
}
