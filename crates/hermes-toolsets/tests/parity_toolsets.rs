//! Parity oracles for the toolsets surface, mirroring upstream
//! tests/tools/test_toolsets.py and tests/tools/test_toolset_distributions.py
//! @ 5d59366 (all classes). The distribution-section tests below predate
//! the oracle split and stay green. Registry-dependent cases share the
//! process-global registry under REGISTRY_TEST_LOCK (upstream monkeypatches
//! a fresh ToolRegistry per test; here we serialize + clean up so exact
//! set assertions stay stable across the shared singleton).
//! Tier: `unit`.

use std::collections::HashSet;
use std::sync::Mutex;

/// Serializes tests that read/mutate the process-global tool registry against
/// each other. The registry is a shared OnceLock behind a Mutex in
/// hermes-tools::registry; without this, `registry_tools_merge_into_builtin_toolset`
/// can register `plugin_search_x` while `resolve_special_all_alias` snapshots
/// the "all" view, producing an intermittent "missing from all" failure even
/// though each test is individually correct.
static REGISTRY_TEST_LOCK: Mutex<()> = Mutex::new(());

use hermes_toolsets::{
    bundle_non_core_tools, clear_resolve_toolset_memo, create_custom_toolset, get_all_toolsets,
    get_distribution, get_toolset, get_toolset_info, get_toolset_names, inject_distribution,
    list_distributions, remove_custom_toolset, remove_distribution, resolve_multiple_toolsets,
    resolve_toolset, resolve_toolset_hit, sample_toolsets_from_distribution,
    sample_toolsets_from_distribution_verbose, validate_distribution, validate_toolset,
};

fn toolset_of(name: &str) -> serde_json::Value {
    get_toolset(name, false).expect("toolset")
}

#[test]
fn known_toolset() {
    let web = toolset_of("web");
    assert_eq!(
        web["description"].as_str(),
        Some("Web research and content extraction tools")
    );
    assert_eq!(
        web["tools"],
        serde_json::json!(["web_search", "web_extract"])
    );
    assert_eq!(web["includes"], serde_json::json!([]));
    // ORACLE: `test_known_toolset` uses the default include_registry=True —
    // merged view is the sorted union (membership here; exact-set equality
    // lives in the registry merge test under the registry lock).
    let merged = get_toolset("web", true).expect("merged web");
    let tools = merged["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t.as_str() == Some("web_search")));
    assert!(tools.iter().any(|t| t.as_str() == Some("web_extract")));
    let names: Vec<&str> = tools.iter().filter_map(|t| t.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "merged view must be sorted");
    // ORACLE: static entries carry no `posture` key (only `coding` has one)
    // and only `hermes-yuanbao` carries `module` — upstream `_ts(...)` extra
    // keys / dict-literal keys.
    assert!(web.get("posture").is_none(), "web must not carry posture");
    let yuanbao = toolset_of("hermes-yuanbao");
    assert_eq!(yuanbao["module"].as_str(), Some("tools.yuanbao_tools"));
    let coding = toolset_of("coding");
    assert_eq!(coding["posture"].as_bool(), Some(true));
}

#[test]
fn leaf_toolset() {
    let vision = toolset_of("vision");
    assert_eq!(vision["tools"], serde_json::json!(["vision_analyze"]));
    assert_eq!(vision["includes"], serde_json::json!([]));
}

#[test]
fn composite_toolset() {
    // ORACLE: `test_composite_toolset` + `test_static_view_threads_through_includes`
    // — debugging direct tools [terminal, process_manage] and includes
    // [web, file] all reach the resolution (include_registry=False static view).
    let tools = resolve_toolset("debugging", None, false);
    assert!(tools.contains(&"terminal".to_string()));
    assert!(tools.contains(&"process_manage".to_string()));
    assert!(tools.contains(&"web_search".to_string()));
    assert!(tools.contains(&"web_extract".to_string()));
    assert!(tools.contains(&"read_file".to_string()));
}

#[test]
fn cycle_detection() {
    // ORACLE: `test_cycle_detection` — A includes B, B includes A; the
    // visited guard terminates and both direct tool lists survive.
    // Lock: create/remove mutates the process-global toolset view that
    // resolve_special_all snapshots (shared-registry isolation analog).
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    create_custom_toolset(
        "_cycle_a",
        "test",
        vec!["t1".into()],
        vec!["_cycle_b".into()],
    );
    create_custom_toolset(
        "_cycle_b",
        "test",
        vec!["t2".into()],
        vec!["_cycle_a".into()],
    );
    let tools = resolve_toolset("_cycle_a", None, true);
    assert!(tools.contains(&"t1".to_string()));
    assert!(tools.contains(&"t2".to_string()));
    // ORACLE: tests `del TOOLSETS[...]` in `finally` — mirror via remove.
    remove_custom_toolset("_cycle_a");
    remove_custom_toolset("_cycle_b");
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
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    let all_tools = resolve_toolset("all", None, true);
    for name in get_toolset_names() {
        for tool in resolve_toolset(&name, None, true) {
            assert!(
                all_tools.contains(&tool),
                "{name} -> {tool} missing from all"
            );
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
    // ORACLE: `test_runtime_creation` — custom toolsets join TOOLSETS:
    // validate/resolve/info work, and the name shows up in names + all
    // (upstream inserts into the TOOLSETS dict itself).
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
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
    // Custom entries join get_toolset_names (upstream: TOOLSETS keys).
    assert!(get_toolset_names().iter().any(|n| n == "my_custom"));
    // AND appear in get_all_toolsets with the required keys.
    let all = get_all_toolsets();
    let def = &all["my_custom"];
    assert!(def.get("description").is_some());
    assert!(def.get("tools").and_then(|v| v.as_array()).is_some());
    assert!(def.get("includes").and_then(|v| v.as_array()).is_some());
    // ORACLE: `finally del TOOLSETS["_test_custom"]` — mirror via remove.
    remove_custom_toolset("my_custom");
    assert!(!validate_toolset("my_custom"));
}

#[test]
fn all_toolsets_have_required_keys() {
    let all = get_all_toolsets();
    let names: Vec<String> = all.as_object().unwrap().keys().cloned().collect();
    assert!(!names.is_empty());
    for name in names {
        let def = &all[&name];
        assert!(
            def.get("description").is_some(),
            "{name} missing description"
        );
        assert!(
            def.get("tools").and_then(|v| v.as_array()).is_some(),
            "{name} missing tools"
        );
        assert!(
            def.get("includes").and_then(|v| v.as_array()).is_some(),
            "{name} missing includes"
        );
    }
}

#[test]
fn hermes_platforms_share_core_tools() {
    // ORACLE: `test_hermes_platforms_share_core_tools` — intersection of the
    // seven platform toolsets (static direct tools) must be non-trivial
    // (> 20); platform extras ride on top (subset check is the oracle's).
    let platforms = [
        "hermes-cli",
        "hermes-telegram",
        "hermes-discord",
        "hermes-whatsapp",
        "hermes-slack",
        "hermes-signal",
        "hermes-homeassistant",
    ];
    let tool_sets: Vec<HashSet<String>> = platforms
        .iter()
        .map(|p| {
            toolset_of(p)["tools"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|t| t.as_str().map(str::to_string))
                .collect()
        })
        .collect();
    let mut core = tool_sets[0].clone();
    for set in tool_sets[1..].iter() {
        core = core.intersection(set).cloned().collect();
    }
    for set in &tool_sets {
        for tool in &core {
            assert!(set.contains(tool), "missing core tool {tool}");
        }
    }
    assert!(
        core.len() > 20,
        "suspiciously small shared core: {}",
        core.len()
    );
    // Direct membership spot-checks (oracle's whatsapp web_search case).
    assert!(tool_sets[3].contains("web_search"));
}

#[test]
fn bundle_non_core_tools_keeps_core_intact() {
    let delta = bundle_non_core_tools("hermes-gateway");
    let core: HashSet<&str> = hermes_toolsets::data::HERMES_CORE_TOOLS
        .iter()
        .copied()
        .collect();
    for tool in &delta {
        assert!(
            !core.contains(tool.as_str()),
            "{tool} is core, should not be in delta"
        );
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
    all.as_object_mut()
        .unwrap()
        .insert("FAKE".into(), serde_json::json!({}));
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
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    register_tool("plugin_search_x", "web");
    // ORACLE: `test_merges_registry_tools_into_builtin_toolset` — exact set
    // equality {web_search, web_extract, web_search_plus}. Shared-registry
    // isolation: deregister at the end (upstream monkeypatches a fresh
    // ToolRegistry per test) so exact-set assertions elsewhere stay stable.
    let web = get_toolset("web", true).expect("web");
    let tools: std::collections::BTreeSet<String> = web["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t.as_str().map(str::to_string))
        .collect();
    let expected: std::collections::BTreeSet<String> =
        ["web_search", "web_extract", "plugin_search_x"]
            .iter()
            .map(|s| s.to_string())
            .collect();
    assert_eq!(tools, expected);
    // Static view excludes the registry overlay.
    let static_web = get_toolset("web", false).expect("static web");
    let static_tools = static_web["tools"].as_array().unwrap();
    assert!(!static_tools
        .iter()
        .any(|t| t.as_str() == Some("plugin_search_x")));
    // Cleanup (bumps registry generation — memo keys rotate with it).
    hermes_tools::registry::registry()
        .deregister("plugin_search_x", None)
        .expect("deregister");
}

#[test]
fn registry_alias_validates() {
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    hermes_tools::registry::registry().register_toolset_alias("mcp-smart", "web");
    assert!(validate_toolset("mcp-smart"));
}

// =====================================================================
// Oracle cases from tests/tools/test_toolsets.py @ 5d59366
// =====================================================================

#[test]
fn oracle_x_search_description_points_to_xurl() {
    // ORACLE: `test_x_search_toolset_marks_read_only_and_points_to_xurl`.
    let ts = get_toolset("x_search", true).expect("x_search");
    assert_eq!(ts["tools"], serde_json::json!(["x_search"]));
    let description = ts["description"].as_str().unwrap().to_lowercase();
    assert!(description.contains("read-only"), "{description}");
    assert!(description.contains("xurl"), "{description}");
    assert!(description.contains("authenticated"), "{description}");
}

#[test]
fn oracle_static_and_mcp_alias_same_name_merged() {
    // ORACLE: `test_static_and_mcp_alias_with_same_name_are_merged` — an MCP
    // server named like a built-in registers a bare alias to its
    // `mcp-<name>` toolset; the static entry must union those tools in and
    // keep its own includes. Cleanup mirrors the oracle's finally-block.
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    create_custom_toolset(
        "_mergetest",
        "static",
        vec!["builtin_tool_a".into()],
        vec!["web".into()],
    );
    register_tool("mcp__mergetest_call", "mcp-_mergetest");
    hermes_tools::registry::registry().register_toolset_alias("_mergetest", "mcp-_mergetest");
    let ts = get_toolset("_mergetest", true).expect("_mergetest");
    let tools: std::collections::BTreeSet<String> = ts["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t.as_str().map(str::to_string))
        .collect();
    for expected in ["builtin_tool_a", "mcp__mergetest_call"] {
        assert!(tools.contains(expected), "missing {expected}: {tools:?}");
    }
    assert_eq!(ts["includes"], serde_json::json!(["web"]));
    // Cleanup: deregister drops the alias (its target toolset empties) —
    // then remove the custom dict entry.
    hermes_tools::registry::registry()
        .deregister("mcp__mergetest_call", None)
        .expect("deregister");
    remove_custom_toolset("_mergetest");
}

#[test]
fn oracle_plugin_toolset_uses_registry_snapshot() {
    // ORACLE: `test_plugin_toolset_uses_registry_snapshot` — a registry-only
    // (plugin) toolset resolves to its tools, sorted by name.
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    register_tool("plugin_b", "plugin_example");
    register_tool("plugin_a", "plugin_example");
    assert_eq!(
        resolve_toolset("plugin_example", None, true),
        vec!["plugin_a".to_string(), "plugin_b".to_string()]
    );
    hermes_tools::registry::registry()
        .deregister("plugin_a", None)
        .expect("deregister a");
    hermes_tools::registry::registry()
        .deregister("plugin_b", None)
        .expect("deregister b");
}

#[test]
fn oracle_mcp_alias_validate_and_resolve() {
    // ORACLE: `test_mcp_alias_uses_live_registry` — alias key and canonical
    // registry toolset both validate; resolving the alias yields the tools.
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    register_tool("mcp__dynserver__ping", "mcp-dynserver");
    hermes_tools::registry::registry().register_toolset_alias("dynserver", "mcp-dynserver");
    assert!(validate_toolset("dynserver"));
    assert!(validate_toolset("mcp-dynserver"));
    assert!(resolve_toolset("dynserver", None, true)
        .iter()
        .any(|t| t == "mcp__dynserver__ping"));
    // Cleanup: last tool of mcp-dynserver leaves → its aliases are retained
    // out (registry.deregister drops aliases targeting the empty toolset).
    hermes_tools::registry::registry()
        .deregister("mcp__dynserver__ping", None)
        .expect("deregister");
}

#[test]
fn oracle_get_toolset_info_leaf_and_composite() {
    // ORACLE: `test_leaf` / `test_composite` — leaf is_composite false with
    // the exact resolved count; composite is_composite true with
    // tool_count > direct. `x_search` is used for the leaf (upstream uses
    // `web`) because the shared process registry can carry extras under
    // `web` from sibling tests — see shared-registry isolation note.
    let info = get_toolset_info("x_search").expect("info");
    assert_eq!(info["name"].as_str(), Some("x_search"));
    assert_eq!(info["is_composite"].as_bool(), Some(false));
    assert_eq!(info["tool_count"].as_i64(), Some(1));
    let info = get_toolset_info("debugging").expect("composite info");
    assert_eq!(info["is_composite"].as_bool(), Some(true));
    let direct = info["direct_tools"].as_array().unwrap().len() as i64;
    assert!(info["tool_count"].as_i64().unwrap() > direct);
}

#[test]
fn oracle_get_all_toolsets_includes_plugin_toolset() {
    // ORACLE: `test_get_all_toolsets_includes_plugin_toolset`.
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    register_tool("plugin_tool", "plugin_bundle");
    let all = get_all_toolsets();
    let plugin = &all["plugin_bundle"];
    assert!(
        plugin.get("description").is_some(),
        "plugin_bundle missing from get_all_toolsets"
    );
    assert_eq!(plugin["tools"], serde_json::json!(["plugin_tool"]));
    hermes_tools::registry::registry()
        .deregister("plugin_tool", None)
        .expect("deregister");
}

#[test]
fn oracle_include_registry_false_excludes_registry_tools() {
    // ORACLE: `test_include_registry_false_excludes_registry_tools` — a tool
    // registered into `terminal` at runtime appears only in the merged view;
    // the static view is exactly the dict entry (oracle's
    // {terminal, process_manage}; `discover_builtin_tools` is a seam that
    // loads no builtins here, so the static set is the definition itself).
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    register_tool("__probe_registry_only_tool__", "terminal");
    let merged = resolve_toolset("terminal", None, true);
    let static_view: std::collections::BTreeSet<String> = resolve_toolset("terminal", None, false)
        .into_iter()
        .collect();
    let expected: std::collections::BTreeSet<String> = ["terminal", "process_manage"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(static_view, expected);
    assert!(merged.iter().any(|t| t == "__probe_registry_only_tool__"));
    assert!(!static_view.contains("__probe_registry_only_tool__"));
    hermes_tools::registry::registry()
        .deregister("__probe_registry_only_tool__", None)
        .expect("deregister");
}

#[test]
fn oracle_registry_only_static_view_is_empty() {
    // ORACLE: `test_registry_only_toolset_static_view_is_empty`.
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    register_tool("not_static_probe", "__definitely_not_a_real_toolset__");
    assert_eq!(
        resolve_toolset("__definitely_not_a_real_toolset__", None, false),
        Vec::<String>::new()
    );
    // Sanity: the merged view sees it (registry membership is live).
    assert!(
        resolve_toolset("__definitely_not_a_real_toolset__", None, true)
            .iter()
            .any(|t| t == "not_static_probe")
    );
    hermes_tools::registry::registry()
        .deregister("not_static_probe", None)
        .expect("deregister");
}

#[test]
fn oracle_resolve_memo_second_call_is_hit() {
    // ORACLE: `test_second_resolution_is_cached` — the public entry memoizes;
    // the second resolve is a hit with an identical result.
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    clear_resolve_toolset_memo();
    let (first, hit1) = resolve_toolset_hit("hermes-cli", true);
    let (second, hit2) = resolve_toolset_hit("hermes-cli", true);
    assert!(!hit1, "first resolution is a miss");
    assert!(hit2, "second resolution must be a memo hit");
    assert_eq!(first, second);
    assert!(!first.is_empty());
}

#[test]
fn oracle_resolve_memo_generation_bump_invalidates() {
    // ORACLE: `test_generation_bump_invalidates_memo` — a registry mutation
    // (generation bump) forces a fresh resolve.
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    clear_resolve_toolset_memo();
    let (before, hit1) = resolve_toolset_hit("hermes-cli", true);
    assert!(!hit1);
    // register() bumps the registry generation (registry.rs:339).
    register_tool("__memo_probe__", "__memo_probe_ts__");
    let (after_bump, hit2) = resolve_toolset_hit("hermes-cli", true);
    assert!(!hit2, "generation bump must invalidate the memo");
    let (third, hit3) = resolve_toolset_hit("hermes-cli", true);
    assert!(hit3, "re-resolve after the bump memoizes again");
    assert_eq!(before, after_bump);
    assert_eq!(after_bump, third, "memo never changes the resolved result");
    hermes_tools::registry::registry()
        .deregister("__memo_probe__", None)
        .expect("deregister");
}

#[test]
fn oracle_resolve_memo_include_registry_false_stable() {
    // ORACLE: `test_memo_result_matches_fresh_resolution`.
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    clear_resolve_toolset_memo();
    let (first, hit1) = resolve_toolset_hit("hermes-cli", false);
    let (second, hit2) = resolve_toolset_hit("hermes-cli", false);
    assert!(!hit1 && hit2);
    assert_eq!(first, second);
    assert!(!first.is_empty());
}

// =====================================================================
// Oracle cases from tests/tools/test_toolset_distributions.py @ 5d59366
// (grouped "+"-compound entries — #64503)
// =====================================================================

#[test]
fn oracle_compound_hit_selects_all_members_together() {
    // ORACLE: `test_hit_roll_selects_all_members_together` — every roll hits
    // (rng 0.0); `browser+search` yields both members, never the raw key.
    let (result, warnings) =
        sample_toolsets_from_distribution_verbose("browser_tasks", Some(&mut || 0.0))
            .expect("sample");
    assert!(result.contains(&"browser".to_string()));
    assert!(result.contains(&"search".to_string()));
    assert!(
        !result.iter().any(|t| t == "browser+search"),
        "members, not the raw key: {result:?}"
    );
    assert!(warnings.is_empty(), "all members are valid: {warnings:?}");
}

#[test]
fn oracle_compound_fallback_expands_members() {
    // ORACLE: `test_fallback_expands_compound_members` — every roll misses
    // (rng 1.0); nothing selected → the first maximal entry
    // (`browser+search` @ 97) forces in, expanded to its members.
    let (result, warnings) =
        sample_toolsets_from_distribution_verbose("browser_tasks", Some(&mut || 1.0))
            .expect("sample");
    let set: std::collections::BTreeSet<String> = result.into_iter().collect();
    let expected: std::collections::BTreeSet<String> = ["browser", "search"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(set, expected);
    assert!(warnings.is_empty());
}

#[test]
fn oracle_invalid_member_skips_whole_entry_with_warning() {
    // ORACLE: `test_invalid_member_skips_whole_entry` — an invalid member
    // disqualifies the whole compound entry (even on a hit roll), the valid
    // sibling entries still select, and the warning text reaches stdout.
    // inject/remove are the Rust analog of the oracle's
    // monkeypatch.setitem(DISTRIBUTIONS, ...) (see distributions.rs PORT SEAMS).
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    inject_distribution(
        "_test_compound_invalid",
        "t",
        vec![
            ("browser+__not_a_toolset__".to_string(), 100),
            ("web".to_string(), 100),
        ],
    );
    let (result, warnings) =
        sample_toolsets_from_distribution_verbose("_test_compound_invalid", Some(&mut || 0.0))
            .expect("sample");
    assert!(
        !result.contains(&"browser".to_string()),
        "invalid member disqualifies the group"
    );
    assert!(result.contains(&"web".to_string()));
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("not valid") && w.contains("__not_a_toolset__")),
        "oracle asserts the warning text on stdout: {warnings:?}"
    );
    remove_distribution("_test_compound_invalid");
}

#[test]
fn probe_star_alias_safe_statics_and_member_whitespace() {
    // Oracle-probe pins: `*` == `all`; `safe` resolves to its included
    // non-core members; bundle_non_core(safe) is empty (every included
    // tool is core); `_entry_members` trims whitespace around '+'.
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    assert_eq!(
        resolve_toolset("*", None, false),
        resolve_toolset("all", None, false)
    );
    assert_eq!(
        resolve_toolset("safe", None, false),
        vec![
            "image_generate".to_string(),
            "vision_analyze".to_string(),
            "web_extract".to_string(),
            "web_search".to_string(),
        ]
    );
    assert!(bundle_non_core_tools("safe").is_empty());
    inject_distribution(
        "_probe_ws",
        "t",
        vec![(" browser + search ".to_string(), 100)],
    );
    let (result, _) =
        sample_toolsets_from_distribution_verbose("_probe_ws", Some(&mut || 0.0)).expect("sample");
    let set: std::collections::BTreeSet<String> = result.into_iter().collect();
    let expected: std::collections::BTreeSet<String> = ["browser", "search"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(set, expected, "members trim whitespace");
    remove_distribution("_probe_ws");
}
