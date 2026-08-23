//! Parity oracles for the desktop renderer-event bridge, mirroring upstream
//! tests/tools/test_desktop_ui.py @ b9aa928.

use hermes_tools::desktop_ui::{available, emit, set_emitter, set_sid_provider};
use serde_json::json;
use std::sync::Mutex;

// The upstream emitter slot is process-global; serialize tests that mutate it.
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn unavailable_without_emitter() {
    let _guard = TEST_LOCK.lock().unwrap();
    set_emitter(None);
    assert!(!available());
    assert!(!emit("preview.open", json!({"url": "x"})));
    set_emitter(None);
}

#[test]
fn routes_event_to_owning_window() {
    let _guard = TEST_LOCK.lock().unwrap();
    set_sid_provider(Some(|| "win-7".to_string()));
    let seen: std::sync::Mutex<Vec<(String, String, serde_json::Value)>> =
        std::sync::Mutex::new(Vec::new());
    // The emitter is a plain fn pointer in the thread-local slot; capture via
    // a leaked Box so the closure has no lifetime issues.
    let captured: &'static std::sync::Mutex<Vec<(String, String, serde_json::Value)>> =
        Box::leak(Box::new(seen));
    set_emitter(Some(Box::new(move |sid, event, payload| {
        captured.lock().unwrap().push((sid, event, payload));
    })));
    assert!(available());
    let ok = emit("pane.reveal", json!({"pane": "terminal"}));
    assert!(ok);
    let v = captured.lock().unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].0, "win-7");
    assert_eq!(v[0].1, "pane.reveal");
    assert_eq!(v[0].2, json!({"pane": "terminal"}));
    set_emitter(None);
    set_sid_provider(None);
}
