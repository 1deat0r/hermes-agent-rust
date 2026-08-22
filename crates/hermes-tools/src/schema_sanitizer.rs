//! Tool schema sanitization for strict backend compatibility.
//!
//! PARITY: tools/schema_sanitizer.py @ b9aa928 (687 LOC, ported 1:1).
//!
//! - Property keys are renamed to the `[a-zA-Z0-9_.-]{1,64}` pattern
//!   (Anthropic/Bedrock/Vertex/Azure reject keys like `issue_class~neq`).
//! - Bare-string schema values become dict schemas; object nodes get empty
//!   `properties`.
//! - `type: [X, "null"]` arrays collapse to `type: X` with a `nullable: true`
//!   hint; multi-type arrays become `anyOf` (no branch dropped).
//! - Nullable `anyOf`/`oneOf` unions collapse; const unions become `enum`.
//! - Top-level combinators and `$ref` siblings are stripped for strict
//!   backends (Codex, Fireworks).
//! - Reactive strips: llama.cpp `pattern`/`format` (on grammar rejection),
//!   xAI `enum`-containing-slash.

use serde_json::{json, Map, Value};

/// `[a-zA-Z0-9_.-]{1,64}`
fn prop_key_re(key: &str) -> bool {
    if key.is_empty() || key.chars().count() > 64 {
        return false;
    }
    key.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Deterministically map an arbitrary property key to a conforming one.
pub fn sanitize_property_key(key: &str) -> String {
    let replaced: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    if replaced.is_empty() {
        "param".to_string()
    } else {
        replaced
    }
}

/// `{original_key: conforming_key}` for one properties dict (identity
/// entries omitted; collisions get numeric suffixes).
fn rename_property_keys(props: &Map<String, Value>) -> std::collections::HashMap<String, String> {
    let mut renames: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut taken: Vec<String> = props
        .keys()
        .filter(|k| prop_key_re(k))
        .cloned()
        .collect();
    for key in props.keys() {
        if prop_key_re(key) {
            continue;
        }
        let base = sanitize_property_key(key);
        let mut candidate = base.clone();
        let mut i = 2;
        while taken.contains(&candidate) {
            let suffix = format!("_{i}");
            let limit = 64usize.saturating_sub(suffix.chars().count());
            let prefix: String = base.chars().take(limit).collect();
            candidate = format!("{prefix}{suffix}");
            i += 1;
        }
        taken.push(candidate.clone());
        renames.insert(key.clone(), candidate);
    }
    renames
}

/// Map sanitized property keys in model-emitted args back to wire names.
pub fn unrename_tool_args(params_schema: &Value, args: &Value) -> Value {
    let Value::Object(params) = params_schema else { return args.clone() };
    let Value::Object(args) = args else { return args.clone() };
    let Some(props_schema) = params.get("properties").and_then(Value::as_object) else {
        return Value::Object(args.clone());
    };
    let renames = rename_property_keys(props_schema);
    let reverse: std::collections::HashMap<String, String> = renames
        .iter()
        .map(|(k, v)| (v.clone(), k.clone()))
        .collect();
    let mut out = Map::new();
    for (key, value) in args {
        let orig = reverse.get(key).cloned().unwrap_or_else(|| key.clone());
        let subschema = props_schema.get(&orig);
        let new_value = match subschema {
            Some(s @ Value::Object(_)) => {
                if let Value::Object(_) = value {
                    unrename_tool_args(s, value)
                } else if let (Value::Array(items), Some(Value::Object(items_schema))) =
                    (value, s.get("items"))
                {
                    let items_value = &Value::Object(items_schema.clone());
                    Value::Array(
                        items
                            .iter()
                            .map(|it| {
                                if it.is_object() {
                                    unrename_tool_args(items_value, it)
                                } else {
                                    it.clone()
                                }
                            })
                            .collect(),
                    )
                } else {
                    value.clone()
                }
            }
            _ => value.clone(),
        };
        out.insert(orig, new_value);
    }
    Value::Object(out)
}

fn deep_clone(v: &Value) -> Value {
    v.clone()
}

/// Return a copy of `tools` with each tool's parameter schema sanitized.
pub fn sanitize_tool_schemas(tools: &[Value]) -> Vec<Value> {
    if tools.is_empty() {
        return tools.to_vec();
    }
    tools.iter().map(sanitize_single_tool).collect()
}

fn sanitize_single_tool(tool: &Value) -> Value {
    let mut out = deep_clone(tool);
    let Some(fn_obj) = out.get_mut("function").and_then(Value::as_object_mut) else {
        return out;
    };
    let name = fn_obj.get("name").and_then(Value::as_str).unwrap_or("<tool>").to_string();
    let params = fn_obj.get("parameters").cloned();
    match params {
        Some(Value::Object(_)) => {
            if let Some(p) = fn_obj.get_mut("parameters") {
                *p = sanitize_node(p, &name);
            }
        }
        _ => {
            fn_obj.insert(
                "parameters".to_string(),
                json!({"type": "object", "properties": {}}),
            );
            return out;
        }
    }
    let top = fn_obj.get_mut("parameters").cloned().unwrap_or(Value::Null);
    let top = match top {
        Value::Object(_) => top,
        _ => json!({"type": "object", "properties": {}}),
    };
    let mut top = top;
    if let Some(t) = top.as_object_mut() {
        if t.get("type").and_then(Value::as_str) != Some("object") {
            t.insert("type".to_string(), json!("object"));
        }
        if !t.get("properties").is_some_and(Value::is_object) {
            t.insert("properties".to_string(), json!({}));
        }
    }
    // Final passes: nullable unions, top-level combinators, ref siblings.
    top = strip_nullable_unions(&top, true);
    top = strip_top_level_combinators(&top, &name);
    top = strip_ref_siblings(&top);
    if let Some(p) = fn_obj.get_mut("parameters") {
        *p = top;
    }
    out
}

/// Drop forbidden sibling keywords from nodes that carry `$ref`.
fn strip_ref_siblings(node: &Value) -> Value {
    match node {
        Value::Array(items) => Value::Array(items.iter().map(strip_ref_siblings).collect()),
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), strip_ref_siblings(v));
            }
            if out.contains_key("$ref") {
                {
                    let key = "default";
                    out.remove(key);
                }
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

const TOP_LEVEL_FORBIDDEN_KEYS: [&str; 5] = ["allOf", "anyOf", "oneOf", "enum", "not"];

/// Drop combinator keywords from the top level of a function parameters
/// schema (OpenAI Codex backend).
fn strip_top_level_combinators(params: &Value, _path: &str) -> Value {
    let Value::Object(map) = params else { return params.clone() };
    let mut out = map.clone();
    for key in TOP_LEVEL_FORBIDDEN_KEYS {
        out.remove(key);
    }
    Value::Object(out)
}

/// Collapse `anyOf`/`oneOf` nullable unions to the non-null branch.
pub fn strip_nullable_unions(schema: &Value, keep_nullable_hint: bool) -> Value {
    match schema {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|it| strip_nullable_unions(it, keep_nullable_hint))
                .collect(),
        ),
        Value::Object(map) => {
            let mut stripped = Map::new();
            for (k, v) in map {
                stripped.insert(k.clone(), strip_nullable_unions(v, keep_nullable_hint));
            }
            for key in ["anyOf", "oneOf"] {
                let Some(Value::Array(variants)) = stripped.get(key) else { continue };
                let non_null: Vec<&Value> = variants
                    .iter()
                    .filter(|item| {
                        !(item.is_object()
                            && item.get("type").and_then(Value::as_str) == Some("null"))
                    })
                    .collect();
                if non_null.len() == 1 && non_null.len() != variants.len() {
                    let mut replacement = match non_null[0] {
                        Value::Object(o) => o.clone(),
                        _ => Map::new(),
                    };
                    if keep_nullable_hint && !replacement.contains_key("nullable") {
                        replacement.insert("nullable".to_string(), json!(true));
                    }
                    for meta_key in ["title", "description", "default", "examples"] {
                        if !stripped.contains_key(meta_key) || replacement.contains_key(meta_key) {
                            continue;
                        }
                        if meta_key == "default" && replacement.contains_key("$ref") {
                            continue;
                        }
                        replacement.insert(
                            meta_key.to_string(),
                            stripped.get(meta_key).cloned().unwrap_or(Value::Null),
                        );
                    }
                    return strip_nullable_unions(&Value::Object(replacement), keep_nullable_hint);
                }
            }
            Value::Object(stripped)
        }
        other => other.clone(),
    }
}

/// JSON-Schema primitive type of a pure `const` branch (bool before int).
fn const_branch_type(branch: &Value) -> Option<&'static str> {
    let Value::Object(obj) = branch else { return None };
    if !obj.contains_key("const") {
        return None;
    }
    let extra: Vec<&String> = obj
        .keys()
        .filter(|k| k.as_str() != "const" && k.as_str() != "type" && k.as_str() != "title" && k.as_str() != "description")
        .collect();
    if !extra.is_empty() {
        return None;
    }
    let value = obj.get("const").unwrap();
    let json_type = match value {
        Value::Bool(_) => Some("boolean"),
        Value::Number(n) if n.is_i64() => Some("integer"),
        Value::Number(_) => Some("number"),
        Value::String(_) => Some("string"),
        _ => None,
    };
    let json_type = json_type?;
    let declared = obj.get("type").and_then(Value::as_str);
    if declared.is_some() && declared != Some(json_type) {
        return None;
    }
    Some(json_type)
}

/// Collapse `anyOf`/`oneOf` unions of same-typed consts to `enum`.
pub fn collapse_const_unions(schema: &Value) -> Value {
    match schema {
        Value::Array(items) => Value::Array(items.iter().map(collapse_const_unions).collect()),
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), collapse_const_unions(v));
            }
            for key in ["anyOf", "oneOf"] {
                let Some(Value::Array(variants)) = out.get(key) else { continue };
                if variants.is_empty() {
                    continue;
                }
                let null_branches: Vec<&Value> = variants
                    .iter()
                    .filter(|item| {
                        item.is_object()
                            && item.get("type").and_then(Value::as_str) == Some("null")
                            && !item.get("const").is_some()
                    })
                    .collect();
                let const_branches: Vec<&Value> = variants
                    .iter()
                    .filter(|item| !null_branches.contains(item))
                    .collect();
                if null_branches.len() > 1 || const_branches.is_empty() {
                    continue;
                }
                let branch_types: std::collections::HashSet<Option<&'static str>> =
                    const_branches.iter().map(|b| const_branch_type(b)).collect();
                if branch_types.len() != 1 || branch_types.contains(&None) {
                    continue;
                }
                let json_type = branch_types.into_iter().next().unwrap().unwrap();
                let enum_values: Vec<Value> = const_branches
                    .iter()
                    .filter_map(|b| b.get("const").cloned())
                    .collect();
                let mut replacement = Map::new();
                replacement.insert("type".to_string(), json!(json_type));
                replacement.insert("enum".to_string(), Value::Array(enum_values));
                if !null_branches.is_empty() {
                    replacement.insert("nullable".to_string(), json!(true));
                }
                for meta_key in ["title", "description", "default", "examples"] {
                    if !out.contains_key(meta_key) || replacement.contains_key(meta_key) {
                        continue;
                    }
                    replacement.insert(
                        meta_key.to_string(),
                        out.get(meta_key).cloned().unwrap_or(Value::Null),
                    );
                }
                return Value::Object(replacement);
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// Recursively sanitize a JSON-Schema fragment.
fn sanitize_node(node: &Value, path: &str) -> Value {
    match node {
        Value::String(s) => {
            if matches!(
                s.as_str(),
                "object" | "string" | "number" | "integer" | "boolean" | "array" | "null"
            ) {
                if s == "object" {
                    json!({"type": "object", "properties": {}})
                } else {
                    json!({"type": s})
                }
            } else {
                json!({"type": "object", "properties": {}})
            }
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                out.push(sanitize_node(item, &format!("{path}[{i}]")));
            }
            Value::Array(out)
        }
        Value::Object(map) => {
            // Compute property-key renames up front.
            let prop_renames = map
                .get("properties")
                .and_then(Value::as_object)
                .map(rename_property_keys)
                .unwrap_or_default();
            let mut out = Map::new();
            for (key, value) in map {
                if key == "type" {
                    if let Value::Array(types) = value {
                        let has_null = types.iter().any(|t| t.as_str() == Some("null"));
                        let non_null: Vec<&str> = types
                            .iter()
                            .filter_map(|t| t.as_str())
                            .filter(|t| *t != "null")
                            .collect();
                        if non_null.len() == 1 {
                            out.insert("type".to_string(), json!(non_null[0]));
                            if has_null {
                                out.entry("nullable".to_string()).or_insert(json!(true));
                            }
                            continue;
                        }
                        if non_null.len() >= 2 {
                            out.insert(
                                "anyOf".to_string(),
                                Value::Array(
                                    non_null
                                        .iter()
                                        .map(|t| json!({"type": t}))
                                        .collect(),
                                ),
                            );
                            if has_null {
                                out.entry("nullable".to_string()).or_insert(json!(true));
                            }
                            continue;
                        }
                        out.insert(
                            "type".to_string(),
                            json!(if has_null { "null" } else { "object" }),
                        );
                        continue;
                    }
                }
                if matches!(key.as_str(), "properties" | "$defs" | "definitions")
                    && value.is_object()
                {
                    let empty_renames: std::collections::HashMap<String, String> = Default::default();
                    let renames_here = if key == "properties" { &prop_renames } else { &empty_renames };
                    let mut new_props = Map::new();
                    if let Some(props) = value.as_object() {
                        for (sub_k, sub_v) in props {
                            let out_k = renames_here.get(sub_k).cloned().unwrap_or_else(|| sub_k.clone());
                            let sub_path = format!("{path}.{key}.{out_k}");
                            new_props.insert(out_k, sanitize_node(sub_v, &sub_path));
                        }
                    }
                    out.insert(key.clone(), Value::Object(new_props));
                } else if matches!(key.as_str(), "items" | "additionalProperties") {
                    if value.is_boolean() {
                        out.insert(key.clone(), value.clone());
                    } else {
                        out.insert(key.clone(), sanitize_node(value, &format!("{path}.{key}")));
                    }
                } else if matches!(key.as_str(), "anyOf" | "oneOf" | "allOf") && value.is_array() {
                    let items = value.as_array().unwrap();
                    let mut arr = Vec::with_capacity(items.len());
                    for (i, item) in items.iter().enumerate() {
                        arr.push(sanitize_node(item, &format!("{path}.{key}[{i}]")));
                    }
                    out.insert(key.clone(), Value::Array(arr));
                } else if matches!(key.as_str(), "required" | "enum" | "examples" | "dependentRequired") {
                    if key == "required" && !prop_renames.is_empty() {
                        if let Value::Array(reqs) = value {
                            out.insert(
                                key.clone(),
                                Value::Array(
                                    reqs.iter()
                                        .map(|r| {
                                            r.as_str()
                                                .map(|s| {
                                                    prop_renames
                                                        .get(s)
                                                        .cloned()
                                                        .unwrap_or_else(|| s.to_string())
                                                })
                                                .map(Value::String)
                                                .unwrap_or_else(|| r.clone())
                                        })
                                        .collect(),
                                ),
                            );
                        }
                    } else {
                        out.insert(key.clone(), deep_clone(value));
                    }
                } else {
                    out.insert(
                        key.clone(),
                        if value.is_object() || value.is_array() {
                            sanitize_node(value, &format!("{path}.{key}"))
                        } else {
                            value.clone()
                        },
                    );
                }
            }
            // Object nodes without properties: inject empty properties dict.
            if out.get("type").and_then(Value::as_str) == Some("object")
                && !out.get("properties").is_some_and(Value::is_object)
            {
                out.insert("properties".to_string(), json!({}));
            }
            // Prune required entries that don't exist in properties.
            if out.get("type").and_then(Value::as_str) == Some("object") {
                if let Some(Value::Array(reqs)) = out.get("required").cloned() {
                    let props = out.get("properties").and_then(Value::as_object).cloned().unwrap_or_default();
                    let valid: Vec<Value> = reqs
                        .iter()
                        .filter(|r| r.as_str().map(|s| props.contains_key(s)).unwrap_or(false))
                        .cloned()
                        .collect();
                    if valid.is_empty() {
                        out.remove("required");
                    } else if valid.len() != reqs.len() {
                        out.insert("required".to_string(), Value::Array(valid));
                    }
                }
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// Reactive strip: remove `pattern`/`format` keywords for llama.cpp recovery.
pub fn strip_pattern_and_format(tools: &mut [Value]) -> i64 {
    let mut stripped: i64 = 0;

    fn walk(node: &mut Value, stripped: &mut i64) {
        match node {
            Value::Object(map) => {
                let is_schema_node = map.contains_key("type")
                    || map.contains_key("anyOf")
                    || map.contains_key("oneOf")
                    || map.contains_key("allOf");
                let keys: Vec<String> = map.keys().cloned().collect();
                for key in keys {
                    if is_schema_node && (key == "pattern" || key == "format") {
                        map.remove(&key);
                        *stripped += 1;
                        continue;
                    }
                    if let Some(v) = map.get_mut(&key) {
                        walk(v, stripped);
                    }
                }
            }
            Value::Array(items) => {
                for item in items {
                    walk(item, stripped);
                }
            }
            _ => {}
        }
    }

    for tool in tools.iter_mut() {
        if !tool.is_object() {
            continue;
        }
        let mut any = false;
        if let Some(fn_obj) = tool.get_mut("function").and_then(Value::as_object_mut) {
            if let Some(Value::Object(_)) = fn_obj.get("parameters") {
                if let Some(p) = fn_obj.get_mut("parameters") {
                    walk(p, &mut stripped);
                    any = true;
                }
            }
        }
        if !any {
            if let Some(Value::Object(_)) = tool.get("parameters") {
                if let Some(p) = tool.get_mut("parameters") {
                    walk(p, &mut stripped);
                }
            }
        }
    }
    stripped
}

/// Reactive strip: remove `enum` keywords whose values contain a slash (xAI).
pub fn strip_slash_enum(tools: &mut [Value]) -> i64 {
    let mut stripped: i64 = 0;

    fn walk(node: &mut Value, stripped: &mut i64) {
        match node {
            Value::Object(map) => {
                if let Some(Value::Array(enum_vals)) = map.get("enum") {
                    let has_slash = enum_vals
                        .iter()
                        .any(|v| v.as_str().map(|s| s.contains('/')).unwrap_or(false));
                    if has_slash {
                        map.remove("enum");
                        *stripped += 1;
                    }
                }
                let keys: Vec<String> = map.keys().cloned().collect();
                for key in keys {
                    if let Some(v) = map.get_mut(&key) {
                        walk(v, stripped);
                    }
                }
            }
            Value::Array(items) => {
                for item in items {
                    walk(item, stripped);
                }
            }
            _ => {}
        }
    }

    for tool in tools.iter_mut() {
        if !tool.is_object() {
            continue;
        }
        let mut any = false;
        if let Some(fn_obj) = tool.get_mut("function").and_then(Value::as_object_mut) {
            if let Some(Value::Object(_)) = fn_obj.get("parameters") {
                if let Some(p) = fn_obj.get_mut("parameters") {
                    walk(p, &mut stripped);
                    any = true;
                }
            }
        }
        if !any {
            if let Some(Value::Object(_)) = tool.get("parameters") {
                if let Some(p) = tool.get_mut("parameters") {
                    walk(p, &mut stripped);
                }
            }
        }
    }
    stripped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_without_properties_gets_empty_properties() {
        let out = sanitize_tool_schemas(&[json!({
            "type": "function",
            "function": {"name": "t", "parameters": {"type": "object"}},
        })]);
        assert_eq!(out[0]["function"]["parameters"]["properties"], json!({}));
    }

    #[test]
    fn bare_string_schema_replaced() {
        let out = sanitize_tool_schemas(&[json!({
            "type": "function",
            "function": {"name": "t", "parameters": {"type": "object", "properties": {"x": "string"}}},
        })]);
        assert_eq!(out[0]["function"]["parameters"]["properties"]["x"], json!({"type": "string"}));
    }

    #[test]
    fn nullable_type_array_collapsed() {
        let out = sanitize_tool_schemas(&[json!({
            "type": "function",
            "function": {"name": "t", "parameters": {"type": "object", "properties": {"x": {"type": ["string", "null"]}}}},
        })]);
        let x = &out[0]["function"]["parameters"]["properties"]["x"];
        assert_eq!(x["type"], json!("string"));
        assert_eq!(x["nullable"], json!(true));
    }

    #[test]
    fn multitype_array_becomes_anyof() {
        let out = sanitize_tool_schemas(&[json!({
            "type": "function",
            "function": {"name": "t", "parameters": {"type": "object", "properties": {"x": {"type": ["number", "string"]}}}},
        })]);
        let x = &out[0]["function"]["parameters"]["properties"]["x"];
        assert_eq!(x["anyOf"], json!([{"type": "number"}, {"type": "string"}]));
    }

    #[test]
    fn required_pruned_to_existing_properties() {
        let out = sanitize_tool_schemas(&[json!({
            "type": "function",
            "function": {"name": "t", "parameters": {"type": "object", "properties": {"a": {"type": "string"}}, "required": ["a", "b"]}},
        })]);
        assert_eq!(out[0]["function"]["parameters"]["required"], json!(["a"]));
    }

    #[test]
    fn missing_parameters_gets_default_schema() {
        let out = sanitize_tool_schemas(&[json!({"function": {"name": "t"}})]);
        assert_eq!(out[0]["function"]["parameters"], json!({"type": "object", "properties": {}}));
    }

    #[test]
    fn property_key_renamed_and_unrenamed() {
        let schema = json!({
            "type": "object",
            "properties": {"issue_class~neq": {"type": "string"}},
            "required": ["issue_class~neq"],
        });
        let sanitized = sanitize_tool_schemas(&[json!({"function": {"name": "t", "parameters": schema.clone()}})]);
        let params = &sanitized[0]["function"]["parameters"];
        let keys: Vec<&str> = params["properties"].as_object().unwrap().keys().map(|k| k.as_str()).collect();
        // Original key renamed to the conforming pattern.
        assert!(!keys.contains(&"issue_class~neq"));
        let renamed_key = keys[0];
        assert!(prop_key_re(renamed_key));
        // required remapped to the renamed key.
        assert_eq!(params["required"][0], json!(renamed_key));
        // unrename maps args back to the wire name.
        let args = json!({renamed_key: "v"});
        let out = unrename_tool_args(&schema, &args);
        assert_eq!(out.get("issue_class~neq").and_then(Value::as_str), Some("v"));
    }

    #[test]
    fn const_union_collapses_to_enum() {
        // collapse_const_unions is used by the MCP ingestion path (not the
        // sanitize_tool_schemas pipeline); test it directly.
        let out = collapse_const_unions(&json!({
            "anyOf": [{"const": "red"}, {"const": "green"}]
        }));
        assert_eq!(out["type"], json!("string"));
        assert_eq!(out["enum"], json!(["red", "green"]));
    }

    #[test]
    fn test_strip_pattern_and_format() {
        let mut tools = vec![json!({
            "type": "function",
            "function": {"name": "t", "parameters": {"type": "object", "properties": {"x": {"type": "string", "pattern": "\\d+", "format": "date-time"}}}},
        })];
        let n = strip_pattern_and_format(&mut tools);
        assert_eq!(n, 2);
        let x = &tools[0]["function"]["parameters"]["properties"]["x"];
        assert!(x.get("pattern").is_none());
        assert!(x.get("format").is_none());
    }

    #[test]
    fn test_strip_slash_enum() {
        let mut tools = vec![json!({
            "type": "function",
            "function": {"name": "t", "parameters": {"type": "object", "properties": {"model": {"type": "string", "enum": ["Qwen/Qwen3"]}}}},
        })];
        let n = strip_slash_enum(&mut tools);
        assert_eq!(n, 1);
        let model = &tools[0]["function"]["parameters"]["properties"]["model"];
        assert!(model.get("enum").is_none());
    }

    #[test]
    fn well_formed_schema_unchanged() {
        let input = json!({
            "type": "function",
            "function": {"name": "echo", "parameters": {"type": "object", "properties": {"v": {"type": "string"}}}},
        });
        let out = sanitize_tool_schemas(std::slice::from_ref(&input));
        assert_eq!(out[0], input);
    }
}
