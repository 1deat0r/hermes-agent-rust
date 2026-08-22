//! Toolset definitions and composition resolution.
//!
//! PARITY: toolsets.py @ b9aa928 —
//!   get_toolset                    (644–715)
//!   bundle_non_core_tools          (717–743)
//!   resolve_toolset                (745–825)
//!   resolve_multiple_toolsets      (827–844)
//!   _get_plugin_toolset_names      (846–861)
//!   _get_registry_toolset_aliases  (863–870)
//!   get_all_toolsets               (872–895)
//!   get_toolset_names              (897–918)
//!   validate_toolset               (920–937)
//!   create_custom_toolset          (940–950)
//!   get_toolset_info               (964–983)
//!
//! Registry seam: `registry_lookup` currently returns empty; the tools crate
//! (P2) wires `tools.registry` behind the same interface.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use crate::data::{ToolsetDef, TOOLSETS};

/// Registry-dependent lookups, backed by the live hermes-tools singleton.
pub(crate) fn registry_lookup() -> RegistryView {
    let reg = hermes_tools::registry::registry();
    RegistryView {
        names_for_toolset: reg
            .get_registered_toolset_names()
            .into_iter()
            .map(|ts| {
                let names = reg.get_tool_names_for_toolset(&ts);
                (ts, names)
            })
            .collect(),
        registered_toolset_names: reg.get_registered_toolset_names().into_iter().collect(),
        toolset_aliases: reg.get_registered_toolset_aliases(),
        platform_registered: reg
            .get_registered_toolset_names()
            .into_iter()
            .filter(|n| n.starts_with("hermes-"))
            .collect(),
    }
}

/// A snapshot of the tools.registry surface the toolsets module reads.
pub(crate) struct RegistryView {
    pub names_for_toolset: HashMap<String, Vec<String>>,
    pub registered_toolset_names: HashSet<String>,
    pub toolset_aliases: HashMap<String, String>,
    pub platform_registered: HashSet<String>,
}

fn static_toolset(name: &str) -> Option<ToolsetDef> {
    TOOLSETS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, def)| ToolsetDef {
            description: def.description,
            tools: def.tools,
            includes: def.includes,
            posture: def.posture,
        })
}

/// A runtime-created custom toolset: (description, tools, includes).
pub(crate) type CustomToolset = (String, Vec<String>, Vec<String>);

/// Runtime-created custom toolsets (create_custom_toolset), layered over the
/// static table.
static CUSTOM_TOOLSETS: OnceLock<Mutex<HashMap<String, CustomToolset>>> = OnceLock::new();

fn custom_toolsets() -> &'static Mutex<HashMap<String, CustomToolset>> {
    CUSTOM_TOOLSETS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn toolset_all(name: &str) -> Option<(String, Vec<String>, Vec<String>)> {
    // Static first, then runtime overlay.
    if let Some(def) = static_toolset(name) {
        return Some((
            def.description.to_string(),
            def.tools.iter().map(|s| s.to_string()).collect(),
            def.includes.iter().map(|s| s.to_string()).collect(),
        ));
    }
    let custom = custom_toolsets().lock().expect("custom toolsets lock");
    if let Some((desc, tools, includes)) = custom.get(name) {
        return Some((desc.clone(), tools.clone(), includes.clone()));
    }
    None
}

/// Get a toolset definition by name (static + custom view).
///
/// PARITY: toolsets.py get_toolset @ b9aa928. The registry-merge behavior of
/// `include_registry=True` is deferred until the tools crate lands — the
/// parameter is accepted for call-site parity and behaves statically.
pub fn get_toolset(name: &str, include_registry: bool) -> Option<serde_json::Value> {
    let registry = registry_lookup();
    let static_view = toolset_all(name);
    if !include_registry {
        // Static view only: built-in/custom definitions. Registry/MCP-only
        // toolsets have no static counterpart -> None.
        let (description, tools, includes) = static_view?;
        return Some(serde_json::json!({
            "description": description,
            "tools": tools,
            "includes": includes,
        }));
    }
    if let Some((description, tools, includes)) = static_view {
        // Registry-merged view: overlay tools plugins registered into this
        // toolset onto the static definition.
        let mut merged: Vec<String> = tools.into_iter().collect();
        if let Some(extra) = registry.names_for_toolset.get(name) {
            for t in extra {
                if !merged.contains(t) {
                    merged.push(t.clone());
                }
            }
        }
        merged.sort();
        return Some(serde_json::json!({
            "description": description,
            "tools": merged,
            "includes": includes,
        }));
    }
    // Registry-only toolset (plugin / MCP): synthesize the upstream shape.
    let names = registry.names_for_toolset.get(name)?.clone();
    let mut names = names;
    names.sort();
    Some(serde_json::json!({
        "description": format!("Plugin toolset: {name}"),
        "tools": names,
        "includes": [],
    }))
}

/// A bundle's platform-specific tools, excluding core.
///
/// PARITY: toolsets.py bundle_non_core_tools @ b9aa928 (717–743)
pub fn bundle_non_core_tools(toolset_name: &str) -> HashSet<String> {
    let core: HashSet<&str> = crate::data::HERMES_CORE_TOOLS.iter().copied().collect();
    let ts_def = get_toolset(toolset_name, true);
    let ts_def = ts_def.as_ref().and_then(|v| v.as_object());
    let ts_tools: Option<Vec<String>> = ts_def.and_then(|o| {
        o.get("tools")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
    });
    let Some(ts_tools) = ts_tools else {
        return resolve_toolset(toolset_name, Some(HashSet::new()), true)
            .into_iter()
            .collect::<HashSet<_>>()
            .difference(&core.iter().map(|s| s.to_string()).collect::<HashSet<_>>())
            .cloned()
            .collect();
    };
    let mut to_remove: HashSet<String> = ts_tools
        .into_iter()
        .filter(|t| !core.contains(t.as_str()))
        .collect();
    if let Some(includes) = ts_def.and_then(|o| o.get("includes")).and_then(|v| v.as_array()) {
        for inc in includes.iter().filter_map(|v| v.as_str()) {
            if let Some(inc_def) = get_toolset(inc, true).and_then(|v| v.as_object().cloned()) {
                if let Some(inc_tools) = inc_def.get("tools").and_then(|v| v.as_array()) {
                    for t in inc_tools.iter().filter_map(|x| x.as_str()) {
                        if !core.contains(t) {
                            to_remove.insert(t.to_string());
                        }
                    }
                }
            }
        }
    }
    to_remove
}

/// Recursively resolve a toolset to all tool names (cycle-safe).
///
/// PARITY: toolsets.py resolve_toolset @ b9aa928 (745–825)
pub fn resolve_toolset(
    name: &str,
    visited: Option<HashSet<String>>,
    _include_registry: bool,
) -> Vec<String> {
    let mut visited = visited.unwrap_or_default();

    // Special aliases representing all tools across every toolset.
    if name == "all" || name == "*" {
        let mut all_tools: HashSet<String> = HashSet::new();
        for (toolset_name, _) in TOOLSETS {
            let branch = resolve_toolset(toolset_name, Some(visited.clone()), _include_registry);
            all_tools.extend(branch);
        }
        let mut sorted: Vec<String> = all_tools.into_iter().collect();
        sorted.sort();
        return sorted;
    }

    if visited.contains(name) {
        return Vec::new();
    }
    visited.insert(name.to_string());

    let Some(toolset) = get_toolset(name, _include_registry) else {
        // Plugin platforms (hermes-<name>) auto-generate core + registered
        // platform tools. Registry seam: static core only.
        if _include_registry && name.starts_with("hermes-") {
            let registry = registry_lookup();
            let platform_name = &name["hermes-".len()..];
            if registry.platform_registered.contains(platform_name) {
                let mut plugin_tools: HashSet<String> =
                    crate::data::HERMES_CORE_TOOLS.iter().map(|s| s.to_string()).collect();
                if let Some(tools) = registry.names_for_toolset.get(platform_name) {
                    plugin_tools.extend(tools.iter().cloned());
                }
                let mut sorted: Vec<String> = plugin_tools.into_iter().collect();
                sorted.sort();
                return sorted;
            }
        }
        return Vec::new();
    };

    let toolset = toolset.as_object().cloned().unwrap_or_default();
    let mut tools: HashSet<String> = toolset
        .get("tools")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    if let Some(includes) = toolset.get("includes").and_then(|v| v.as_array()) {
        for included_name in includes.iter().filter_map(|v| v.as_str()) {
            let included_tools =
                resolve_toolset(included_name, Some(visited.clone()), _include_registry);
            tools.extend(included_tools);
        }
    }

    let mut sorted: Vec<String> = tools.into_iter().collect();
    sorted.sort();
    sorted
}

/// Resolve multiple toolsets and combine their tools (deduplicated).
///
/// PARITY: toolsets.py resolve_multiple_toolsets @ b9aa928 (827–844)
pub fn resolve_multiple_toolsets(toolset_names: &[String]) -> Vec<String> {
    let mut all_tools: HashSet<String> = HashSet::new();
    for name in toolset_names {
        let tools = resolve_toolset(name, None, true);
        all_tools.extend(tools);
    }
    let mut sorted: Vec<String> = all_tools.into_iter().collect();
    sorted.sort();
    sorted
}

/// Static toolset names (registry-expanded once the tools crate lands).
pub fn get_toolset_names() -> Vec<String> {
    let mut names: Vec<String> = TOOLSETS
        .iter()
        .map(|(n, _)| n.to_string())
        .collect();
    names.sort();
    names
}

/// All available toolsets: static definitions keyed by name.
pub fn get_all_toolsets() -> serde_json::Value {
    let mut result = serde_json::Map::new();
    for (name, def) in TOOLSETS {
        result.insert(
            name.to_string(),
            serde_json::json!({
                "description": def.description,
                "tools": def.tools,
                "includes": def.includes,
            }),
        );
    }
    serde_json::Value::Object(result)
}

/// Check if a toolset name is valid (special aliases + static names).
///
/// PARITY: toolsets.py validate_toolset @ b9aa928 (920–937)
pub fn validate_toolset(name: &str) -> bool {
    if name == "all" || name == "*" {
        return true;
    }
    if static_toolset(name).is_some() {
        return true;
    }
    if custom_toolsets().lock().expect("lock").contains_key(name) {
        return true;
    }
    let registry = registry_lookup();
    if registry.registered_toolset_names.contains(name) {
        return true;
    }
    registry.toolset_aliases.contains_key(name)
}

/// Create a custom toolset at runtime.
///
/// PARITY: toolsets.py create_custom_toolset @ b9aa928 (940–950)
pub fn create_custom_toolset(
    name: &str,
    description: &str,
    tools: Vec<String>,
    includes: Vec<String>,
) {
    custom_toolsets()
        .lock()
        .expect("lock")
        .insert(name.to_string(), (description.to_string(), tools, includes));
}

/// Detailed information about a toolset including resolved tools.
///
/// PARITY: toolsets.py get_toolset_info @ b9aa928 (964–983)
pub fn get_toolset_info(name: &str) -> Option<serde_json::Value> {
    let toolset = get_toolset(name, true)?;
    let toolset = toolset.as_object().cloned().unwrap_or_default();
    let resolved_tools = resolve_toolset(name, None, true);
    Some(serde_json::json!({
        "name": name,
        "description": toolset.get("description"),
        "direct_tools": toolset.get("tools"),
        "includes": toolset.get("includes"),
        "resolved_tools": resolved_tools,
        "tool_count": resolved_tools.len(),
        "is_composite": toolset.get("includes").and_then(|v| v.as_array()).map(|a| !a.is_empty()).unwrap_or(false),
    }))
}
