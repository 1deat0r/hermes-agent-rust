//! Parity tests for `tools/close_terminal_tool.py` + the
//! `request_close_terminal` contract @ b9aa928. Upstream has no dedicated
//! test file (missing-test gap, noted in the ledger); cases derive from
//! the upstream code as oracle.

use std::sync::{Arc, Mutex};

use serde_json::Value;

use hermes_tools::close_terminal_tool::{
    close_terminal_tool, register_close_terminal, request_close_terminal, set_close_terminal_sink,
};
use hermes_tools::registry::registry;

#[test]
fn lives_in_the_desktop_ui_toolset() {
    register_close_terminal();
    let entry = registry().get_entry("close_terminal").unwrap();
    assert_eq!(entry.toolset, "desktop_ui");
}

#[test]
fn no_sink_yields_the_desktop_only_error() {
    set_close_terminal_sink(None);
    let result = close_terminal_tool("sess-1");
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "error");
    assert_eq!(
        parsed["error"],
        "close_terminal is only available in the Hermes desktop app."
    );
}

#[test]
fn empty_process_id_is_required_error() {
    set_close_terminal_sink(None);
    // The required-process_id error fires BEFORE the sink lookup — the
    // CLI (never-desktop) caller still gets the argument error.
    let result = close_terminal_tool("  ");
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed["error"],
        "process_id is required (the background process whose tab to close)."
    );
    let result = close_terminal_tool("");
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed["error"],
        "process_id is required (the background process whose tab to close)."
    );
}

#[test]
fn successful_close_carries_id_and_reassurance_note() {
    set_close_terminal_sink(None);
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let seen_cb = Arc::clone(&seen);
    set_close_terminal_sink(Some(Arc::new(move |session_id: &str| {
        seen_cb.lock().unwrap().push(session_id.to_string());
        Ok(())
    })));
    let result = close_terminal_tool("sess-9");
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["closed"], "sess-9");
    assert!(parsed["note"]
        .as_str()
        .unwrap()
        .starts_with("Closed the read-only terminal tab"));
    // The id was forwarded verbatim to the sink.
    assert_eq!(
        seen.lock().unwrap().last().map(String::as_str),
        Some("sess-9")
    );
}

#[test]
fn sink_error_string_becomes_the_error_field() {
    set_close_terminal_sink(Some(Arc::new(|_: &str| Err("renderer gone".to_string()))));
    let result = close_terminal_tool("sess-1");
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "error");
    assert_eq!(parsed["error"], "renderer gone");
}

#[test]
fn panicking_sink_is_caught_as_error() {
    set_close_terminal_sink(Some(Arc::new(|_: &str| {
        panic!("websocket write failed");
    })));
    let result = close_terminal_tool("sess-1");
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "error");
    assert_eq!(parsed["error"], "websocket write failed");
}

#[test]
fn request_close_terminal_direct_contract() {
    set_close_terminal_sink(None);
    let parsed = request_close_terminal("any");
    assert_eq!(parsed["status"], "error");
}
