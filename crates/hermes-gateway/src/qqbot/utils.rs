//! QQBot shared utilities — User-Agent, HTTP helpers, config coercion.
//!
//! PARITY: `gateway/platforms/qqbot/utils.py` @ b9aa928 (whole module).
//!
//! TRANSLATION NOTE: `build_user_agent`'s Python-version slot describes the
//! host interpreter upstream; a Rust runtime has none, so
//! [`build_user_agent`] reports `Python/unknown` while
//! [`build_user_agent_with`] keeps the grammar fully parameterized for
//! callers/tests. `_get_hermes_version`'s fail-open `importlib.metadata`
//! lookup is a compile-time constant here (`hermes_cli::VERSION`).

use serde_json::Value;

use super::constants::QQBOT_VERSION;

/// Build a descriptive User-Agent string.
///
/// Format:
///
/// ```text
/// QQBotAdapter/<qqbot_version> (Python/<py_version>; <os>; Hermes/<hermes_version>)
/// ```
///
/// PARITY: `build_user_agent` (upstream lines 27-39), with the host facts
/// substituted: `platform.system().lower()` → [`std::env::consts::OS`]
/// (same lowercase single-word grammar), `importlib.metadata` → the
/// `hermes_cli` version constant, and the Python slot reports `unknown`.
pub fn build_user_agent() -> String {
    build_user_agent_with("unknown", std::env::consts::OS, hermes_cli::VERSION)
}

/// Explicit-host-facts form of [`build_user_agent`].
pub fn build_user_agent_with(py_version: &str, os_name: &str, hermes_version: &str) -> String {
    format!(
        "QQBotAdapter/{QQBOT_VERSION} (Python/{py_version}; {os_name}; Hermes/{hermes_version})"
    )
}

/// Return standard HTTP headers for QQBot API requests.
///
/// Includes `Content-Type`, `Accept`, and a dynamic `User-Agent`.
/// `q.qq.com` requires `Accept: application/json` — without it, the server
/// returns a JavaScript anti-bot challenge page.
///
/// PARITY: `get_api_headers` (upstream lines 42-51).
pub fn get_api_headers() -> Vec<(String, String)> {
    vec![
        ("Content-Type".to_string(), "application/json".to_string()),
        ("Accept".to_string(), "application/json".to_string()),
        ("User-Agent".to_string(), build_user_agent()),
    ]
}

/// Coerce config values into a trimmed string list.
///
/// Accepts comma-separated strings, lists, tuples, sets, or single values.
///
/// PARITY: `coerce_list` (upstream lines 54-63). JSON has no tuple/set —
/// both arrive as arrays, so the array arm covers them. Python's
/// `str(item).strip()` falsiness checks (`if str(item).strip()`) mean
/// whitespace-only and null entries are dropped.
pub fn coerce_list(value: Option<&Value>) -> Vec<String> {
    match value {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(s)) => s
            .split(',')
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| json_str(item).trim().to_string())
            .filter(|item| !item.is_empty())
            .collect(),
        Some(other) => {
            let s = json_str(other).trim().to_string();
            if s.is_empty() {
                Vec::new()
            } else {
                vec![s]
            }
        }
    }
}

/// Python `str(item)` over a JSON scalar (objects render as their JSON,
/// the closest lossless analogue of Python's repr-based fallback).
fn json_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => {
            if *b {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}
