//! Model tool orchestration layer (schema provider + argument coercion +
//! dispatch shims).
//!
//! PARITY: model_tools.py @ b9aa928 —
//!   _LEGACY_TOOLSET_MAP            (252–273)
//!   get_tool_definitions           (305–389)
//!   _compute_tool_definitions      (391–612)
//!   _sanitize_tool_error           (710–728)
//!   coerce_tool_args               (730–837)
//!   _schema_accepts_kind           (838–857)
//!   _normalize_json_strings_for_schema (859–934)
//!   _coerce_value                  (936–963)
//!   _schema_allows_null            (965–987)
//!   _coerce_json                   (989–1018)
//!   _coerce_number                 (1020–1036)
//!   _coerce_boolean                (1038–1046)
//!   get_all_tool_names             (1547–1550)
//!   get_toolset_for_tool           (1552–1555)
//!   get_available_toolsets         (1557–1560)
//!   check_toolset_requirements     (1562–1565)
//!   check_tool_availability        (1567–1570)
//!
//! DEFERRED SEAMS (documented, wired with the agent/core crates):
//! `handle_function_call` (+ observer fields/post-call hooks, rewind and
//! coordinator middleware) lives with the agent loop; execute_code /
//! discord dynamic-schema rebuilds, schema_sanitizer, tool_search assembly,
//! and `_resolve_active_context_length` need their tool/config crates.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Map, Value};

use hermes_tools::registry::registry;
use crate::{bundle_non_core_tools, get_all_toolsets, get_toolset, resolve_toolset, validate_toolset};

/// Legacy `_tools`-suffixed toolset names -> tool name lists.
pub static LEGACY_TOOLSET_MAP: Lazy<HashMap<&'static str, Vec<&'static str>>> =
    Lazy::new(|| {
        let mut m = HashMap::new();
        m.insert("web_tools", vec!["web_search", "web_extract"]);
        m.insert("terminal_tools", vec!["terminal"]);
        m.insert("vision_tools", vec!["vision_analyze"]);
        m.insert("image_tools", vec!["image_generate"]);
        m.insert("skills_tools", vec!["skills_list", "skill_view", "skill_manage"]);
        m.insert(
            "browser_tools",
            vec![
                "browser_navigate", "browser_snapshot", "browser_click", "browser_type",
                "browser_scroll", "browser_back", "browser_press", "browser_get_images",
                "browser_vision", "browser_console",
            ],
        );
        m.insert("cronjob_tools", vec!["cronjob"]);
        m.insert(
            "file_tools",
            vec!["read_file", "write_file", "patch", "search_files"],
        );
        m.insert("tts_tools", vec!["text_to_speech"]);
        m
    });

/// Resolved tool names from the last `get_tool_definitions()` call.
pub fn last_resolved_tool_names() -> Vec<String> {
    last_resolved().lock().expect("last resolved lock").clone()
}

fn set_last_resolved(names: &[Value]) {
    let list: Vec<String> = names
        .iter()
        .filter_map(|t| t.get("function").and_then(|f| f.get("name")).and_then(Value::as_str))
        .map(|s| s.to_string())
        .collect();
    *last_resolved().lock().expect("last resolved lock") = list;
}

static LAST_RESOLVED: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
fn last_resolved() -> &'static Mutex<Vec<String>> {
    LAST_RESOLVED.get_or_init(|| Mutex::new(Vec::new()))
}

/// Simple FIFO-bound memo cache mirroring Python's insertion-ordered dict
/// eviction (pop next(iter(...)) == oldest).
#[derive(Default)]
struct ToolDefsCache {
    entries: std::collections::VecDeque<(Vec<u8>, Vec<Value>)>,
    max: usize,
}

impl ToolDefsCache {
    fn get(&mut self, key: &[u8]) -> Option<Vec<Value>> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }
    fn insert(&mut self, key: Vec<u8>, value: Vec<Value>) {
        while self.entries.len() >= self.max {
            self.entries.pop_front();
        }
        self.entries.push_back((key, value));
    }
}

static TOOL_DEFS_CACHE_MAX: usize = 8;
static TOOL_DEFS_CACHE: OnceLock<Mutex<ToolDefsCache>> = OnceLock::new();
fn defs_cache() -> &'static Mutex<ToolDefsCache> {
    TOOL_DEFS_CACHE.get_or_init(|| {
        Mutex::new(ToolDefsCache {
            entries: Default::default(),
            max: TOOL_DEFS_CACHE_MAX,
        })
    })
}

pub fn clear_tool_defs_cache() {
    defs_cache().lock().expect("cache lock").entries.clear();
}

/// Get tool definitions for model API calls with toolset-based filtering.
///
/// PARITY: model_tools.py get_tool_definitions @ b9aa928 (305–389)
#[allow(clippy::too_many_arguments)]
pub fn get_tool_definitions(
    enabled_toolsets: Option<&[String]>,
    disabled_toolsets: Option<&[String]>,
    quiet_mode: bool,
    skip_tool_search_assembly: bool,
    kanban_task_env: bool,
    delegated_child: bool,
    dispatcher_owned_worker: bool,
) -> Vec<Value> {
    // Fast path: memoized when quiet (no stdout side effects).
    if quiet_mode {
        let mut key_bytes: Vec<u8> = Vec::new();
        if let Some(ts) = enabled_toolsets {
            key_bytes.extend_from_slice(b"E:");
            for t in ts {
                key_bytes.extend_from_slice(t.as_bytes());
                key_bytes.push(0);
            }
        } else {
            key_bytes.extend_from_slice(b"E:null");
        }
        key_bytes.push(1);
        if let Some(ts) = disabled_toolsets {
            for t in ts {
                key_bytes.extend_from_slice(t.as_bytes());
                key_bytes.push(0);
            }
        } else {
            key_bytes.push(2);
        }
        key_bytes.push(3);
        key_bytes.extend_from_slice(&registry().generation_public().to_le_bytes());
        key_bytes.push(4);
        key_bytes.push(kanban_task_env as u8);
        key_bytes.push(skip_tool_search_assembly as u8);
        key_bytes.push(delegated_child as u8);
        key_bytes.push(dispatcher_owned_worker as u8);
        let mut cache = defs_cache().lock().expect("cache lock");
        if let Some(cached) = cache.get(&key_bytes) {
            *last_resolved().lock().expect("last resolved") = cached
                .iter()
                .filter_map(|t| t.get("function").and_then(|f| f.get("name")).and_then(Value::as_str))
                .map(|s| s.to_string())
                .collect();
            return cached;
        }
        drop(cache);

        let result = compute_tool_definitions(
            enabled_toolsets,
            disabled_toolsets,
            quiet_mode,
            skip_tool_search_assembly,
            kanban_task_env,
            delegated_child,
            dispatcher_owned_worker,
        );
        defs_cache()
            .lock()
            .expect("cache lock")
            .insert(key_bytes, result.clone());
        return result;
    }
    compute_tool_definitions(
        enabled_toolsets,
        disabled_toolsets,
        quiet_mode,
        skip_tool_search_assembly,
        kanban_task_env,
        delegated_child,
        dispatcher_owned_worker,
    )
}

/// Uncached implementation of `get_tool_definitions`.
///
/// PARITY: model_tools.py _compute_tool_definitions @ b9aa928 (391–612)
#[allow(clippy::too_many_arguments)]
pub fn compute_tool_definitions(
    enabled_toolsets: Option<&[String]>,
    disabled_toolsets: Option<&[String]>,
    quiet_mode: bool,
    _skip_tool_search_assembly: bool,
    kanban_task_env: bool,
    delegated_child: bool,
    dispatcher_owned_worker: bool,
) -> Vec<Value> {
    let mut tools_to_include: Vec<String> = Vec::new();

    if let Some(enabled) = enabled_toolsets {
        let mut effective: Vec<String> = enabled.to_vec();
        if kanban_task_env && !delegated_child && dispatcher_owned_worker && !effective.contains(&"kanban".to_string()) {
            effective.push("kanban".to_string());
        }
        for toolset_name in effective {
            if validate_toolset(&toolset_name) {
                let resolved = resolve_toolset(&toolset_name, None, true);
                extend_unique(&mut tools_to_include, &resolved);
                if !quiet_mode {
                    eprintln!(
                        "✅ Enabled toolset '{}': {}",
                        toolset_name,
                        if resolved.is_empty() { "no tools".into() } else { resolved.join(", ") }
                    );
                }
            } else if let Some(legacy) = LEGACY_TOOLSET_MAP.get(toolset_name.as_str()) {
                let legacy_owned: Vec<String> = legacy.iter().map(|s| s.to_string()).collect();
                extend_unique(&mut tools_to_include, &legacy_owned);
                if !quiet_mode {
                    eprintln!(
                        "✅ Enabled legacy toolset '{}': {}",
                        toolset_name,
                        legacy_owned.join(", ")
                    );
                }
            } else if !quiet_mode {
                eprintln!("⚠️  Unknown toolset: {toolset_name}");
            }
        }
    } else {
        // Default: start with everything.
        for ts_name in get_all_toolsets()
            .as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
        {
            let resolved = resolve_toolset(&ts_name, None, true);
            extend_unique(&mut tools_to_include, &resolved);
        }
    }

    // Always apply disabled toolsets as a subtraction step at the end.
    if let Some(disabled) = disabled_toolsets {
        for toolset_name in disabled {
            if validate_toolset(toolset_name) {
                let is_bundle = toolset_name.starts_with("hermes-");
                let posture = get_toolset(toolset_name, true)
                    .and_then(|t| t.get("posture").cloned())
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let to_remove: Vec<String> = if is_bundle || posture {
                    bundle_non_core_tools(toolset_name)
                        .into_iter()
                        .collect()
                } else {
                    resolve_toolset(toolset_name, None, true)
                };
                retain_not_in(&mut tools_to_include, &to_remove);
                let resolved_sorted = {
                    let mut v = to_remove.clone();
                    v.sort();
                    v
                };
                if !quiet_mode {
                    eprintln!(
                        "🚫 Disabled toolset '{}': {}",
                        toolset_name,
                        if resolved_sorted.is_empty() { "no tools".into() } else { resolved_sorted.join(", ") }
                    );
                }
            } else if let Some(legacy) = LEGACY_TOOLSET_MAP.get(toolset_name.as_str()) {
                let legacy_owned: Vec<String> = legacy.iter().map(|s| s.to_string()).collect();
                retain_not_in(&mut tools_to_include, &legacy_owned);
                if !quiet_mode {
                    eprintln!("🚫 Disabled legacy toolset '{}': {}", toolset_name, legacy_owned.join(", "));
                }
            } else if !quiet_mode {
                eprintln!("⚠️  Unknown toolset: {toolset_name}");
            }
        }
    }

    let toolset_refs: std::collections::HashSet<String> =
        tools_to_include.iter().cloned().collect();
    let mut filtered = registry().get_definitions(&toolset_refs, quiet_mode);

    // browser_navigate cross-reference strip when web tools are missing.
    let available_names: Vec<String> = filtered
        .iter()
        .filter_map(|t| t.get("function").and_then(|f| f.get("name")).and_then(Value::as_str))
        .map(|s| s.to_string())
        .collect();
    if available_names.iter().any(|n| n == "browser_navigate") {
        let web_available = available_names
            .iter()
            .any(|n| n == "web_search" || n == "web_extract");
        if !web_available {
            for td in filtered.iter_mut() {
                if td.get("function").and_then(|f| f.get("name")).and_then(Value::as_str) == Some("browser_navigate") {
                    let mut fn_obj = td.get("function").cloned().unwrap_or(Value::Null);
                    if let Some(obj) = fn_obj.as_object_mut() {
                        if let Some(Value::String(desc)) = obj.get("description") {
                            let stripped = desc.replace(
                                " For simple information retrieval, prefer web_search or web_extract (faster, cheaper).",
                                "",
                            );
                            obj.insert("description".to_string(), Value::String(stripped));
                        }
                    }
                    *td = json!({"type": "function", "function": fn_obj});
                    break;
                }
            }
        }
    }

    set_last_resolved(&filtered);
    if !quiet_mode {
        if filtered.is_empty() {
            eprintln!("🛠️  No tools selected (all filtered out or unavailable)");
        } else {
            let names: Vec<&str> = filtered
                .iter()
                .filter_map(|t| t.get("function").and_then(|f| f.get("name")).and_then(Value::as_str))
                .collect();
            eprintln!("🛠️  Final tool selection ({} tools): {}", filtered.len(), names.join(", "));
        }
    }
    filtered
}

fn extend_unique(dst: &mut Vec<String>, src: &[String]) {
    for s in src {
        if !dst.contains(s) {
            dst.push(s.clone());
        }
    }
}

fn retain_not_in(dst: &mut Vec<String>, remove: &[String]) {
    dst.retain(|s| !remove.contains(s));
}

// ── tool error sanitization ───────────────────────────────────────────────

static TOOL_ERROR_ROLE_TAG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)</?(?:tool_call|function_call|result|response|output|input|system|assistant|user)>")
        .expect("role tag re")
});
static TOOL_ERROR_FENCE_OPEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^\s*```(?:json|xml|html|markdown)?\s*").expect("fence open re")
});
static TOOL_ERROR_FENCE_CLOSE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)\s*```\s*$").expect("fence close re")
});
static TOOL_ERROR_CDATA_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?s)<!\[CDATA\[.*?\]\]>").expect("cdata re")
});
const TOOL_ERROR_MAX_LEN: usize = 2000;

/// Strip structural framing tokens from a tool error before showing it to
/// the model.
pub fn sanitize_tool_error(error_msg: &str) -> String {
    if error_msg.is_empty() {
        return "[TOOL_ERROR] ".to_string();
    }
    let mut sanitized = TOOL_ERROR_ROLE_TAG_RE.replace_all(error_msg, "").into_owned();
    sanitized = TOOL_ERROR_FENCE_OPEN_RE.replace_all(&sanitized, "").into_owned();
    sanitized = TOOL_ERROR_FENCE_CLOSE_RE.replace_all(&sanitized, "").into_owned();
    sanitized = TOOL_ERROR_CDATA_RE.replace_all(&sanitized, "").into_owned();
    if sanitized.chars().count() > TOOL_ERROR_MAX_LEN {
        let truncated: String = sanitized.chars().take(TOOL_ERROR_MAX_LEN - 3).collect();
        sanitized = format!("{truncated}...");
    }
    format!("[TOOL_ERROR] {sanitized}")
}

// ── argument coercion ─────────────────────────────────────────────────────

/// Coerce tool call arguments to match their JSON Schema types.
///
/// PARITY: model_tools.py coerce_tool_args @ b9aa928 (730–837)
pub fn coerce_tool_args(tool_name: &str, args: Value) -> Value {
    let Value::Object(mut args) = args else {
        return args;
    };
    if args.is_empty() {
        return Value::Object(args);
    }
    let Some(schema) = registry().get_schema(tool_name) else {
        return Value::Object(args);
    };
    // Note: upstream runs the schema_sanitizer unrename pass here; that
    // crate arrives with the sanitizer, and sanitized wire names only matter
    // once sanitization is active.
    let properties = schema
        .get("parameters")
        .and_then(|p| p.get("properties"))
        .and_then(Value::as_object)
        .cloned();
    let Some(properties) = properties else {
        return Value::Object(args);
    };

    let keys: Vec<String> = args.keys().cloned().collect();
    for key in keys {
        let Some(prop_schema) = properties.get(&key) else { continue };
        let expected = prop_schema.get("type").cloned();
        let value = args.get(&key).cloned().unwrap_or(Value::Null);
        let is_str = matches!(value, Value::String(_));
        let is_nullable_str = is_str;
        let expected_str = match &expected {
            Some(Value::String(s)) => Some(s.clone()),
            Some(Value::Array(_)) => None, // union handled in _coerce_value
            _ => None,
        };
        let expected_kind = expected_str.as_deref();

        // Wrap bare non-list values when the schema declares array.
        if expected_kind == Some("array") && !matches!(value, Value::Null | Value::Array(_)) {
            if is_nullable_str {
                let expected_ref: &Value = expected.as_ref().unwrap_or(&Value::Null);
                let coerced = coerce_value(value.clone(), expected_ref, Some(prop_schema));
                // Track whether coercion changed the value (string -> parsed).
                let changed = !matches!((&value, &coerced), (Value::String(_), Value::String(_)));
                if changed {
                    let _ = args.insert(key.clone(), coerced);
                    continue;
                }
                if matches!(&value, Value::String(s) if s.trim_start().starts_with('[')) {
                    eprintln!(
                        "[hermes-tools] WARN: coerce_tool_args: {tool_name}.{key} looks like a JSON array string but could not be parsed"
                    );
                }
                let _ = args.insert(key.clone(), Value::Array(vec![value.clone()]));
                continue;
            }
            let _ = args.insert(key.clone(), Value::Array(vec![value]));
            continue;
        }

        if !is_str {
            // Recurse into already-native containers for JSON-encoded
            // elements / sub-fields.
            if (expected_kind == Some("array") && matches!(value, Value::Array(_)))
                || (expected_kind == Some("object") && matches!(value, Value::Object(_)))
            {
                let normalized = normalize_json_strings_for_schema(value, prop_schema);
                let _ = args.insert(key.clone(), normalized);
            }
            continue;
        }
        if expected.is_none() && !schema_allows_null(Some(prop_schema)) {
            continue;
        }
        if expected.is_none() {
            continue;
        }
        let expected_ref: &Value = expected.as_ref().unwrap_or(&Value::Null);
        let coerced = coerce_value(value, expected_ref, Some(prop_schema));
        match (&args.get(&key), &coerced) {
            (Some(Value::String(_)), Value::String(_)) => {}
            _ => {
                let is_container = matches!(coerced, Value::Array(_) | Value::Object(_));
                let _ = args.insert(key.clone(), coerced.clone());
                if is_container {
                    let normalized = normalize_json_strings_for_schema(coerced, prop_schema);
                    let _ = args.insert(key.clone(), normalized);
                }
            }
        }
    }
    Value::Object(args)
}

/// True when *schema* permits a value of JSON type `kind` (array/object).
fn schema_accepts_kind(schema: &Value, kind: &str) -> bool {
    let Some(obj) = schema.as_object() else { return false };
    match obj.get("type") {
        Some(Value::String(s)) if s == kind => return true,
        Some(Value::Array(a)) if a.iter().any(|v| v.as_str() == Some(kind)) => return true,
        _ => {}
    }
    for union_key in ["anyOf", "oneOf", "allOf"] {
        if let Some(Value::Array(branches)) = obj.get(union_key) {
            if branches.iter().any(|b| schema_accepts_kind(b, kind)) {
                return true;
            }
        }
    }
    false
}

/// Recursively parse JSON-encoded string values that a schema expects to be
/// arrays or objects.
fn normalize_json_strings_for_schema(value: Value, schema: &Value) -> Value {
    if !schema.is_object() {
        return value;
    }
    if let Value::String(s) = &value {
        let trimmed = s.trim();
        let expects_array = schema_accepts_kind(schema, "array");
        let expects_object = schema_accepts_kind(schema, "object");
        if (expects_array && trimmed.starts_with('['))
            || (expects_object && trimmed.starts_with('{'))
        {
            if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                if expects_array && parsed.is_array() {
                    return normalize_json_strings_for_schema(parsed, schema);
                }
                if expects_object && parsed.is_object() {
                    return normalize_json_strings_for_schema(parsed, schema);
                }
            }
        }
        return value;
    }

    if let Value::Array(items) = &value {
        let Some(items_schema) = schema.get("items") else {
            return value;
        };
        if !items_schema.is_object() {
            return value;
        }
        let changed = items
            .iter()
            .any(|it| matches!(it, Value::String(_)) && container_like(it, items_schema));
        if !changed {
            return value;
        }
        let out: Vec<Value> = items
            .iter()
            .map(|it| normalize_json_strings_for_schema(it.clone(), items_schema))
            .collect();
        return Value::Array(out);
    }

    if let Value::Object(map) = &value {
        let Some(props) = schema.get("properties").and_then(Value::as_object) else {
            return value;
        };
        let mut changed = false;
        let mut out: Map<String, Value> = Map::new();
        for (k, v) in map {
            if let Some(prop_schema) = props.get(k) {
                let nxt = normalize_json_strings_for_schema(v.clone(), prop_schema);
                changed = changed || nxt != *v;
                out.insert(k.clone(), nxt);
            } else {
                out.insert(k.clone(), v.clone());
            }
        }
        if changed {
            Value::Object(out)
        } else {
            value
        }
    } else {
        value
    }
}

fn container_like(v: &Value, schema: &Value) -> bool {
    let Value::String(s) = v else { return false };
    let t = s.trim();
    (schema_accepts_kind(schema, "array") && t.starts_with('['))
        || (schema_accepts_kind(schema, "object") && t.starts_with('{'))
}

/// Attempt to coerce a string *value* to *expected_type*. Returns the
/// original string when coercion is not applicable or fails.
fn coerce_value(value: Value, expected: &Value, schema: Option<&Value>) -> Value {
    let Value::String(s) = &value else {
        return value;
    };
    if schema_allows_null(schema) && s.trim().to_lowercase() == "null" {
        return Value::Null;
    }
    match expected {
        Value::Array(types) => {
            for t in types {
                let result = coerce_value(value.clone(), t, schema);
                if !same_json_string(&value, &result) {
                    return result;
                }
            }
            value
        }
        Value::String(kind) => match kind.as_str() {
            "integer" => coerce_number(s, true),
            "number" => coerce_number(s, false),
            "boolean" => coerce_boolean(s),
            "array" => coerce_json(s, true),
            "object" => coerce_json_object(s),
            "null" if s.trim().to_lowercase() == "null" => Value::Null,
            _ => value,
        },
        _ => value,
    }
}

fn same_json_string(a: &Value, b: &Value) -> bool {
    matches!((a, b), (Value::String(x), Value::String(y)) if x == y)
}

/// True when a JSON Schema fragment explicitly permits null.
fn schema_allows_null(schema: Option<&Value>) -> bool {
    let Some(schema) = schema else { return false };
    let Some(obj) = schema.as_object() else { return false };
    if let Some(Value::String(s)) = obj.get("type") {
        if s == "null" {
            return true;
        }
    }
    if let Some(Value::Array(a)) = obj.get("type") {
        if a.iter().any(|v| v.as_str() == Some("null")) {
            return true;
        }
    }
    if obj.get("nullable").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    for union_key in ["anyOf", "oneOf"] {
        if let Some(Value::Array(variants)) = obj.get(union_key) {
            for variant in variants {
                if variant.get("type").and_then(Value::as_str) == Some("null") {
                    return true;
                }
            }
        }
    }
    false
}

/// Parse *value* as JSON when the schema expects an array.
fn coerce_json(value: &str, _expects_array: bool) -> Value {
    match serde_json::from_str::<Value>(value) {
        Ok(parsed @ Value::Array(_)) => parsed,
        Ok(_) => Value::String(value.to_string()),
        Err(_) => Value::String(value.to_string()),
    }
}

/// Parse *value* as JSON when the schema expects an object.
fn coerce_json_object(value: &str) -> Value {
    match serde_json::from_str::<Value>(value) {
        Ok(parsed @ Value::Object(_)) => parsed,
        Ok(_) => Value::String(value.to_string()),
        Err(_) => Value::String(value.to_string()),
    }
}

/// Try to parse *value* as a number. Returns original string on failure;
/// integer_only rejects decimals.
fn coerce_number(value: &str, integer_only: bool) -> Value {
    let Ok(f) = value.parse::<f64>() else {
        return Value::String(value.to_string());
    };
    // Guard against inf/nan (not JSON-serializable).
    if f.is_nan() || f.is_infinite() {
        return Value::String(value.to_string());
    }
    if f == f.trunc() {
        return json!(f as i64);
    }
    if integer_only {
        return Value::String(value.to_string());
    }
    json!(f)
}

/// Try to parse *value* as a boolean. Returns original string on failure.
fn coerce_boolean(value: &str) -> Value {
    let low = value.trim().to_lowercase();
    if low == "true" {
        return Value::Bool(true);
    }
    if low == "false" {
        return Value::Bool(false);
    }
    Value::String(value.to_string())
}

// ── public shims (delegating to the registry) ─────────────────────────────

pub fn get_all_tool_names() -> Vec<String> {
    registry().get_all_tool_names()
}

pub fn get_toolset_for_tool(tool_name: &str) -> Option<String> {
    registry().get_toolset_for_tool(tool_name)
}

pub fn get_available_toolsets() -> HashMap<String, Value> {
    registry().get_available_toolsets()
}

pub fn check_toolset_requirements() -> HashMap<String, bool> {
    registry().check_toolset_requirements()
}

pub fn check_tool_availability(quiet: bool) -> (Vec<String>, Vec<Value>) {
    registry().check_tool_availability(quiet)
}
