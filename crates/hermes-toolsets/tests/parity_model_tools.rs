//! Parity oracles for the model_tools surface, mirroring upstream
//! tests/run_agent/test_tool_arg_coercion.py and tests/test_sanitize_tool_
//! error.py @ b9aa928.

use std::collections::HashSet;
use std::sync::Arc;

use hermes_tools::registry::{ToolHandler, ToolResult};
use serde_json::{json, Value};

use hermes_toolsets::model_tools::{
    coerce_tool_args, compute_tool_definitions, get_tool_definitions, last_resolved_tool_names,
    sanitize_tool_error,
};

struct OkHandler;
impl ToolHandler for OkHandler {
    fn call(&self, _: Value, _: Option<&str>, _: Option<&str>) -> ToolResult {
        ToolResult::Text("{}".to_string())
    }
}

fn register(name: &str, toolset: &str, schema: Value) {
    hermes_tools::registry::registry()
        .register(
            name,
            toolset,
            schema,
            Arc::new(OkHandler),
            None,
            None,
            vec![],
            None,
            None,
            None,
            None,
            None,
            false,
        )
        .expect("register");
}

fn make_schema(properties: Value) -> Value {
    json!({
        "name": "test_tool",
        "description": "test",
        "parameters": {"type": "object", "properties": properties},
    })
}

// =====================================================================
// coerce_tool_args
// =====================================================================

#[test]
fn coerces_integer_arg() {
    register("coerce_int", "test", make_schema(json!({"limit": {"type": "integer"}})));
    let args = coerce_tool_args("coerce_int", json!({"limit": "10"}));
    assert_eq!(args["limit"], json!(10));
}

#[test]
fn coerces_number_and_boolean() {
    register(
        "coerce_num_bool",
        "test",
        make_schema(json!({
            "score": {"type": "number"},
            "done": {"type": "boolean"},
        })),
    );
    let args = coerce_tool_args("coerce_num_bool", json!({"score": "3.14", "done": "true"}));
    assert_eq!(args["score"], json!(3.14));
    assert_eq!(args["done"], json!(true));
}

#[test]
fn leaves_already_correct_types() {
    register("coerce_keep", "test", make_schema(json!({"limit": {"type": "integer"}})));
    let args = coerce_tool_args("coerce_keep", json!({"limit": 10}));
    assert_eq!(args["limit"], json!(10));
}

#[test]
fn wraps_bare_string_in_array() {
    register(
        "coerce_wrap",
        "test",
        make_schema(json!({"urls": {"type": "array", "items": {"type": "string"}}})),
    );
    let args = coerce_tool_args("coerce_wrap", json!({"urls": "https://a.com"}));
    assert_eq!(args["urls"], json!(["https://a.com"]));
}

#[test]
fn parses_json_array_string() {
    register(
        "coerce_parse_list",
        "test",
        make_schema(json!({"tags": {"type": "array", "items": {"type": "string"}}})),
    );
    let args = coerce_tool_args("coerce_parse_list", json!({"tags": r#"["a", "b"]"#}));
    assert_eq!(args["tags"], json!(["a", "b"]));
}

#[test]
fn union_type_coerces_first_match() {
    register(
        "coerce_union",
        "test",
        make_schema(json!({"id": {"type": ["integer", "string"]}})),
    );
    let args = coerce_tool_args("coerce_union", json!({"id": "42"}));
    assert_eq!(args["id"], json!(42));
}

#[test]
fn nullable_null_preserved() {
    register(
        "coerce_null",
        "test",
        make_schema(json!({"mode": {"type": ["string", "null"]}})),
    );
    let args = coerce_tool_args("coerce_null", json!({"mode": "null"}));
    assert_eq!(args["mode"], Value::Null);
}

#[test]
fn nested_json_string_elements_normalized() {
    register(
        "coerce_nested",
        "test",
        make_schema(json!({
            "todos": {"type": "array", "items": {"type": "object", "properties": {"id": {"type": "string"}}}}
        })),
    );
    let args = coerce_tool_args(
        "coerce_nested",
        json!({"todos": [r#"{"id": "x"}"#]}),
    );
    assert_eq!(args["todos"][0]["id"], json!("x"));
}

#[test]
fn one_and_zero_not_boolean() {
    register(
        "coerce_zero",
        "test",
        make_schema(json!({"flag": {"type": "boolean"}})),
    );
    let args = coerce_tool_args("coerce_zero", json!({"flag": "1"}));
    assert_eq!(args["flag"], json!("1"));
}

#[test]
fn empty_args_unchanged() {
    let args = coerce_tool_args("coerce_empty", json!({}));
    assert_eq!(args, json!({}));
}

// =====================================================================
// sanitize_tool_error
// =====================================================================

#[test]
fn strips_tool_call_tags() {
    let out = sanitize_tool_error("bad <tool_call>injected</tool_call> happened");
    assert!(!out.contains("<tool_call>"));
    assert!(!out.contains("</tool_call>"));
    assert!(out.contains("bad injected happened"));
}

#[test]
fn strips_role_tags() {
    for tag in ["system", "assistant", "user", "result", "response", "output", "input"] {
        let raw = format!("prefix <{tag}>hi</{tag}> suffix");
        let out = sanitize_tool_error(&raw);
        assert!(!out.contains(&format!("<{tag}>")), "failed to strip <{tag}>");
        assert!(!out.contains(&format!("</{tag}>")), "failed to strip </{tag}>");
    }
}

#[test]
fn unrelated_xml_kept() {
    let out = sanitize_tool_error("Error parsing <ParseError>line 5</ParseError>");
    assert!(out.contains("<ParseError>"));
}

#[test]
fn strips_cdata() {
    let out = sanitize_tool_error("error: <![CDATA[malicious]]> here");
    assert!(!out.contains("<![CDATA["));
    assert!(!out.contains("]]>"));
}

#[test]
fn strips_code_fence_and_truncates() {
    let out = sanitize_tool_error("```json\n{\"x\": 1}");
    assert!(!out.trim_start().starts_with("```"));
    let long = "A".repeat(5000);
    let out = sanitize_tool_error(&long);
    let body = out.strip_prefix("[TOOL_ERROR] ").unwrap();
    assert_eq!(body.chars().count(), 2000);
    assert!(body.ends_with("..."));
    // Prefix wrapper.
    assert!(sanitize_tool_error("oh no").starts_with("[TOOL_ERROR] "));
}

// =====================================================================
// get_tool_definitions
// =====================================================================


fn register_web_tools() {
    register("web_search", "web", make_schema(json!({"q": {"type": "string"}})));
    register("web_extract", "web", make_schema(json!({"url": {"type": "string"}})));
    register("terminal", "terminal", make_schema(json!({"cmd": {"type": "string"}})));
}

#[test]
fn enabled_toolsets_filter_definitions() {
    register_web_tools();

    let defs = get_tool_definitions(
        Some(&["web".to_string()]),
        None,
        true,
        false,
        false, false, false,
    );
    let names: HashSet<&str> = defs
        .iter()
        .filter_map(|d| d.get("function").and_then(|f| f.get("name")).and_then(Value::as_str))
        .collect();
    assert!(names.contains("web_search"));
    assert!(names.contains("web_extract"));
    assert!(!names.contains("terminal"));
    // Last-resolved names updated on compute.
    let last = last_resolved_tool_names();
    assert!(last.contains(&"web_search".to_string()));
}

#[test]
fn disabled_toolset_subtracts() {
    register_web_tools();
    let defs = get_tool_definitions(
        Some(&["web".to_string(), "terminal".to_string()]),
        Some(&["terminal".to_string()]),
        true, false, false, false, false,
    );
    let names: Vec<&str> = defs
        .iter()
        .filter_map(|d| d.get("function").and_then(|f| f.get("name")).and_then(Value::as_str))
        .collect();
    assert!(names.contains(&"web_search"));
    assert!(!names.contains(&"terminal"), "disabled toolset must be subtracted");
}

#[test]
fn disabled_bundle_keeps_core() {
    // Disabling a hermes-* bundle subtracts only its non-core delta, so core
    // tools shared by an enabled toolset survive.
    register_web_tools();
    let defs = get_tool_definitions(
        Some(&["web".to_string()]),
        Some(&["hermes-telegram".to_string()]),
        true, false, false, false, false,
    );
    let names: Vec<&str> = defs
        .iter()
        .filter_map(|d| d.get("function").and_then(|f| f.get("name")).and_then(Value::as_str))
        .collect();
    assert!(names.contains(&"web_search"));
}

#[test]
fn quiet_cache_returns_same_result() {
    register_web_tools();
    let a = get_tool_definitions(Some(&["web".to_string()]), None, true, false, false, false, false);
    let b = get_tool_definitions(Some(&["web".to_string()]), None, true, false, false, false, false);
    assert_eq!(a, b);
}

#[test]
fn web_browser_crossref_stripped_when_web_missing() {
    // The strip fires for a browser tool whose toolset does NOT include web
    // tools (the built-in "browser" toolset lists web_search itself).
    let toolset = "browser_nw";
    register(
        "browser_navigate",
        toolset,
        json!({
            "name": "browser_navigate",
            "description": "Navigate the browser to a URL. For simple information retrieval, prefer web_search or web_extract (faster, cheaper).",
            "parameters": {"type": "object", "properties": {"url": {"type": "string"}}},
        }),
    );

    let defs = compute_tool_definitions(
        Some(&[toolset.to_string()]),
        None,
        true, false, false, false, false,
    );
    let desc = defs
        .iter()
        .find(|d| d.get("function").and_then(|f| f.get("name")).and_then(Value::as_str) == Some("browser_navigate"))
        .map(|d| d.get("function").and_then(|f| f.get("description")).and_then(Value::as_str).unwrap_or(""));
    assert!(desc.is_some());
    assert!(!desc.unwrap().contains("prefer web_search"), "cross-reference must be stripped when web tools unavailable");
}
