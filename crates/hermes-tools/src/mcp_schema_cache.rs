//! Persistent MCP tool-schema cache for lazy server startup.
//!
//! PARITY: tools/mcp_schema_cache.py @ b9aa928 (121 LOC, ported 1:1). Stores
//! per-server tool manifests on disk so MCP tools can be registered into the
//! agent snapshot without spawning the stdio child at idle dashboard startup.
//! Entries are keyed by server name + a fingerprint of the connection config.

use std::path::PathBuf;
use std::sync::Mutex;

use hermes_utils::atomic::atomic_json_write;
use once_cell::sync::Lazy;
use serde_json::{json, Value};

const CACHE_FILENAME: &str = "mcp_schema_cache.json";

/// Cache access is serialized like upstream's `_cache_lock`.
static CACHE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn cache_path() -> PathBuf {
    hermes_constants::home::get_hermes_home().join("cache").join(CACHE_FILENAME)
}

/// Stable hash of the connection-defining parts of an MCP server config.
pub fn config_fingerprint(config: &Value) -> String {
    let tools_filter = config.get("tools").filter(|v| !v.is_null()).cloned().unwrap_or_default();
    let tools_filter = if tools_filter.is_object() { tools_filter } else { Value::Object(Default::default()) };
    let payload = json!({
        "command": config.get("command").filter(|v| !v.is_null()),
        "args": config.get("args").and_then(|v| v.as_array()).cloned().unwrap_or_default(),
        "url": config.get("url").filter(|v| !v.is_null()),
        "transport": config.get("transport").filter(|v| !v.is_null()),
        "tools_include": sorted_strings(&tools_filter, "include"),
        "tools_exclude": sorted_strings(&tools_filter, "exclude"),
    });
    // Python json.dumps(sort_keys=True, separators=(",", ":")) then sha256.
    let mut items: Vec<(String, Value)> = Vec::new();
    if let Value::Object(map) = payload {
        items = map.into_iter().collect();
    }
    items.sort_by(|a, b| a.0.cmp(&b.0));
    let mut raw = String::new();
    raw.push('{');
    for (i, (k, v)) in items.iter().enumerate() {
        if i > 0 {
            raw.push(',');
        }
        raw.push('"');
        raw.push_str(&json_escape(k));
        raw.push('"');
        raw.push(':');
        raw.push_str(&compact_json(v));
    }
    raw.push('}');
    let digest = sha256_hex(raw.as_bytes());
    digest[..16].to_string()
}

fn sorted_strings(v: &Value, key: &str) -> Vec<String> {
    let mut out: Vec<String> = v
        .get(key)
        .and_then(|x| x.as_array())
        .map(|arr| arr.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    out.sort();
    out
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// Serialize like `json.dumps(v, ensure_ascii=True, separators=(",", ":"))`
/// (non-ASCII → \uXXXX escapes, compact separators, no spaces).
fn compact_json(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => (*b).to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            let mut out = String::with_capacity(s.len() + 2);
            out.push('"');
            py_escape(s, &mut out);
            out.push('"');
            out
        }
        Value::Array(items) => {
            let mut out = String::from("[");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&compact_json(item));
            }
            out.push(']');
            out
        }
        Value::Object(items) => {
            let mut entries: Vec<(String, &Value)> = items.iter().map(|(k, v)| (k.clone(), v)).collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut out = String::from("{");
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('"');
                py_escape(k, &mut out);
                out.push('"');
                out.push(':');
                out.push_str(&compact_json(v));
            }
            out.push('}');
            out
        }
    }
}

/// Python `ensure_ascii=True` string escaping: quotes/backslash/control
/// escaped literally, everything non-ASCII emitted as `\uXXXX`/`\UXXXXXXXX`.
fn py_escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c if (c as u32) < 0x80 => out.push(c),
            c if (c as u32) < 0x10000 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => {
                let cp = c as u32;
                let v = cp - 0x10000;
                let hi = 0xD800 + (v >> 10);
                let lo = 0xDC00 + (v & 0x3FF);
                out.push_str(&format!("\\u{:04x}\\u{:04x}", hi, lo));
            }
        }
    }
}

/// SHA-256 hex, matching `hashlib.sha256(...).hexdigest()` (16-char prefix
/// used by upstream `config_fingerprint`).
fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    let digest = h.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn load_all() -> Value {
    let path = cache_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Value::Object(Default::default());
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(map)) => Value::Object(map),
        _ => Value::Object(Default::default()),
    }
}

fn save_all(data: &Value) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = atomic_json_write(&path, data, 2, Some(0o600));
}

/// Return cached entry when fingerprint matches, else None.
pub fn get_cached_entry(server_name: &str, fingerprint: &str) -> Option<Value> {
    let _guard = CACHE_LOCK.lock().unwrap();
    let entry = load_all().get(server_name).cloned();
    let entry = entry?;
    if !entry.is_object() {
        return None;
    }
    if entry.get("fingerprint").and_then(Value::as_str) != Some(fingerprint) {
        return None;
    }
    Some(entry)
}

pub fn has_cached_entry(server_name: &str, fingerprint: &str) -> bool {
    get_cached_entry(server_name, fingerprint).is_some()
}

/// Persist tool schemas after a successful live connect.
pub fn write_cache_entry(
    server_name: &str,
    fingerprint: &str,
    tools: Vec<Value>,
    utility_tools: Option<Vec<Value>>,
) {
    let entry = json!({
        "fingerprint": fingerprint,
        "tools": tools,
        "utility_tools": utility_tools.unwrap_or_default(),
    });
    let _guard = CACHE_LOCK.lock().unwrap();
    let mut data = load_all();
    if data.get(server_name) == Some(&entry) {
        return;
    }
    if let Value::Object(map) = &mut data {
        map.insert(server_name.to_string(), entry);
    }
    save_all(&data);
}

pub fn clear_cache_entry(server_name: &str) {
    let _guard = CACHE_LOCK.lock().unwrap();
    let mut data = load_all();
    if let Value::Object(map) = &mut data {
        if map.remove(server_name).is_some() {
            save_all(&data);
        }
    }
}

/// Return cached MCP tool dicts (name, description, inputSchema).
pub fn tools_from_cache_entry(entry: &Value) -> Vec<Value> {
    entry
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

pub fn utility_tools_from_cache_entry(entry: &Value) -> Vec<Value> {
    entry
        .get("utility_tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}
