//! Parity oracles for the `read_terminal` GUI-surface tool.
//!
//! NOTE: upstream has NO tests/tools/test_read_terminal_tool.py @ b9aa928
//! (test gap recorded in the ledger).  These tests mirror the structural
//! twin test_read_preview_tool.py and the schema/window semantics of
//! tools/read_terminal_tool.py itself (the upstream code is the oracle).
//! Evidence tier: unit.

use std::collections::HashMap;
use std::sync::Arc;

use hermes_tools::read_terminal_tool::{
    clear_read_terminal_callback, read_terminal_tool, register_read_terminal,
    set_read_terminal_callback, ReadTerminalCallback,
};
use hermes_tools::registry::registry;
use serde_json::{json, Value};

fn parse(s: &str) -> Value {
    serde_json::from_str(s).expect("json output")
}

fn cb<F>(f: F) -> Arc<ReadTerminalCallback>
where
    F: Fn(&HashMap<String, i64>) -> String + Send + Sync + 'static,
{
    Arc::new(f)
}

#[test]
fn lives_in_the_gui_surface_toolset() {
    register_read_terminal();
    let entry = registry()
        .get_entry("read_terminal")
        .expect("registered entry");
    assert_eq!(entry.toolset, "desktop_ui");
    assert!(entry.check_fn.is_none());
}

#[test]
fn requires_callback() {
    let result = parse(&read_terminal_tool(None, None, None));
    let error = result["error"].as_str().expect("error");
    assert!(error.contains("desktop"), "{error}");
}

#[test]
fn empty_answer_means_no_terminal_open() {
    let result = parse(&read_terminal_tool(None, None, Some(cb(|_| String::new()))));
    assert!(result.get("error").is_some());
}

#[test]
fn passes_json_through() {
    let payload = json!({
        "total_lines": 120,
        "start": 0,
        "end": 24,
        "viewport_rows": 24,
        "cursor_row": 3,
        "text": "hermes$ ",
    });
    let payload_str = payload.to_string();
    let result = parse(&read_terminal_tool(
        None,
        None,
        Some(cb(move |_| payload_str.clone())),
    ));
    assert_eq!(result, payload);
}

#[test]
fn wraps_non_json_text() {
    let result = parse(&read_terminal_tool(
        None,
        None,
        Some(cb(|_| "raw buffer".to_string())),
    ));
    assert_eq!(result, json!({"text": "raw buffer"}));
}

#[test]
fn window_forwarded_and_validated() {
    let seen: Arc<std::sync::Mutex<HashMap<String, i64>>> =
        Arc::new(std::sync::Mutex::new(HashMap::new()));
    let seen2 = seen.clone();
    let cb = cb(move |window: &HashMap<String, i64>| {
        seen2.lock().unwrap().extend(window.clone());
        json!({"total_lines": 10}).to_string()
    });

    let _ = read_terminal_tool(Some(json!(5)), Some(json!(20)), Some(cb.clone()));
    assert_eq!(
        *seen.lock().unwrap(),
        HashMap::from([("start_line".to_string(), 5), ("count".to_string(), 20)])
    );

    // Floors mirror read_preview: start_line >= 0, count >= 1.
    seen.lock().unwrap().clear();
    let _ = read_terminal_tool(Some(json!(-3)), Some(json!(0)), Some(cb.clone()));
    assert_eq!(
        *seen.lock().unwrap(),
        HashMap::from([("start_line".to_string(), 0), ("count".to_string(), 1)])
    );

    let result = parse(&read_terminal_tool(Some(json!("boom")), None, Some(cb)));
    assert!(result["error"].as_str().unwrap().contains("integers"));
}

#[test]
fn callback_failure_is_reported() {
    let result = parse(&read_terminal_tool(
        None,
        None,
        Some(cb(|_| -> String { panic!("renderer went away") })),
    ));
    assert!(result["error"].as_str().is_some());
    assert!(result["error"]
        .as_str()
        .unwrap()
        .contains("renderer went away"));
}

#[test]
fn numeric_string_window_values_accepted() {
    // Python `int("42")` semantics: numeric strings coerce.
    let seen: Arc<std::sync::Mutex<HashMap<String, i64>>> =
        Arc::new(std::sync::Mutex::new(HashMap::new()));
    let seen2 = seen.clone();
    let cb = cb(move |window: &HashMap<String, i64>| {
        seen2.lock().unwrap().extend(window.clone());
        String::new()
    });
    let _ = read_terminal_tool(Some(json!("7")), Some(json!("9")), Some(cb));
    assert_eq!(
        *seen.lock().unwrap(),
        HashMap::from([("start_line".to_string(), 7), ("count".to_string(), 9)])
    );
}

#[test]
fn dispatch_path_uses_thread_local_callback() {
    register_read_terminal();
    set_read_terminal_callback(|_window: &HashMap<String, i64>| {
        json!({"total_lines": 3, "text": "hi"}).to_string()
    });
    let out = registry().dispatch("read_terminal", json!({}), None, None);
    let parsed = parse(out.as_str().unwrap_or(""));
    assert_eq!(parsed, json!({"total_lines": 3, "text": "hi"}));
    clear_read_terminal_callback();

    let out = registry().dispatch("read_terminal", json!({}), None, None);
    let parsed = parse(out.as_str().unwrap_or(""));
    assert!(parsed["error"].as_str().unwrap().contains("desktop"));
}
