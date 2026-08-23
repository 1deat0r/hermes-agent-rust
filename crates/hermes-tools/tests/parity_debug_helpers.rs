//! Parity oracles for DebugSession, mirroring upstream
//! tests/tools/test_debug_helpers.py @ b9aa928 — disabled-state half.
//!
//! Evidence tier: unit (env-var gated; the env-dependent enabled-state cases
//! live in parity_debug_helpers_env.rs so no process-sharing test can race
//! the `FAKE_DEBUG_VAR_XYZ` env read).

use hermes_tools::debug_helpers::DebugSession;
use serde_json::Value;

// TestDebugSessionDisabled.test_not_active_by_default
#[test]
fn not_active_by_default() {
    let ds = DebugSession::new("test_tool", "FAKE_DEBUG_VAR_XYZ");
    assert!(!ds.active());
    assert!(!ds.enabled());
    assert_eq!(ds.session_id(), "");
}

// TestDebugSessionDisabled.test_get_session_info_disabled
#[test]
fn get_session_info_disabled() {
    let ds = DebugSession::new("test_tool", "FAKE_DEBUG_VAR_XYZ");
    let info = ds.get_session_info();
    assert_eq!(info["enabled"], Value::Bool(false));
    assert_eq!(info["session_id"], Value::Null);
    assert_eq!(info["log_path"], Value::Null);
    assert_eq!(info["total_calls"], Value::from(0));
}

// Disabled sessions are cheap no-ops (log_call/save do nothing).
#[test]
fn disabled_methods_are_noops() {
    let mut ds = DebugSession::new("test_tool", "FAKE_DEBUG_VAR_XYZ");
    ds.log_call("web_search", serde_json::json!({"query": "q"}));
    ds.save(); // must not create any file
    let info = ds.get_session_info();
    assert_eq!(info["total_calls"], Value::from(0));
}
