//! Parity oracles for the `read_preview` GUI-surface tool, mirroring
//! upstream tests/tools/test_read_preview_tool.py @ b9aa928.
//! Evidence tier: unit (registry + injected callback seam; no external
//! subsystems).

use std::collections::HashMap;
use std::sync::Arc;

use hermes_tools::read_preview_tool::{
    clear_read_preview_callback, read_preview_tool, register_read_preview,
    set_read_preview_callback, ReadPreviewCallback,
};
use hermes_tools::registry::registry;
use serde_json::{json, Value};

fn parse(s: &str) -> Value {
    serde_json::from_str(s).expect("json output")
}

fn cb<F>(f: F) -> Arc<ReadPreviewCallback>
where
    F: Fn(&HashMap<String, i64>) -> String + Send + Sync + 'static,
{
    Arc::new(f)
}

#[test]
fn lives_in_the_gui_surface_toolset() {
    // Mirrors test_lives_in_the_gui_surface_toolset: scoped by toolset, not
    // by the backend's env.
    register_read_preview();
    let entry = registry()
        .get_entry("read_preview")
        .expect("registered entry");
    assert_eq!(entry.toolset, "desktop_ui");
    assert!(entry.check_fn.is_none());
}

#[test]
fn requires_callback() {
    // Outside the desktop GUI there is no bridge — a clear error, no crash.
    let result = parse(&read_preview_tool(None, None, None));
    assert!(result["error"].as_str().is_some(), "{result}");
    assert!(result["error"].as_str().unwrap().contains("desktop"));
}

#[test]
fn empty_answer_means_nothing_open() {
    let result = parse(&read_preview_tool(None, None, Some(cb(|_| String::new()))));
    assert!(result.get("error").is_some());
}

#[test]
fn passes_json_through() {
    let payload = json!({
        "kind": "url",
        "url": "https://news.ycombinator.com",
        "title": "HN",
        "text": "hello",
    });
    let payload_str = payload.to_string();
    let result = parse(&read_preview_tool(
        None,
        None,
        Some(cb(move |_| payload_str.clone())),
    ));
    assert_eq!(result, payload);
}

#[test]
fn wraps_non_json_text() {
    let result = parse(&read_preview_tool(
        None,
        None,
        Some(cb(|_| "plain words".to_string())),
    ));
    assert_eq!(result, json!({"text": "plain words"}));
}

#[test]
fn window_forwarded_and_validated() {
    let seen: Arc<std::sync::Mutex<HashMap<String, i64>>> =
        Arc::new(std::sync::Mutex::new(HashMap::new()));
    let seen2 = seen.clone();
    let cb = cb(move |window: &HashMap<String, i64>| {
        seen2.lock().unwrap().extend(window.clone());
        json!({"kind": "url"}).to_string()
    });

    let _ = read_preview_tool(Some(json!(100)), Some(json!(500)), Some(cb.clone()));
    assert_eq!(
        *seen.lock().unwrap(),
        HashMap::from([("start".to_string(), 100), ("count".to_string(), 500)])
    );

    // Floors mirror read_terminal: start >= 0, count >= 1.
    seen.lock().unwrap().clear();
    let _ = read_preview_tool(Some(json!(-5)), Some(json!(0)), Some(cb.clone()));
    assert_eq!(
        *seen.lock().unwrap(),
        HashMap::from([("start".to_string(), 0), ("count".to_string(), 1)])
    );

    let result = parse(&read_preview_tool(Some(json!("lots")), None, Some(cb)));
    assert!(result["error"].as_str().unwrap().contains("integers"));
}

#[test]
fn callback_failure_is_reported() {
    let result = parse(&read_preview_tool(
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
fn dispatch_path_uses_thread_local_callback() {
    // The registry handler pulls the callback from the thread-local slot
    // (mirrors Python `kw.get("callback")`).
    register_read_preview();
    set_read_preview_callback(|_window: &HashMap<String, i64>| {
        json!({"text": "from slot"}).to_string()
    });
    let out = registry().dispatch("read_preview", json!({}), None, None);
    let parsed = parse(out.as_str().unwrap_or(""));
    assert_eq!(parsed, json!({"text": "from slot"}));
    clear_read_preview_callback();

    // Without a wired callback the dispatch reports desktop-only.
    let out = registry().dispatch("read_preview", json!({}), None, None);
    let parsed = parse(out.as_str().unwrap_or(""));
    assert!(parsed["error"].as_str().unwrap().contains("desktop"));
}
