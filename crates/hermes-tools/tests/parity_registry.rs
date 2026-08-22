//! Parity oracles for the tool registry, mirroring upstream
//! tests/tools/test_registry.py (register/dispatch, OpenAI-format
//! definitions, shared check_fn once-per-call, check_fn availability,
//! handler-exception error json, plugin-override gates, toolset query
//! surfaces) @ b9aa928.

use std::collections::HashSet;
use std::sync::Arc;

use hermes_tools::registry::{
    registry, tool_error, tool_result, CheckFn, ToolHandler, ToolRegistry, ToolResult,
};
use hermes_tools::{ToolRegistry as _ToolRegistry};
use serde_json::{json, Value};

fn make_schema(name: &str) -> Value {
    json!({
        "name": name,
        "description": format!("A {name}"),
        "parameters": {"type": "object", "properties": {}},
    })
}

struct DummyHandler {
    ok: Value,
}

impl DummyHandler {
    fn plain() -> Self {
        DummyHandler { ok: json!({"ok": true}) }
    }
}

impl ToolHandler for DummyHandler {
    fn call(&self, _: Value, _: Option<&str>, _: Option<&str>) -> ToolResult {
        ToolResult::Text(serde_json::to_string(&self.ok).expect("json"))
    }
}

struct PanicHandler;
impl ToolHandler for PanicHandler {
    fn call(&self, _: Value, _: Option<&str>, _: Option<&str>) -> ToolResult {
        panic!("boom")
    }
}

struct BoolCheck(pub bool);
impl CheckFn for BoolCheck {
    fn check(&self) -> bool {
        self.0
    }
}

fn register(t: &ToolRegistry, name: &str, toolset: &str, handler: Arc<dyn ToolHandler>) {
    t.register(
        name, toolset, make_schema(name), handler, None, None, vec![], None, None, None, None,
        None, false,
    )
    .expect("register");
}

#[test]
fn register_and_dispatch() {
    let t = _ToolRegistry::new();
    register(&t, "alpha", "core", Arc::new(DummyHandler::plain()));
    let result = t.dispatch("alpha", json!({}), None, None);
    // Dispatch returns the handler's raw JSON string (upstream parity).
    assert_eq!(result.as_str().map(|s| serde_json::from_str::<Value>(s).expect("parse")),
               Some(json!({"ok": true})));
}

#[test]
fn unknown_tool_returns_error_json() {
    let t = _ToolRegistry::new();
    let result = t.dispatch("nonexistent", json!({}), None, None);
    assert!(result["error"].as_str().unwrap().contains("Unknown tool"));
}

#[test]
fn definitions_are_openai_format() {
    let t = _ToolRegistry::new();
    register(&t, "t1", "s1", Arc::new(DummyHandler::plain()));
    register(&t, "t2", "s1", Arc::new(DummyHandler::plain()));
    let defs = t.get_definitions(&HashSet::from(["t1".to_string(), "t2".to_string()]), false);
    assert_eq!(defs.len(), 2);
    assert!(defs.iter().all(|d| d["type"] == json!("function")));
    let names: HashSet<&str> = defs
        .iter()
        .filter_map(|d| d["function"]["name"].as_str())
        .collect();
    assert_eq!(names, HashSet::from(["t1", "t2"]));
}

#[test]
fn shared_check_fn_probed_once_per_call() {
    let t = _ToolRegistry::new();
    let probe = Arc::new(BoolCheck(true));
    t.register(
        "first", "shared", make_schema("first"), Arc::new(DummyHandler::plain()),
        Some(probe.clone()), Some("shared_check"), vec![], None, None, None, None, None, false,
    )
    .expect("r1");
    t.register(
        "second", "shared", make_schema("second"), Arc::new(DummyHandler::plain()),
        Some(probe.clone()), Some("shared_check"), vec![], None, None, None, None, None, false,
    )
    .expect("r2");
    let defs = t.get_definitions(&HashSet::from(["first".to_string(), "second".to_string()]), false);
    assert_eq!(defs.len(), 2);
}

#[test]
fn check_fn_controls_availability() {
    let t = _ToolRegistry::new();
    // No check_fn -> available.
    t.register(
        "free", "free_ts", make_schema("free"), Arc::new(DummyHandler::plain()), None, None,
        vec![], None, None, None, None, None, false,
    )
    .expect("r");
    assert!(t.is_toolset_available("free_ts"));
    // check_fn=false -> unavailable.
    t.register(
        "locked", "locked_ts", make_schema("locked"), Arc::new(DummyHandler::plain()),
        Some(Arc::new(BoolCheck(false))), Some("lock"), vec![], None, None, None, None, None,
        false,
    )
    .expect("r2");
    assert!(!t.is_toolset_available("locked_ts"));
    // Definitions skip the gated tool.
    let defs = t.get_definitions(&HashSet::from(["locked".to_string()]), false);
    assert!(defs.is_empty());
}

#[test]
fn handler_panic_returns_error_json() {
    let t = _ToolRegistry::new();
    t.register(
        "bad", "s", make_schema("bad"), Arc::new(PanicHandler), None, None, vec![], None, None,
        None, None, None, false,
    )
    .expect("r");
    let result = t.dispatch("bad", json!({}), None, None);
    assert!(result["error"].as_str().is_some());
}

#[test]
fn cross_toolset_shadow_rejected() {
    let t = _ToolRegistry::new();
    register(&t, "dup", "one", Arc::new(DummyHandler::plain()));
    // Same-name re-registration under a different toolset without override
    // is silently rejected (returns Ok, keeps original).
    t.register(
        "dup", "two", make_schema("dup"), Arc::new(DummyHandler::plain()), None, None, vec![],
        None, None, None, None, None, false,
    )
    .expect("r2");
    assert_eq!(t.get_toolset_for_tool("dup").as_deref(), Some("one"));
    // Cross-toolset with override=True replaces.
    t.register(
        "dup", "two", make_schema("dup"), Arc::new(DummyHandler::plain()), None, None, vec![],
        None, None, None, None, None, true,
    )
    .expect("r3");
    assert_eq!(t.get_toolset_for_tool("dup").as_deref(), Some("two"));
}

#[test]
fn plugin_override_gate_blocks_unowned_deregister() {
    let t = _ToolRegistry::new();
    t.register(
        "builtin", "core", make_schema("builtin"), Arc::new(DummyHandler::plain()), None, None,
        vec![], None, None, None, None, Some("hermes_plugins.pkg.handlers".to_string()), false,
    )
    .expect("r");
    // Another plugin module cannot deregister without opt-in.
    let err = t
        .deregister("builtin", Some("hermes_plugins.other.handlers"))
        .expect_err("blocked");
    assert!(err.contains("cannot deregister tool"));
    // Owner root module can.
    t.deregister("builtin", Some("hermes_plugins.pkg.cleanup")).expect("owner okay");
    assert!(t.get_entry("builtin").is_none());
    // Opt-in policy unblocks cross-plugin removal.
    t.register(
        "builtin2", "core", make_schema("builtin2"), Arc::new(DummyHandler::plain()), None, None,
        vec![], None, None, None, None, Some("hermes_plugins.pkg.handlers".to_string()), false,
    )
    .expect("r2");
    t.register_plugin_override_policy("hermes_plugins.other", true);
    t.deregister("builtin2", Some("hermes_plugins.other.handlers")).expect("opted in");
}

#[test]
fn toolset_query_surfaces() {
    let t = _ToolRegistry::new();
    t.register(
        "echo", "web", make_schema("echo"), Arc::new(DummyHandler::plain()), None, None,
        vec!["KEY".to_string()], None, Some("🔁".into()), Some(42), None,
        None, false,
    )
    .expect("r");
    assert_eq!(t.get_all_tool_names(), vec!["echo".to_string()]);
    assert_eq!(t.get_toolset_for_tool("echo").as_deref(), Some("web"));
    assert_eq!(t.get_emoji("echo", "⚡"), "🔁");
    assert_eq!(t.get_emoji("nope", "⚡"), "⚡");
    assert_eq!(t.get_max_result_size("echo", None), 42);
    assert_eq!(t.get_max_result_size("nope", Some(7)), 7);
    let map = t.get_tool_to_toolset_map();
    assert_eq!(map.get("echo").map(|s| s.as_str()), Some("web"));
    let reqs = t.get_toolset_requirements();
    let envs = reqs["web"]["env_vars"].as_array().unwrap();
    assert_eq!(envs, &vec![json!("KEY")]);
    let (available, _) = t.check_tool_availability(false);
    assert!(available.contains(&"web".to_string()));
}

#[test]
fn aliases_and_generation_surfaces() {
    let t = _ToolRegistry::new();
    t.register_toolset_alias("mcp-alias", "canonical");
    assert_eq!(t.get_toolset_alias_target("mcp-alias").as_deref(), Some("canonical"));
    let aliases = t.get_registered_toolset_aliases();
    assert_eq!(aliases.get("mcp-alias").map(|s| s.as_str()), Some("canonical"));
}

#[test]
fn tool_error_and_tool_result_helpers() {
    assert_eq!(tool_error("file not found", &[]), r#"{"error":"file not found"}"#);
    assert_eq!(
        tool_error("bad input", &[("success".to_string(), json!(false))]),
        r#"{"error":"bad input","success":false}"#
    );
    assert_eq!(tool_result(json!({"ok": 1})), r#"{"ok":1}"#);
}

#[test]
fn singleton_registry_roundtrip() {
    let reg = registry();
    reg.register(
        "singleton_probe", "test", make_schema("singleton_probe"),
        Arc::new(DummyHandler::plain()), None, None, vec![], None, None, None, None, None, false,
    )
    .expect("r");
    let out = registry().dispatch("singleton_probe", json!({}), None, None);
    let parsed: Value = serde_json::from_str(out.as_str().unwrap()).expect("parse");
    assert_eq!(parsed, json!({"ok": true}));
}
