//! Parity oracles for the clarify tool, mirroring upstream
//! tests/tools/test_clarify_tool.py @ b9aa928.

use std::sync::{Arc, Mutex};

use hermes_tools::clarify::{
    clear_clarify_callback, clarify_tool, flatten_choice, parse_multi_select_response,
    register_clarify, set_clarify_callback, MAX_CHOICES, CLARIFY_SCHEMA,
};
use hermes_tools::registry::registry;
use serde_json::{json, Value};

// The callback lives on the current thread; tests that install one must not
// run in parallel on the same thread (Rust test harness threads are distinct,
// so thread-locals are naturally isolated).

fn parse(s: &str) -> Value {
    serde_json::from_str(s).expect("json")
}

#[test]
fn simple_question_with_callback() {
    register_clarify();
    set_clarify_callback(|question: &str, choices: Option<Vec<String>>, _ms: bool| {
        assert_eq!(question, "What color?");
        assert!(choices.is_none());
        "blue".to_string()
    });
    let result = parse(&clarify_tool("What color?", None, false));
    assert_eq!(result["question"], json!("What color?"));
    assert_eq!(result["choices_offered"], Value::Null);
    assert_eq!(result["user_response"], json!("blue"));
    clear_clarify_callback();
}

#[test]
fn no_callback_returns_error() {
    clear_clarify_callback();
    let result = parse(&clarify_tool("What do you want?", None, false));
    assert!(result.get("error").is_some());
    assert!(result["error"].as_str().unwrap().to_lowercase().contains("not available"));
}

#[test]
fn choices_trimmed_to_max() {
    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let seen2 = seen.clone();
    set_clarify_callback(move |_q: &str, choices: Option<Vec<String>>, _ms: bool| {
        if let Some(cs) = choices {
            seen2.lock().unwrap().extend(cs);
        }
        "picked".to_string()
    });
    clarify_tool("Pick one", Some(vec![json!("a"), json!("b"), json!("c"), json!("d"), json!("e"), json!("f"), json!("g")]), false);
    assert_eq!(seen.lock().unwrap().len(), MAX_CHOICES);
    clear_clarify_callback();
}

#[test]
fn choices_converted_to_strings() {
    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let seen2 = seen.clone();
    set_clarify_callback(move |_q: &str, choices: Option<Vec<String>>, _ms: bool| {
        if let Some(cs) = choices {
            seen2.lock().unwrap().extend(cs);
        }
        "answer".to_string()
    });
    clarify_tool("Pick", Some(vec![json!(1), json!(2), json!(3)]), false);
    assert_eq!(*seen.lock().unwrap(), vec!["1".to_string(), "2".to_string(), "3".to_string()]);
    clear_clarify_callback();
}

#[test]
fn callback_exception_returns_error_via_dispatch() {
    // The clarify_tool fn itself holds no catch; dispatch catches panics.
    // For a panicking callback the equivalent observable is a tool error JSON.
    register_clarify();
    set_clarify_callback(|_q: &str, _c: Option<Vec<String>>, _ms: bool| {
        panic!("User cancelled")
    });
    let out = registry().dispatch("clarify", json!({"question": "Question?"}), None, None);
    assert!(out["error"].as_str().is_some());
    clear_clarify_callback();
}

#[test]
fn user_response_stripped() {
    set_clarify_callback(|_q: &str, _c: Option<Vec<String>>, _ms: bool| {
        "  response with spaces  \n".to_string()
    });
    let result = parse(&clarify_tool("Q?", None, false));
    assert_eq!(result["user_response"], json!("response with spaces"));
    clear_clarify_callback();
}

#[test]
fn flatten_unwraps_label_first() {
    assert_eq!(flatten_choice(&json!("plain")), "plain");
    let v = flatten_choice(&json!({"description": "desc only"}));
    assert_eq!(v, "desc only");
    // label first, then description, then text, then title.
    assert_eq!(
        flatten_choice(&json!({"label": "L", "description": "D", "text": "T", "title": "H"})),
        "L"
    );
    assert_eq!(flatten_choice(&json!({"title": "H"})), "H");
    assert_eq!(flatten_choice(&json!({"name": "ignored", "value": "x"})), "");
    assert_eq!(flatten_choice(&Value::Null), "");
}

#[test]
fn multi_select_true_returns_list_and_single_choice_still_list() {
    set_clarify_callback(|_q: &str, _c: Option<Vec<String>>, _ms: bool| {
        "a, b".to_string()
    });
    let result = parse(&clarify_tool("Pick many", Some(vec![json!("a"), json!("b")]), true));
    assert_eq!(result["user_response"], json!(["a", "b"]));
    clear_clarify_callback();
}

#[test]
fn multi_select_single_choice_still_list() {
    set_clarify_callback(|_q: &str, _c: Option<Vec<String>>, _ms: bool| {
        "a".to_string()
    });
    let result = parse(&clarify_tool("Pick", Some(vec![json!("a")]), true));
    assert_eq!(result["user_response"], json!(["a"]));
    clear_clarify_callback();
}

#[test]
fn multi_select_parses_json_array_response() {
    set_clarify_callback(|_q: &str, _c: Option<Vec<String>>, _ms: bool| {
        r#"["x", "y"]"#.to_string()
    });
    let result = parse(&clarify_tool("Pick", Some(vec![json!("x"), json!("y")]), true));
    assert_eq!(result["user_response"], json!(["x", "y"]));
    clear_clarify_callback();
}

#[test]
fn schema_name_and_max_choices() {
    assert_eq!(CLARIFY_SCHEMA["name"], json!("clarify"));
    assert_eq!(MAX_CHOICES, 4);
    assert_eq!(CLARIFY_SCHEMA["parameters"]["properties"]["multi_select"]["type"], json!("boolean"));
    assert_eq!(CLARIFY_SCHEMA["parameters"]["required"], json!(["question"]));
}

#[test]
fn registry_includes_clarify() {
    register_clarify();
    assert_eq!(registry().get_toolset_for_tool("clarify").as_deref(), Some("clarify"));
    assert_eq!(registry().get_emoji("clarify", "⚡"), "❓");
    let defs = registry().get_definitions(&std::collections::HashSet::from(["clarify".to_string()]), false);
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0]["function"]["name"], json!("clarify"));
}

#[test]
fn parse_multi_select_handles_forms() {
    // list input
    assert_eq!(parse_multi_select_response(json!(["a", " b "])), vec!["a".to_string(), "b".to_string()]);
    // json array string
    assert_eq!(parse_multi_select_response(json!("[\"a\",\"b\"]")), vec!["a".to_string(), "b".to_string()]);
    // comma separated
    assert_eq!(parse_multi_select_response(json!("a, b,, c")), vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    assert!(parse_multi_select_response(json!("")).is_empty());
}
