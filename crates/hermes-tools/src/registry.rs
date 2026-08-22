//! Tool registry — the singleton every tool registers into.
//!
//! PARITY: tools/registry.py @ b9aa928 (956 LOC, ported 1:1 for the
//! observable surfaces; see the crate docs for the handler/ownership seams).
//!
//! Tool modules register into `registry()` at startup; consumers read
//! schemas via `get_definitions` and execute via `dispatch`.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};

use serde_json::{json, Value};

pub const CHECK_FN_CACHE_BYPASS: &str = "";
const CHECK_FN_TTL_SECONDS: f64 = 30.0;
const CHECK_FN_FAILURE_GRACE_SECONDS: f64 = 60.0;
const CHECK_FN_CACHE_MAX: usize = 512;

/// Result of a tool handler: the string passed to the agent pipeline, or the
/// supported multimodal envelope.
#[derive(Debug, Clone)]
pub enum ToolResult {
    Text(String),
    /// `{"_multimodal": true, "content": [...]}` envelope.
    Multimodal(Value),
}

/// A registered tool's executable handler.
pub trait ToolHandler: Send + Sync {
    /// Invoke the handler with the parsed args plus the dispatch kwargs
    /// (`task_id`, `user_task`) that Python handlers receive.
    fn call(&self, args: Value, task_id: Option<&str>, user_task: Option<&str>) -> ToolResult;
}

/// Availability probe (Python `check_fn`). Exceptions are swallowed by the
/// caller exactly like upstream.
pub trait CheckFn: Send + Sync {
    fn check(&self) -> bool;
}

/// Stable identity for a check_fn so the TTL cache can key on it.
pub struct CheckFnEntry {
    pub id: usize,
    pub name: &'static str,
    pub probe: Arc<dyn CheckFn>,
}

impl CheckFnEntry {
    fn new(probe: Arc<dyn CheckFn>, name: &'static str) -> Self {
        // Identity follows the Arc pointer: two registrations passing the
        // same Arc share one cache entry (mirrors Python's callable-identity
        // cache keys).
        let id = std::sync::Arc::as_ptr(&probe) as *const u8 as usize;
        CheckFnEntry { id, name, probe }
    }
}

/// Optional zero-arg callable returning schema overrides applied at
/// `get_definitions()` time.
pub type DynamicOverrides = dyn Fn() -> Value + Send + Sync;

/// Metadata for a single registered tool.
pub struct ToolEntry {
    pub name: String,
    pub toolset: String,
    pub schema: Value,
    pub handler: Arc<dyn ToolHandler>,
    pub check_fn: Option<Arc<CheckFnEntry>>,
    pub requires_env: Vec<String>,
    pub description: String,
    pub emoji: String,
    pub max_result_size_chars: Option<i64>,
    pub dynamic_schema_overrides: Option<Arc<DynamicOverrides>>,
    /// Python handler module (`handler.__globals__["__name__"]` equivalent),
    /// used by the plugin-override ownership policy.
    pub owner_module: Option<String>,
}

/// Shared check_fn cache keyed by (check_fn id, scope).
#[derive(Default)]
pub struct CheckFnCache {
    cache: Mutex<HashMap<(usize, String), (f64, bool)>>,
    last_good: Mutex<HashMap<(usize, String), f64>>,
}

impl CheckFnCache {
    pub fn prune(&self, now: f64) {
        let mut cache = self.cache.lock().expect("cache lock");
        let mut last_good = self.last_good.lock().expect("last_good lock");
        cache.retain(|_, (ts, _)| now - *ts < CHECK_FN_TTL_SECONDS);
        last_good.retain(|_, ts| now - *ts < CHECK_FN_FAILURE_GRACE_SECONDS);
        while cache.len() >= CHECK_FN_CACHE_MAX {
            let key = cache.keys().next().cloned();
            if let Some(key) = key {
                cache.remove(&key);
            } else {
                break;
            }
        }
        while last_good.len() >= CHECK_FN_CACHE_MAX {
            let key = last_good.keys().next().cloned();
            if let Some(key) = key {
                last_good.remove(&key);
            } else {
                break;
            }
        }
    }

    /// `_check_fn_cached`: bool(fn()), TTL-cached, flake-suppressed.
    ///
    /// PARITY: tools/registry.py _check_fn_cached @ b9aa928 (271–339)
    pub fn cached(&self, check: &CheckFnEntry, scope: &str) -> bool {
        let now = monotonic();
        if scope == CHECK_FN_CACHE_BYPASS {
            return check.probe.check();
        }
        let key = (check.id, scope.to_string());
        {
            let cache = self.cache.lock().expect("cache lock");
            self.prune_lock(&cache, now);
            if let Some((ts, value)) = cache.get(&key) {
                if now - *ts < CHECK_FN_TTL_SECONDS {
                    return *value;
                }
            }
        }
        let value = check.probe.check();
        {
            let mut cache = self.cache.lock().expect("cache lock");
            let mut last_good = self.last_good.lock().expect("last_good lock");
            self.prune_caches(&mut cache, &mut last_good, now);
            if value {
                last_good.insert(key.clone(), now);
                cache.insert(key, (now, true));
                return true;
            }
            if let Some(last) = last_good.get(&key) {
                if now - *last < CHECK_FN_FAILURE_GRACE_SECONDS {
                    // Recent success -> treat as a flake; serve last-good and
                    // do NOT cache the failure.
                    return true;
                }
            }
            cache.insert(key, (now, false));
            false
        }
    }

    /// `get_cached_check_fn_result`: never runs the probe.
    pub fn get_cached(&self, check: &CheckFnEntry, scope: &str) -> Option<bool> {
        let now = monotonic();
        if scope == CHECK_FN_CACHE_BYPASS {
            return None;
        }
        let cache = self.cache.lock().expect("cache lock");
        let (ts, value) = cache.get(&(check.id, scope.to_string()))?;
        if now - *ts < CHECK_FN_TTL_SECONDS {
            Some(*value)
        } else {
            None
        }
    }

    pub fn invalidate(&self) {
        self.cache.lock().expect("cache lock").clear();
        self.last_good.lock().expect("last_good lock").clear();
    }

    fn prune_lock(&self, cache: &HashMap<(usize, String), (f64, bool)>, now: f64) {
        let _ = cache; // prune happens under write paths; read path leaves stale rows
        let _ = now;
    }

    fn prune_caches(
        &self,
        cache: &mut HashMap<(usize, String), (f64, bool)>,
        last_good: &mut HashMap<(usize, String), f64>,
        now: f64,
    ) {
        cache.retain(|_, (ts, _)| now - *ts < CHECK_FN_TTL_SECONDS);
        last_good.retain(|_, ts| now - *ts < CHECK_FN_FAILURE_GRACE_SECONDS);
        while cache.len() >= CHECK_FN_CACHE_MAX {
            let key = cache.keys().next().cloned();
            if let Some(key) = key {
                cache.remove(&key);
            } else {
                break;
            }
        }
    }
}

static MONO_EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn monotonic() -> f64 {
    let epoch = MONO_EPOCH.get_or_init(std::time::Instant::now);
    epoch.elapsed().as_secs_f64()
}

/// The singleton registry.
pub struct ToolRegistry {
    inner: RwLock<RegistryInner>,
    pub check_fn_cache: CheckFnCache,
}

struct RegistryInner {
    tools: HashMap<String, Arc<ToolEntry>>,
    // toolset -> first-registered check_fn identity (classification only).
    toolset_checks: HashMap<String, usize>,
    toolset_aliases: HashMap<String, String>,
    plugin_override_policy: HashMap<String, bool>,
    generation: u64,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        ToolRegistry {
            inner: RwLock::new(RegistryInner {
                tools: HashMap::new(),
                toolset_checks: HashMap::new(),
                toolset_aliases: HashMap::new(),
                plugin_override_policy: HashMap::new(),
                generation: 0,
            }),
            check_fn_cache: CheckFnCache::default(),
        }
    }

    /// Generation counter for cache-keying (registry mutations bump it).
    pub fn generation_public(&self) -> u64 {
        self.inner.read().expect("lock").generation
    }

    fn snapshot_entries(&self) -> Vec<Arc<ToolEntry>> {
        let inner = self.inner.read().expect("lock");
        let mut entries: Vec<Arc<ToolEntry>> = inner.tools.values().cloned().collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }

    /// Register a tool. Called by tool modules at startup.
    ///
    /// PARITY: tools/registry.py ToolRegistry.register @ b9aa928 (521–603)
    #[allow(clippy::too_many_arguments)]
    pub fn register(
        &self,
        name: &str,
        toolset: &str,
        schema: Value,
        handler: Arc<dyn ToolHandler>,
        check_fn: Option<Arc<dyn CheckFn>>,
        check_fn_name: Option<&'static str>,
        requires_env: Vec<String>,
        description: Option<String>,
        emoji: Option<String>,
        max_result_size_chars: Option<i64>,
        dynamic_schema_overrides: Option<Arc<DynamicOverrides>>,
        owner_module: Option<String>,
        override_allowed: bool,
    ) -> Result<(), String> {
        let mut inner = self.inner.write().expect("lock");
        let existing = inner.tools.get(name).cloned();
        let mut raised: Option<String> = None;
        if let Some(existing) = &existing {
            if existing.toolset != toolset {
                if override_allowed {
                    // Plugin ownership opt-in gate (same policy upstream).
                    if let Some(owner) = &owner_module {
                        if owner.starts_with("hermes_plugins.")
                            && !inner.plugin_override_policy.get(owner).copied().unwrap_or(false)
                            && existing.owner_module.as_deref().map(|o| o != owner).unwrap_or(true)
                        {
                            raised = Some(format!(
                                "Plugin module {owner:?} cannot override built-in tool {name:?} without operator opt-in (allow_tool_override)."
                            ));
                        }
                    }
                    if raised.is_none() {
                        // Explicit opt-in: replace.
                    }
                } else {
                    // Reject every cross-toolset shadow.
                    return Ok(());
                }
            }
            if let Some(msg) = raised {
                return Err(msg);
            }
        }
        let check_entry = check_fn.map(|probe| {
            Arc::new(CheckFnEntry::new(
                probe,
                check_fn_name.unwrap_or("check_fn"),
            ))
        });
        let description = description.unwrap_or_else(|| {
            schema
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        });
        if let Some(check) = &check_entry {
            if !inner.toolset_checks.contains_key(toolset) {
                inner.toolset_checks.insert(toolset.to_string(), check.id);
            }
        }
        inner.tools.insert(
            name.to_string(),
            Arc::new(ToolEntry {
                name: name.to_string(),
                toolset: toolset.to_string(),
                schema,
                handler,
                check_fn: check_entry,
                requires_env,
                description,
                emoji: emoji.unwrap_or_default(),
                max_result_size_chars,
                dynamic_schema_overrides,
                owner_module,
            }),
        );
        inner.generation += 1;
        Ok(())
    }

    /// Remove a tool, gated by the plugin-override ownership policy for
    /// non-MCP toolsets.
    ///
    /// PARITY: tools/registry.py ToolRegistry.deregister @ b9aa928 (605–672)
    pub fn deregister(&self, name: &str, caller_module: Option<&str>) -> Result<(), String> {
        let mut inner = self.inner.write().expect("lock");
        let Some(entry) = inner.tools.get(name).cloned() else {
            return Ok(());
        };
        if !entry.toolset.starts_with("mcp-") {
            if let Some(caller) = caller_module {
                let caller_root = caller.split('.').take(2).collect::<Vec<_>>().join(".");
                let owner_root = entry
                    .owner_module
                    .as_deref()
                    .map(|o| o.split('.').take(2).collect::<Vec<_>>().join("."))
                    .unwrap_or_default();
                let same_plugin = entry.owner_module.is_some() && caller_root == owner_root;
                if caller.starts_with("hermes_plugins.")
                    && !same_plugin
                    && !inner
                        .plugin_override_policy
                        .get(&caller_root)
                        .copied()
                        .unwrap_or(false)
                {
                    return Err(format!(
                        "Plugin module {caller:?} cannot deregister tool {name:?} (toolset {:?}) without operator opt-in (allow_tool_override).",
                        entry.toolset
                    ));
                }
            }
        }
        inner.tools.remove(name);
        let toolset_still_exists = inner
            .tools
            .values()
            .any(|e| e.toolset == entry.toolset);
        if !toolset_still_exists {
            inner.toolset_checks.remove(&entry.toolset);
            inner.toolset_aliases.retain(|_, target| target != &entry.toolset);
        }
        inner.generation += 1;
        Ok(())
    }

    pub fn register_plugin_override_policy(&self, module_namespace: &str, allowed: bool) {
        self.inner
            .write()
            .expect("lock")
            .plugin_override_policy
            .insert(module_namespace.to_string(), allowed);
    }

    pub fn register_toolset_alias(&self, alias: &str, toolset: &str) {
        let mut inner = self.inner.write().expect("lock");
        if let Some(existing) = inner.toolset_aliases.get(alias) {
            if existing != toolset {
                eprintln!(
                    "[hermes-tools] WARN: toolset alias collision: '{alias}' ({existing}) overwritten by {toolset}"
                );
            }
        }
        inner.toolset_aliases.insert(alias.to_string(), toolset.to_string());
        inner.generation += 1;
    }

    pub fn get_registered_toolset_aliases(&self) -> HashMap<String, String> {
        self.inner.read().expect("lock").toolset_aliases.clone()
    }

    pub fn get_toolset_alias_target(&self, alias: &str) -> Option<String> {
        self.inner
            .read()
            .expect("lock")
            .toolset_aliases
            .get(alias)
            .cloned()
    }

    // ── query helpers ────────────────────────────────────────────────────

    pub fn get_entry(&self, name: &str) -> Option<Arc<ToolEntry>> {
        self.inner.read().expect("lock").tools.get(name).cloned()
    }

    pub fn get_registered_toolset_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .snapshot_entries()
            .iter()
            .map(|e| e.toolset.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    pub fn get_tool_names_for_toolset(&self, toolset: &str) -> Vec<String> {
        let mut names: Vec<String> = self
            .snapshot_entries()
            .iter()
            .filter(|e| e.toolset == toolset)
            .map(|e| e.name.clone())
            .collect();
        names.sort();
        names
    }

    /// OpenAI-format tool schemas for the requested tool names, filtered by
    /// check_fn availability.
    ///
    /// PARITY: tools/registry.py ToolRegistry.get_definitions @ b9aa928
    /// (676–729)
    pub fn get_definitions(&self, tool_names: &HashSet<String>, quiet: bool) -> Vec<Value> {
        let entries_by_name: HashMap<String, Arc<ToolEntry>> = self
            .snapshot_entries()
            .into_iter()
            .map(|e| (e.name.clone(), e))
            .collect();
        let mut result: Vec<Value> = Vec::new();
        let mut check_results: HashMap<usize, bool> = HashMap::new();
        let scope = self.cache_scope();
        let mut names: Vec<&String> = tool_names.iter().collect();
        names.sort();
        for name in names {
            let Some(entry) = entries_by_name.get(name) else { continue };
            if let Some(check) = &entry.check_fn {
                let verdict = *check_results.entry(check.id).or_insert_with(|| {
                    self.check_fn_cache.cached(check, &scope)
                });
                if !verdict {
                    if !quiet {
                        eprintln!("[hermes-tools] DEBUG: Tool {name} unavailable (check failed)");
                    }
                    continue;
                }
            }
            let mut schema_with_name = entry.schema.clone();
            if let Some(obj) = schema_with_name.as_object_mut() {
                obj.insert("name".to_string(), json!(entry.name));
            }
            if let Some(overrides) = &entry.dynamic_schema_overrides {
                let overrides = overrides();
                if let Some(over_obj) = overrides.as_object() {
                    if let Some(obj) = schema_with_name.as_object_mut() {
                        for (k, v) in over_obj {
                            obj.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
            result.push(json!({"type": "function", "function": schema_with_name}));
        }
        result
    }

    /// Execute a tool handler by name; normalizes to string / multimodal.
    ///
    /// PARITY: tools/registry.py ToolRegistry.dispatch @ b9aa928 (760–792)
    pub fn dispatch(
        &self,
        name: &str,
        args: Value,
        task_id: Option<&str>,
        user_task: Option<&str>,
    ) -> Value {
        let Some(entry) = self.get_entry(name) else {
            return json!({"error": format!("Unknown tool: {name}")});
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry.handler.call(args, task_id, user_task)
        }));
        match result {
            Ok(ToolResult::Text(s)) => json!(s),
            Ok(ToolResult::Multimodal(m)) => m,
            Err(_) => json!({
                "error": "Tool execution failed: ToolPanic: handler panicked"
            }),
        }
    }

    pub fn get_max_result_size(&self, name: &str, default: Option<i64>) -> i64 {
        if let Some(entry) = self.get_entry(name) {
            if let Some(limit) = entry.max_result_size_chars {
                return limit;
            }
        }
        default.unwrap_or(100_000)
    }

    pub fn get_all_tool_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.snapshot_entries().iter().map(|e| e.name.clone()).collect();
        names.sort();
        names
    }

    pub fn get_schema(&self, name: &str) -> Option<Value> {
        self.get_entry(name).map(|e| e.schema.clone())
    }

    pub fn get_toolset_for_tool(&self, name: &str) -> Option<String> {
        self.get_entry(name).map(|e| e.toolset.clone())
    }

    pub fn get_emoji(&self, name: &str, default: &str) -> String {
        self.get_entry(name)
            .map(|e| if e.emoji.is_empty() { default.to_string() } else { e.emoji.clone() })
            .unwrap_or_else(|| default.to_string())
    }

    pub fn get_tool_to_toolset_map(&self) -> HashMap<String, String> {
        self.snapshot_entries()
            .into_iter()
            .map(|e| (e.name.clone(), e.toolset.clone()))
            .collect()
    }

    fn toolset_has_exposable_tools(&self, toolset: &str, entries: &[Arc<ToolEntry>]) -> bool {
        let scope = self.cache_scope();
        let mut check_results: HashMap<usize, bool> = HashMap::new();
        for entry in entries {
            if entry.toolset != toolset {
                continue;
            }
            let Some(check) = &entry.check_fn else {
                return true;
            };
            let verdict = *check_results.entry(check.id).or_insert_with(|| {
                self.check_fn_cache.cached(check, &scope)
            });
            if verdict {
                return true;
            }
        }
        false
    }

    pub fn is_toolset_available(&self, toolset: &str) -> bool {
        let entries = self.snapshot_entries();
        self.toolset_has_exposable_tools(toolset, &entries)
    }

    pub fn check_toolset_requirements(&self) -> HashMap<String, bool> {
        let entries = self.snapshot_entries();
        let mut toolsets: Vec<String> = entries.iter().map(|e| e.toolset.clone()).collect();
        toolsets.sort();
        toolsets.dedup();
        toolsets
            .into_iter()
            .map(|t| {
                let ok = self.toolset_has_exposable_tools(&t, &entries);
                (t, ok)
            })
            .collect()
    }

    pub fn get_available_toolsets(&self) -> HashMap<String, Value> {
        let entries = self.snapshot_entries();
        let mut toolsets: HashMap<String, Value> = HashMap::new();
        for entry in &entries {
            let ts = entry.toolset.clone();
            let slot = toolsets.entry(ts.clone()).or_insert_with(|| {
                let available = self.toolset_has_exposable_tools(&ts, &entries);
                json!({
                    "available": available,
                    "tools": Vec::<String>::new(),
                    "description": "",
                    "requirements": Vec::<String>::new(),
                })
            });
            if let Some(obj) = slot.as_object_mut() {
                obj.get_mut("tools")
                    .and_then(Value::as_array_mut)
                    .unwrap()
                    .push(json!(entry.name.clone()));
                for env in &entry.requires_env {
                    let reqs = obj.get_mut("requirements").and_then(Value::as_array_mut).unwrap();
                    if !reqs.iter().any(|v| v.as_str() == Some(env)) {
                        reqs.push(json!(env.clone()));
                    }
                }
            }
        }
        toolsets
    }

    pub fn get_toolset_requirements(&self) -> HashMap<String, Value> {
        let entries = self.snapshot_entries();
        let inner = self.inner.read().expect("lock");
        let mut result: HashMap<String, Value> = HashMap::new();
        for entry in &entries {
            let ts = entry.toolset.clone();
            let slot = result.entry(ts.clone()).or_insert_with(|| {
                let check_id = inner.toolset_checks.get(&ts).copied();
                json!({
                    "name": ts.clone(),
                    "env_vars": Vec::<String>::new(),
                    "check_fn": check_id.map(|id| json!(id)).unwrap_or(Value::Null),
                    "setup_url": Value::Null,
                    "tools": Vec::<String>::new(),
                })
            });
            if let Some(obj) = slot.as_object_mut() {
                let tools = obj.get_mut("tools").and_then(Value::as_array_mut).unwrap();
                if !tools.iter().any(|v| v.as_str() == Some(&entry.name)) {
                    tools.push(json!(entry.name.clone()));
                }
                for env in &entry.requires_env {
                    let envs = obj.get_mut("env_vars").and_then(Value::as_array_mut).unwrap();
                    if !envs.iter().any(|v| v.as_str() == Some(env)) {
                        envs.push(json!(env.clone()));
                    }
                }
            }
        }
        result
    }

    pub fn check_tool_availability(&self, quiet: bool) -> (Vec<String>, Vec<Value>) {
        let entries = self.snapshot_entries();
        let mut toolsets: Vec<String> = entries.iter().map(|e| e.toolset.clone()).collect();
        toolsets.sort();
        toolsets.dedup();
        let mut available: Vec<String> = Vec::new();
        let mut unavailable: Vec<Value> = Vec::new();
        for ts in &toolsets {
            let ts_entries: Vec<Arc<ToolEntry>> =
                entries.iter().filter(|e| &e.toolset == ts).cloned().collect();
            if self.toolset_has_exposable_tools(ts, &entries) {
                available.push(ts.clone());
            } else {
                unavailable.push(json!({
                    "name": ts,
                    "env_vars": ts_entries.first().map(|e| e.requires_env.clone()).unwrap_or_default(),
                    "tools": ts_entries.iter().map(|e| json!(e.name.clone())).collect::<Vec<_>>(),
                }));
            }
        }
        let _ = quiet;
        (available, unavailable)
    }

    /// Cache scope seam: multiplex profile isolation (agent/secret_scope)
    /// deferred — single-profile processes keep the process-wide cache.
    pub fn cache_scope(&self) -> String {
        String::new()
    }
}

/// Module-level singleton (tools/registry.py `registry = ToolRegistry()`).
pub fn registry() -> &'static ToolRegistry {
    static REG: once_cell::sync::OnceCell<ToolRegistry> = once_cell::sync::OnceCell::new();
    REG.get_or_init(ToolRegistry::new)
}

/// JSON error string for tool handlers.
///
/// PARITY: tools/registry.py tool_error @ b9aa928 (930–941)
pub fn tool_error(message: impl Into<String>, extra: &[(String, Value)]) -> String {
    let mut result = serde_json::Map::new();
    result.insert("error".to_string(), json!(message.into()));
    for (k, v) in extra {
        result.insert(k.clone(), v.clone());
    }
    serde_json::to_string(&Value::Object(result)).expect("json")
}

/// JSON result string for tool handlers.
///
/// PARITY: tools/registry.py tool_result @ b9aa928 (944–953)
pub fn tool_result(data: Value) -> String {
    serde_json::to_string(&data).expect("json")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoHandler;
    impl ToolHandler for EchoHandler {
        fn call(&self, args: Value, _: Option<&str>, _: Option<&str>) -> ToolResult {
            ToolResult::Text(args["value"].as_str().unwrap_or("").to_string())
        }
    }

    #[test]
    fn register_and_dispatch() {
        let r = ToolRegistry::new();
        r.register(
            "echo",
            "web",
            json!({"description": "echo", "properties": {"value": {"type": "string"}}}),
            Arc::new(EchoHandler),
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
        assert_eq!(r.get_all_tool_names(), vec!["echo".to_string()]);
        let out = r.dispatch("echo", json!({"value": "hi"}), None, None);
        assert_eq!(out, json!("hi"));
        let defs = r.get_definitions(&HashSet::from(["echo".to_string()]), false);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0]["function"]["name"], json!("echo"));
    }
}
