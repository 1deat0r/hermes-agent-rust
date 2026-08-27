//! Shared helpers for classifying tool result payloads.
//!
//! PARITY: `agent/tool_result_classification.py` @ b9aa928 (whole module).

use crate::config::json_truthy;
use serde_json::Value;

/// PARITY: `FILE_MUTATING_TOOL_NAMES` (upstream line 8).
pub const FILE_MUTATING_TOOL_NAMES: [&str; 2] = ["write_file", "patch"];

/// Tools whose interrupted/dangling execution is safe to discard because they
/// cannot mutate either external state or Hermes session state. Unknown,
/// plugin, and MCP tools stay effect-capable by default.
///
/// PARITY: `NO_EFFECT_TOOL_NAMES` (upstream lines 11-17).
pub const NO_EFFECT_TOOL_NAMES: [&str; 12] = [
    "read_file",
    "search_files",
    "session_search",
    "skill_view",
    "skills_list",
    "web_extract",
    "web_search",
    "vision_analyze",
    "browser_snapshot",
    "browser_get_images",
    "browser_console",
    "read_terminal",
];

/// PARITY: `tool_may_have_side_effect` (upstream line 20).
pub fn tool_may_have_side_effect(tool_name: &str) -> bool {
    !NO_EFFECT_TOOL_NAMES.contains(&tool_name)
}

/// Return `true` when a file mutation result proves the write landed.
///
/// PARITY: `file_mutation_result_landed` (upstream lines 23-40). `result` is
/// the tool payload: the source requires a `str` carrying a JSON document, so
/// an already-parsed mapping fails the `isinstance(result, str)` guard and is
/// represented here by a non-string [`Value`]. A truthy top-level `error`
/// discards the proof, `write_file` is proven by the `bytes_written` key, and
/// `patch` by `success` being exactly `True`.
pub fn file_mutation_result_landed(tool_name: &str, result: Option<&Value>) -> bool {
    if !FILE_MUTATING_TOOL_NAMES.contains(&tool_name) {
        return false;
    }
    // `not isinstance(result, str)` — including a missing payload.
    let Some(Value::String(payload)) = result else {
        return false;
    };
    let Ok(data) = serde_json::from_str::<Value>(payload.trim()) else {
        return false;
    };
    let Value::Object(data) = data else {
        return false;
    };
    if json_truthy(data.get("error")) {
        return false;
    }
    match tool_name {
        "write_file" => data.contains_key("bytes_written"),
        "patch" => matches!(data.get("success"), Some(Value::Bool(true))),
        _ => false,
    }
}
