//! Parity oracles for DebugSession's enabled state, mirroring upstream
//! tests/tools/test_debug_helpers.py @ b9aa928 — enabled-state half.
//!
//! Evidence tier: unit (env-var gated; this whole binary runs the cases
//! sequentially — the env vars are process-global).
//!
//! Mirrors upstream `_make_enabled`: the TEST_DEBUG env var is set while the
//! session is constructed, then `log_dir` is redirected to a temp dir for the
//! save assertions. HERMES_HOME is also redirected to a temp dir so the
//! constructor's `<home>/logs` mkdir never touches a real user home.

use std::fs;

use hermes_tools::debug_helpers::DebugSession;
use serde_json::Value;
use std::sync::Mutex;
use tempfile::TempDir;

// HERMES_HOME and the debug variables are process-global.  The two upstream
// cases are independent under pytest, but Rust integration-test cases in this
// binary run concurrently unless explicitly serialized.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_env<K: AsRef<std::ffi::OsStr>, V: AsRef<std::ffi::OsStr>>(
    key: K,
    value: V,
    f: impl FnOnce(),
) {
    let old = std::env::var(&key).ok();
    std::env::set_var(&key, value);
    f();
    match old {
        Some(v) => std::env::set_var(key, v),
        None => std::env::remove_var(key),
    }
}

#[test]
fn enabled_session_cases() {
    let _guard = ENV_LOCK.lock().unwrap();
    let hermes_home = TempDir::new().expect("tmp home");
    let view_dir = TempDir::new().expect("tmp view");
    // Redirect the whole session to temps so nothing touches a real home.
    with_env("HERMES_HOME", hermes_home.path(), || {
        with_env("TEST_DEBUG", "true", || {
            let mut ds = DebugSession::new("test_tool", "TEST_DEBUG");

            // test_active_when_env_set
            assert!(ds.active());
            assert!(ds.enabled());

            // test_session_id_generated
            assert!(!ds.session_id().is_empty());

            // Constructor created <HERMES_HOME>/logs (upstream
            // `self.log_dir.mkdir(parents=True, exist_ok=True)`).
            let home_logs = hermes_home.path().join("logs");
            assert!(home_logs.is_dir(), "<hermes_home>/logs was not created");

            // get_session_info when enabled.
            let info = ds.get_session_info();
            assert_eq!(info["enabled"], Value::Bool(true));
            assert_eq!(info["session_id"], Value::String(ds.session_id().to_string()));
            assert!(info["log_path"].as_str().unwrap().ends_with(&format!(
                "test_tool_debug_{}.json",
                ds.session_id()
            )));
            assert_eq!(info["total_calls"], Value::from(0));

            // test_save_empty_log (redirect to view dir first, mirroring
            // upstream `ds.log_dir = tmp_path`).
            ds.set_log_dir(view_dir.path());
            ds.save();
            let json_files: Vec<_> = fs::read_dir(view_dir.path())
                .expect("view dir")
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
                .collect();
            assert_eq!(json_files.len(), 1, "expected exactly one JSON log");
            let data: Value = serde_json::from_str(
                &fs::read_to_string(json_files[0].path()).expect("read log"),
            )
            .expect("valid json");
            assert_eq!(data["total_calls"], Value::from(0));
            assert_eq!(data["tool_calls"], Value::Array(vec![]));
            assert_eq!(data["debug_enabled"], Value::Bool(true));
            assert_eq!(
                data["session_id"],
                Value::String(ds.session_id().to_string())
            );

            // Supplementary (module-code oracle; upstream logs a call via
            // log_call and saves it — the save payload contract):
            ds.log_call("web_search", serde_json::json!({"query": "q1", "results": 3}));
            ds.log_call("web_search", serde_json::json!({"query": "q2"}));
            ds.save();
            let data: Value = serde_json::from_str(
                &fs::read_to_string(json_files[0].path()).expect("read log"),
            )
            .expect("valid json");
            assert_eq!(data["total_calls"], Value::from(2));
            let calls = data["tool_calls"].as_array().expect("tool_calls array");
            assert_eq!(calls.len(), 2);
            assert_eq!(calls[0]["tool_name"], Value::String("web_search".into()));
            assert_eq!(calls[0]["query"], Value::String("q1".into()));
            assert_eq!(calls[0]["results"], Value::from(3));
            // Each call entry carries a timestamp + tool_name.
            assert!(calls[0]["timestamp"].as_str().is_some());
            // The second call has no "results" key (call_data per-call).
            assert!(calls[1].get("results").is_none());
            assert_eq!(calls[1]["query"], Value::String("q2".into()));

            // get_session_info reflects recorded calls.
            let info = ds.get_session_info();
            assert_eq!(info["total_calls"], Value::from(2));

            // Session info log path matches the redirected dir.
            assert!(info["log_path"]
                .as_str()
                .unwrap()
                .starts_with(view_dir.path().to_string_lossy().as_ref()));
        });
    });
}

// Case-insensitive "TRUE" enables too (os.getenv(...).lower() == "true").
#[test]
fn env_var_value_is_case_insensitive() {
    let _guard = ENV_LOCK.lock().unwrap();
    let hermes_home = TempDir::new().expect("tmp home");
    with_env("HERMES_HOME", hermes_home.path(), || {
        with_env("TEST_DEBUG_CAPS", "TRUE", || {
            let ds = DebugSession::new("test_tool", "TEST_DEBUG_CAPS");
            assert!(ds.enabled());
            assert!(!ds.session_id().is_empty());
        });
        with_env("TEST_DEBUG_CAPS", "TruE", || {
            let ds = DebugSession::new("test_tool", "TEST_DEBUG_CAPS");
            assert!(ds.enabled());
        });
        with_env("TEST_DEBUG_CAPS", "1", || {
            let ds = DebugSession::new("test_tool", "TEST_DEBUG_CAPS");
            assert!(!ds.enabled(), "only the literal string 'true' enables");
        });
    });
}
