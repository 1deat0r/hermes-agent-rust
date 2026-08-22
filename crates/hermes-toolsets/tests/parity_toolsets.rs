//! Parity oracles for the toolsets surface, mirroring upstream
//! tests/test_toolsets.py (known/leaf/composite/cycle/runtime-creation,
//! all-toolsets-have-required-keys, hermes_platforms_share_core,
//! include_registry_false static view) and tests/test_toolset_distributions
//! .py @ b9aa928. Registry-dependent assertions (plugin registries, MCP
//! aliases) are deferred with the tools crate and marked as such.

use std::collections::HashSet;

use hermes_toolsets::{
    bundle_non_core_tools, create_custom_toolset, get_all_toolsets, get_distribution,
    get_toolset, get_toolset_info, get_toolset_names, list_distributions,
    resolve_multiple_toolsets, resolve_toolset, sample_toolsets_from_distribution,
    validate_distribution, validate_toolset,
};

fn toolset_of(name: &str) -> serde_json::Value {
    get_toolset(name, false).expect("toolset")
}

#[test]
fn known_toolset() {
    let web = toolset_of("web");
    assert_eq!(web["description"].as_str(), Some("Web research and content extraction tools"));
    assert_eq!(web["tools"], serde_json::json!(["web_search", "web_extract"]));
    assert_eq!(web["includes"], serde_json::json!([]));
}

#[test]
fn leaf_toolset() {
    let vision = toolset_of("vision");
    assert_eq!(vision["tools"], serde_json::json!(["vision_analyze"]));
    assert_eq!(vision["includes"], serde_json::json!([]));
}

#[test]
fn composite_toolset() {
    // Upstream's composite oracle: debugging includes "web".
    let tools = resolve_toolset("debugging", None, false);
    assert!(tools.contains(&"terminal".to_string()));
    assert!(tools.contains(&"web_search".to_string()));
    assert!(tools.contains(&"web_extract".to_string()));
}

#[test]
fn cycle_detection() {
    // Resolving a self-cyclic path terminates with the already-visited guard.
    create_custom_toolset("loop_a", "A", vec![], vec!["loop_b".to_string()]);
    create_custom_toolset("loop_b", "B", vec!["terminal".to_string()], vec!["loop_a".to_string()]);
    let tools = resolve_toolset("loop_a", None, true);
    assert!(tools.contains(&"terminal".to_string()));
}

#[test]
fn combines_and_deduplicates() {
    let combined = resolve_multiple_toolsets(&["web".to_string(), "vision".to_string()]);
    assert!(combined.contains(&"web_search".to_string()));
    assert!(combined.contains(&"vision_analyze".to_string()));
    let set: HashSet<&String> = combined.iter().collect();
    assert_eq!(set.len(), combined.len());
}

#[test]
fn resolve_special_all_alias() {
    let all_tools = resolve_toolset("all", None, true);
    for name in get_toolset_names() {
        for tool in resolve_toolset(&name, None, true) {
            assert!(all_tools.contains(&tool), "{name} -> {tool} missing from all");
        }
    }
}

#[test]
fn unknown_toolset_resolves_to_empty() {
    assert!(resolve_toolset("no_such_toolset", None, true).is_empty());
    assert!(get_toolset("no_such_toolset", true).is_none());
}

#[test]
fn validator_accepts_static_and_aliases() {
    assert!(validate_toolset("web"));
    assert!(validate_toolset("all"));
    assert!(validate_toolset("*"));
    assert!(!validate_toolset("not_a_real_toolset"));
}

#[test]
fn runtime_creation() {
    create_custom_toolset(
        "my_custom",
        "My custom toolset for specific tasks",
        vec!["web_search".to_string()],
        vec!["terminal".to_string(), "vision".to_string()],
    );
    assert!(validate_toolset("my_custom"));
    let tools = resolve_toolset("my_custom", None, true);
    assert!(tools.contains(&"web_search".to_string()));
    assert!(tools.contains(&"terminal".to_string()));
    assert!(tools.contains(&"vision_analyze".to_string()));
    let info = get_toolset_info("my_custom").expect("info");
    assert_eq!(info["name"].as_str(), Some("my_custom"));
    assert_eq!(info["is_composite"].as_bool(), Some(true));
    assert!(info["tool_count"].as_i64().unwrap() >= 3);
}

#[test]
fn all_toolsets_have_required_keys() {
    let all = get_all_toolsets();
    let names: Vec<String> = all.as_object().unwrap().keys().cloned().collect();
    assert!(!names.is_empty());
    for name in names {
        let def = &all[&name];
        assert!(def.get("description").is_some(), "{name} missing description");
        assert!(def.get("tools").and_then(|v| v.as_array()).is_some(), "{name} missing tools");
        assert!(def.get("includes").and_then(|v| v.as_array()).is_some(), "{name} missing includes");
    }
}

#[test]
fn hermes_platforms_share_core_tools() {
    let platform = toolset_of("hermes-telegram");
    assert!(platform["tools"].as_array().unwrap().iter().any(|t| t.as_str() == Some("terminal")));
    let whatsapp = toolset_of("hermes-whatsapp");
    assert!(whatsapp["tools"].as_array().unwrap().iter().any(|t| t.as_str() == Some("web_search")));
}

#[test]
fn bundle_non_core_tools_keeps_core_intact() {
    let delta = bundle_non_core_tools("hermes-gateway");
    let core: HashSet<&str> = hermes_toolsets::data::HERMES_CORE_TOOLS.iter().copied().collect();
    for tool in &delta {
        assert!(!core.contains(tool.as_str()), "{tool} is core, should not be in delta");
    }
    let garbage = bundle_non_core_tools("hermes-unknown_platform");
    for tool in &garbage {
        assert!(!core.contains(tool.as_str()));
    }
}

#[test]
fn all_names_are_sorted_and_unique() {
    let names = get_toolset_names();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    let set: HashSet<&String> = names.iter().collect();
    assert_eq!(set.len(), names.len());
}

// =====================================================================
// distributions
// =====================================================================

#[test]
fn known_distribution() {
    let dist = get_distribution("default").expect("dist");
    assert!(!dist["description"].as_str().unwrap().is_empty());
    let toolsets = dist["toolsets"].as_object().unwrap();
    assert!(toolsets.contains_key("web"));
    assert!(toolsets.contains_key("terminal"));
}

#[test]
fn returns_copy() {
    let dist = get_distribution("default").expect("dist");
    assert!(dist.is_object());
    let mut all = list_distributions();
    all.as_object_mut().unwrap().insert("FAKE".into(), serde_json::json!({}));
    let again = list_distributions();
    assert!(again.get("FAKE").is_none());
}

#[test]
fn valid_and_minimal() {
    assert!(validate_distribution("default"));
    assert!(!validate_distribution("no_such_dist"));
    let list = list_distributions();
    assert!(list.as_object().unwrap().len() >= 5);
}

#[test]
fn sample_returns_high_probability_toolsets() {
    let sample = sample_toolsets_from_distribution("default", Some(&mut || 0.01)).expect("sample");
    assert!(sample.contains(&"web".to_string()));
    assert!(sample.contains(&"terminal".to_string()));
    let sample = sample_toolsets_from_distribution("default", Some(&mut || 0.99)).expect("sample2");
    assert!(!sample.is_empty());
    let err = sample_toolsets_from_distribution("nope", None).expect_err("unknown");
    assert!(err.contains("Unknown distribution"));
}

// =====================================================================
// registry seam integration (hermes-tools registry wired into toolsets)
// =====================================================================

use std::sync::Arc;

struct OkHandler;
impl hermes_tools::registry::ToolHandler for OkHandler {
    fn call(
        &self,
        _: serde_json::Value,
        _: Option<&str>,
        _: Option<&str>,
    ) -> hermes_tools::registry::ToolResult {
        hermes_tools::registry::ToolResult::Text("{}".to_string())
    }
}

fn register_tool(name: &str, toolset: &str) {
    hermes_tools::registry::registry()
        .register(
            name,
            toolset,
            serde_json::json!({"description": name, "parameters": {"type": "object"}}),
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

#[test]
fn registry_tools_merge_into_builtin_toolset() {
    register_tool("plugin_search_x", "web");
    // get_toolset with registry merges sorted union.
    let web = get_toolset("web", true).expect("web");
    let tools = web["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t.as_str() == Some("web_search")));
    assert!(tools.iter().any(|t| t.as_str() == Some("plugin_search_x")));
    // Static view excludes the registry overlay.
    let static_web = get_toolset("web", false).expect("static web");
    let static_tools = static_web["tools"].as_array().unwrap();
    assert!(!static_tools.iter().any(|t| t.as_str() == Some("plugin_search_x")));
}

#[test]
fn registry_alias_validates() {
    hermes_tools::registry::registry().register_toolset_alias("mcp-smart", "web");
    assert!(validate_toolset("mcp-smart"));
}
