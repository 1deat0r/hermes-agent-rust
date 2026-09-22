//! Toolset definitions and composition resolution.
//!
//! PARITY: toolsets.py @ 5d59366 (whole module) —
//!   sanitize-style helpers              (11–69)
//!   TOOLSETS / lists                    (11–241, data.rs)
//!   _registry / _registry_call          (244–258)
//!   _registry_generation                (261–263)
//!   get_toolset                         (266–307)
//!   bundle_non_core_tools               (310–325)
//!   _resolve_toolset_memo               (328–332)
//!   _plugin_platform_bundle             (335–352)
//!   resolve_toolset                     (355–397)
//!   _get_plugin_toolset_names           (400–402)
//!   _get_registry_toolset_aliases       (405–406)
//!   _display_alias                      (409–411)
//!   _plugin_display_names               (414–417)
//!   get_all_toolsets                    (420–431)
//!   get_toolset_names                   (434–436)
//!   validate_toolset                    (439–441)
//!   create_custom_toolset               (444–446)
//!   get_toolset_info                    (449–460)
//!   resolve_multiple_toolsets           (468–484, PLUGIN-COMPAT)
//!
//! PORT SEAMS (documented divergences):
//! - The memo key is `(name, include_registry, registry_generation,
//!   registry_scope)` — upstream also carries `id(registry)` to survive a
//!   monkeypatched registry swap; this port's registry is a process static
//!   that is never swapped, so the identity component is constant and
//!   omitted. Scope reads `registry.cache_scope()` (upstream
//!   `current_scope_key`).
//! - `_plugin_platform_bundle` (upstream lines 335–352) checks the gateway
//!   `platform_registry.is_registered(platform)` and fails open to `[]`
//!   when that import fails. The Rust gateway platform-registry seam has
//!   not landed, so the fail-open arm is what ships: no implicit
//!   `hermes-<plugin-platform>` bundle until `is_registered` is wired.
//! - `_display_alias` scans aliases in sorted-key order; upstream scans
//!   dict insertion order. Both are deterministic per process only when a
//!   single alias points at a canonical (the tested shape); sorted order is
//!   chosen so two aliases → one canonical resolve stably here.
//! - Registry-only `get_toolset` never hits the upstream `registry is None`
//!   arm — the tools crate singleton is always importable in this port.
//! - The PLUGIN-COMPAT `resolve_multiple_toolsets` is ported (upstream
//!   lines 468–484) because it is the public multi-name union helper;
//!   the rest of the PLUGIN-COMPAT block (`__getattr__` lazy pointer) is
//!   not ported (in-tree compat pointers are off limits).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use serde_json::{Map, Value};

use crate::data::{ToolsetDef, TOOLSETS};

/// Owned copy of one toolset definition (static or runtime-custom).
#[derive(Debug, Clone)]
struct OwnedToolset {
    description: String,
    tools: Vec<String>,
    includes: Vec<String>,
    posture: bool,
    module: Option<String>,
}

fn static_toolset(name: &str) -> Option<OwnedToolset> {
    TOOLSETS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, def)| OwnedToolset {
            description: def.description.to_string(),
            tools: def.tools.iter().map(|s| s.to_string()).collect(),
            includes: def.includes.iter().map(|s| s.to_string()).collect(),
            posture: def.posture,
            module: def.module.map(str::to_string),
        })
}

/// A runtime-created custom toolset: (description, tools, includes).
pub(crate) type CustomToolset = (String, Vec<String>, Vec<String>);

/// Runtime-created custom toolsets (create_custom_toolset), layered over the
/// static table — upstream's `TOOLSETS[name] = _ts(...)` inserts into the
/// same dict, so customs are first-class TOOLSETS members here too.
static CUSTOM_TOOLSETS: OnceLock<Mutex<HashMap<String, CustomToolset>>> = OnceLock::new();

fn custom_toolsets() -> &'static Mutex<HashMap<String, CustomToolset>> {
    CUSTOM_TOOLSETS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn toolset_all(name: &str) -> Option<OwnedToolset> {
    if let Some(def) = static_toolset(name) {
        return Some(def);
    }
    let custom = custom_toolsets().lock().expect("custom toolsets lock");
    custom
        .get(name)
        .map(|(desc, tools, includes)| OwnedToolset {
            description: desc.clone(),
            tools: tools.clone(),
            includes: includes.clone(),
            posture: false,
            module: None,
        })
}

/// Names of every dict-level TOOLSETS entry (static + runtime custom) —
/// upstream's `TOOLSETS.keys()`.
fn toolset_dict_keys() -> HashSet<String> {
    let mut keys: HashSet<String> = TOOLSETS.iter().map(|(n, _)| n.to_string()).collect();
    keys.extend(
        custom_toolsets()
            .lock()
            .expect("custom toolsets lock")
            .keys()
            .cloned(),
    );
    keys
}

/// Serialize one definition: base keys always; `posture` only when the
/// upstream dict carried it (truthy `_ts(..., posture=True)` — only
/// `coding`); `module` only when the upstream dict literal carried it
/// (only `hermes-yuanbao`).
///
/// PARITY: `{**toolset, ...}` spreads in `get_toolset` / `dict(TOOLSETS)`
/// in `get_all_toolsets` — key presence matches the upstream entry.
fn owned_to_json(def: &OwnedToolset, tools_override: Option<Vec<String>>) -> Value {
    let mut out = Map::new();
    out.insert("description".into(), Value::String(def.description.clone()));
    out.insert(
        "tools".into(),
        Value::Array(
            tools_override
                .unwrap_or_else(|| def.tools.clone())
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    out.insert(
        "includes".into(),
        Value::Array(def.includes.iter().cloned().map(Value::String).collect()),
    );
    if def.posture {
        out.insert("posture".into(), Value::Bool(true));
    }
    if let Some(module) = &def.module {
        out.insert("module".into(), Value::String(module.clone()));
    }
    Value::Object(out)
}

fn registry() -> &'static hermes_tools::registry::ToolRegistry {
    hermes_tools::registry::registry()
}

/// Registry toolset names absent from the dict-level TOOLSETS (static +
/// custom) — upstream `_get_plugin_toolset_names` (lines 400–402).
fn plugin_toolset_names() -> HashSet<String> {
    let dict_keys = toolset_dict_keys();
    registry()
        .get_registered_toolset_names()
        .into_iter()
        .filter(|n| !dict_keys.contains(n))
        .collect()
}

/// First non-static alias pointing at `ts_name`, or None — upstream
/// `_display_alias` (lines 409–411). Alias keys are scanned in sorted
/// order (see PORT SEAMS).
fn display_alias(ts_name: &str, aliases: &HashMap<String, String>) -> Option<String> {
    let dict_keys = toolset_dict_keys();
    let mut keys: Vec<&String> = aliases.keys().collect();
    keys.sort();
    keys.into_iter()
        .find(|a| aliases.get(a.as_str()) == Some(&ts_name.to_string()) && !dict_keys.contains(*a))
        .cloned()
}

/// Plugin toolset names under their first non-static alias when one exists —
/// upstream `_plugin_display_names` (lines 414–417).
fn plugin_display_names() -> Vec<String> {
    let aliases = registry().get_registered_toolset_aliases();
    plugin_toolset_names()
        .into_iter()
        .map(|n| display_alias(&n, &aliases).unwrap_or(n))
        .collect()
}

/// Get a toolset definition by name (dict-level static/custom + registry
/// merge), or None if unknown.
///
/// PARITY: `get_toolset` (upstream lines 266–307): `include_registry=False`
/// returns the static/custom copy only (so platform reverse-mapping, issue
/// #49622, is immune to registry additions); `True` unions registry tools
/// into static entries (sorted — `sorted(set | set)`), unions alias-target
/// tools when an alias shares the name (MCP server shadowing, lines 292–295),
/// and resolves registry-only names through the plugin / alias arms.
pub fn get_toolset(name: &str, include_registry: bool) -> Option<Value> {
    if !include_registry {
        let def = toolset_all(name)?;
        return Some(owned_to_json(&def, None));
    }
    if let Some(def) = toolset_all(name) {
        let reg = registry();
        // PARITY: `set(toolset["tools"]) | set(registry.get_tool_names_for_toolset(name))`
        // — BTreeSet unions + sorts exactly like `sorted(set | set)`.
        let mut merged: BTreeSet<String> = def.tools.iter().cloned().collect();
        merged.extend(reg.get_tool_names_for_toolset(name));
        // PARITY: alias-target union (lines 292–295) — an MCP server named
        // like a built-in registers a bare alias to its `mcp-<name>`
        // toolset; without the union the static entry shadows the server.
        if let Some(alias_target) = reg.get_toolset_alias_target(name) {
            if alias_target != name {
                merged.extend(reg.get_tool_names_for_toolset(&alias_target));
            }
        }
        let sorted: Vec<String> = merged.into_iter().collect();
        return Some(owned_to_json(&def, Some(sorted)));
    }
    // Registry-only toolset (plugin / MCP alias) — upstream lines 297–307.
    let reg = registry();
    let plugin_names = plugin_toolset_names();
    if plugin_names.contains(name) {
        let aliases = reg.get_registered_toolset_aliases();
        let description = match display_alias(name, &aliases) {
            Some(alias) => format!("MCP server '{alias}' tools"),
            None => format!("Plugin toolset: {name}"),
        };
        let tools = reg.get_tool_names_for_toolset(name);
        return Some(serde_json::json!({
            "description": description,
            "tools": tools,
            "includes": [],
        }));
    }
    // Alias pointing at a registry toolset whose name is not itself a
    // plugin name (the alias key is the lookup name).
    let alias_target = reg.get_toolset_alias_target(name)?;
    Some(serde_json::json!({
        "description": format!("MCP server '{name}' tools"),
        "tools": reg.get_tool_names_for_toolset(&alias_target),
        "includes": [],
    }))
}

/// A bundle's tools minus _HERMES_CORE_TOOLS (one level of includes).
///
/// PARITY: `bundle_non_core_tools` (upstream lines 310–325): disable a
/// `core + extras` bundle without stripping the shared core. One includes
/// pass suffices (only hermes-gateway nests bundles); unknown names fall
/// back to full resolution minus core. The core filter is applied where
/// upstream applies it (`to_remove - core`) — early here is set-equal.
pub fn bundle_non_core_tools(toolset_name: &str) -> HashSet<String> {
    let core: HashSet<&str> = crate::data::HERMES_CORE_TOOLS.iter().copied().collect();
    let Some(ts_def) = get_toolset(toolset_name, true) else {
        return resolve_toolset(toolset_name, None, true)
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .filter(|t| !core.contains(t.as_str()))
            .collect();
    };
    let ts_def = ts_def.as_object().expect("toolset object");
    let mut to_remove: HashSet<String> = ts_def
        .get("tools")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if let Some(includes) = ts_def.get("includes").and_then(Value::as_array) {
        for inc in includes.iter().filter_map(Value::as_str) {
            if let Some(inc_def) = get_toolset(inc, true) {
                if let Some(inc_tools) = inc_def.get("tools").and_then(Value::as_array) {
                    for t in inc_tools.iter().filter_map(Value::as_str) {
                        to_remove.insert(t.to_string());
                    }
                }
            }
        }
    }
    to_remove.retain(|t| !core.contains(t.as_str()));
    to_remove
}

/// PARITY: `_resolve_toolset_memo` (upstream lines 328–332) — keyed on
/// (name, include_registry, registry generation, profile scope). See PORT
/// SEAMS for the omitted `id(registry)` component.
type MemoKey = (String, bool, u64, String);

fn resolve_memo() -> &'static Mutex<HashMap<MemoKey, Vec<String>>> {
    static MEMO: OnceLock<Mutex<HashMap<MemoKey, Vec<String>>>> = OnceLock::new();
    MEMO.get_or_init(|| Mutex::new(HashMap::new()))
}

fn memo_key(name: &str, include_registry: bool) -> MemoKey {
    let reg = registry();
    (
        name.to_string(),
        include_registry,
        reg.generation_public(),
        reg.cache_scope(),
    )
}

/// Drop every memo entry (upstream tests call `_resolve_toolset_memo.clear()`).
pub fn clear_resolve_toolset_memo() {
    resolve_memo().lock().expect("memo lock").clear();
}

/// Recursively resolve a toolset (and its includes) to a sorted tool-name
/// list.
///
/// PARITY: `resolve_toolset` (upstream lines 355–397). `visited: None` is
/// the public entry (memo engages, `external_call`); `Some(set)` is the
/// recursion path (no memo — mirrors the `visited is not None` guard).
/// "all"/"*" spans every toolset (via `get_toolset_names`, which includes
/// plugin display names); diamond/cycle visits return `[]` silently.
pub fn resolve_toolset(
    name: &str,
    visited: Option<HashSet<String>>,
    include_registry: bool,
) -> Vec<String> {
    match visited {
        None => resolve_toolset_hit(name, include_registry).0,
        Some(mut visited) => resolve_walk(name, &mut visited, include_registry),
    }
}

/// Public-entry resolution that also reports whether the memo served it —
/// test seam for the upstream memo oracles (second resolve is a hit;
/// generation bump invalidates; result never changes).
pub fn resolve_toolset_hit(name: &str, include_registry: bool) -> (Vec<String>, bool) {
    let key = memo_key(name, include_registry);
    if let Some(cached) = resolve_memo().lock().expect("memo lock").get(&key) {
        return (cached.clone(), true);
    }
    let mut visited = HashSet::new();
    let result = resolve_walk(name, &mut visited, include_registry);
    // PARITY: the "all"/"*" arm returns before the memo store upstream
    // (lines 373–377 precede lines 393–396) — mirror by skipping the store;
    // every other external resolution stores, clearing at the 256 cap
    // ("stale-generation entries are never hit again").
    if name != "all" && name != "*" {
        let mut memo = resolve_memo().lock().expect("memo lock");
        if memo.len() >= 256 {
            memo.clear();
        }
        memo.insert(key, result.clone());
    }
    (result, false)
}

/// One non-memoized walk: shared `visited` across includes (upstream passes
/// the same set down the recursion; "all" copies per toolset).
fn resolve_walk(name: &str, visited: &mut HashSet<String>, include_registry: bool) -> Vec<String> {
    // "all"/"*" span every toolset so new toolsets are included automatically.
    if name == "all" || name == "*" {
        let mut all_tools: BTreeSet<String> = BTreeSet::new();
        for toolset_name in get_toolset_names() {
            all_tools.extend(resolve_walk(
                &toolset_name,
                &mut visited.clone(),
                include_registry,
            ));
        }
        return all_tools.into_iter().collect();
    }
    // Diamond include or cycle: [] silently — collected via another path.
    if visited.contains(name) {
        return Vec::new();
    }
    visited.insert(name.to_string());

    let Some(toolset) = get_toolset(name, include_registry) else {
        if include_registry {
            return plugin_platform_bundle(name);
        }
        return Vec::new();
    };
    let toolset = toolset.as_object().cloned().unwrap_or_default();
    let mut tools: BTreeSet<String> = toolset
        .get("tools")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if let Some(includes) = toolset.get("includes").and_then(Value::as_array) {
        for included_name in includes.iter().filter_map(Value::as_str) {
            tools.extend(resolve_walk(included_name, visited, include_registry));
        }
    }
    tools.into_iter().collect()
}

/// PARITY: `_plugin_platform_bundle` (upstream lines 335–352) — fails open
/// to `[]` (see PORT SEAMS: the gateway platform registry seam has not
/// landed; upstream also returns `[]` when its import fails).
fn plugin_platform_bundle(name: &str) -> Vec<String> {
    let _ = name;
    Vec::new()
}

/// Resolve multiple toolsets and combine their tools (deduplicated).
///
/// PARITY: `resolve_multiple_toolsets` (upstream lines 468–484).
pub fn resolve_multiple_toolsets(toolset_names: &[String]) -> Vec<String> {
    let mut all_tools: BTreeSet<String> = BTreeSet::new();
    for name in toolset_names {
        all_tools.extend(resolve_toolset(name, None, true));
    }
    all_tools.into_iter().collect()
}

/// Sorted names of all toolsets (dict-level static + custom, plus plugin
/// display names) — upstream `get_toolset_names` (lines 434–436).
pub fn get_toolset_names() -> Vec<String> {
    let mut names: BTreeSet<String> = toolset_dict_keys().into_iter().collect();
    names.extend(plugin_display_names());
    names.into_iter().collect()
}

/// All toolset definitions: dict-level entries (static + custom) plus
/// plugin-registered names, with static∩alias keys re-resolved through the
/// merged registry view — upstream `get_all_toolsets` (lines 420–431).
pub fn get_all_toolsets() -> Value {
    let mut result = Map::new();
    for (name, def) in TOOLSETS {
        result.insert(name.to_string(), owned_to_json_static(def));
    }
    for (name, def) in custom_toolsets().lock().expect("custom lock").iter() {
        let owned = OwnedToolset {
            description: def.0.clone(),
            tools: def.1.clone(),
            includes: def.2.clone(),
            posture: false,
            module: None,
        };
        result.insert(name.clone(), owned_to_json(&owned, None));
    }
    let aliases = registry().get_registered_toolset_aliases();
    for display_name in plugin_display_names() {
        // Plugin names that already exist as dict entries keep their
        // dict definition (upstream `None if display_name in result`).
        if result.contains_key(&display_name) {
            continue;
        }
        if let Some(toolset) = get_toolset(&display_name, true) {
            result.insert(display_name, toolset);
        }
    }
    // Static names an MCP server also aliases show the merged view.
    let dict_keys = toolset_dict_keys();
    let mut alias_intersection: Vec<String> = dict_keys
        .iter()
        .filter(|k| aliases.contains_key(k.as_str()))
        .cloned()
        .collect();
    alias_intersection.sort();
    for name in alias_intersection {
        if let Some(merged) = get_toolset(&name, true) {
            result.insert(name, merged);
        }
    }
    Value::Object(result)
}

/// Serialize a static (non-custom) definition for `get_all_toolsets`.
fn owned_to_json_static(def: &ToolsetDef) -> Value {
    let owned = OwnedToolset {
        description: def.description.to_string(),
        tools: def.tools.iter().map(|s| s.to_string()).collect(),
        includes: def.includes.iter().map(|s| s.to_string()).collect(),
        posture: def.posture,
        module: def.module.map(str::to_string),
    };
    owned_to_json(&owned, None)
}

/// Check if a toolset name is valid — upstream `validate_toolset`
/// (lines 439–441): the "all"/"*" aliases, dict membership (static +
/// custom), plugin names, or an alias key.
pub fn validate_toolset(name: &str) -> bool {
    if name == "all" || name == "*" {
        return true;
    }
    if toolset_all(name).is_some() {
        return true;
    }
    if plugin_toolset_names().contains(name) {
        return true;
    }
    registry()
        .get_registered_toolset_aliases()
        .contains_key(name)
}

/// Create a runtime toolset in the dict — upstream
/// `create_custom_toolset` (lines 444–446), which inserts into TOOLSETS.
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

/// Remove a runtime toolset — the analog of upstream tests doing
/// `del TOOLSETS[name]` in their `finally` blocks (production upstream has
/// no delete API; the dict mutation exists only as the test cleanup seam).
pub fn remove_custom_toolset(name: &str) {
    custom_toolsets().lock().expect("lock").remove(name);
}

/// Detailed information about a toolset including resolved tools —
/// upstream `get_toolset_info` (lines 449–460).
pub fn get_toolset_info(name: &str) -> Option<Value> {
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
        "is_composite": toolset.get("includes").and_then(Value::as_array).map(|a| !a.is_empty()).unwrap_or(false),
    }))
}
