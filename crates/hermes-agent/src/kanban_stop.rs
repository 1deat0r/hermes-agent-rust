//! Turn-end guard for kanban workers.
//!
//! PARITY: `agent/kanban_stop.py` @ b9aa928 (whole module, lines 1-109).
//!
//! Kanban workers must end with `kanban_complete` or `kanban_block`. Models
//! (especially the GLM / Qwen families) sometimes narrate the next step ("Let me
//! write the report now") and stop with `finish_reason=stop` and no tool calls.
//! Hermes treats that as a clean exit → `rc=0` → dispatcher `protocol_violation`.
//!
//! This module is policy-only: when a kanban worker tries to finish without a
//! terminal board tool, return a bounded synthetic nudge so the conversation
//! loop continues instead of exiting.

use serde_json::Value;
use std::env;

/// PARITY: `_TERMINAL_KANBAN_TOOLS` (upstream line 18).
const TERMINAL_KANBAN_TOOLS: [&str; 2] = ["kanban_complete", "kanban_block"];

/// PARITY: `_DEFAULT_MAX_ATTEMPTS` (upstream line 20).
pub const DEFAULT_MAX_ATTEMPTS: i64 = 2;

/// PARITY: `kanban_stop_nudge_enabled` (upstream lines 23-33).
///
/// On when `HERMES_KANBAN_TASK` is set (a dispatcher-spawned worker), unless
/// `HERMES_KANBAN_STOP_NUDGE` explicitly disables it. Note the asymmetry the
/// source keeps: an *unset* disable variable is not the same as an empty one —
/// both leave the guard on — and only the four explicit off-spellings (matched
/// after `strip().lower()`) turn it off.
pub fn kanban_stop_nudge_enabled() -> bool {
    let disabled = env::var("HERMES_KANBAN_STOP_NUDGE")
        .map(|value| {
            matches!(
                value.trim().to_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            )
        })
        .unwrap_or(false);
    if disabled {
        return false;
    }
    !env::var("HERMES_KANBAN_TASK")
        .unwrap_or_default()
        .trim()
        .is_empty()
}

/// PARITY: `_tool_call_name` (upstream lines 36-46).
///
/// The source probes the dict shapes first and then falls back to attribute
/// access on SDK objects; a non-object payload (a bare string, for example)
/// yields the empty name from the final `getattr(tc, "name", "")` arm, which is
/// what `Value::Null`-ish shapes map to here.
fn tool_call_name(tool_call: &Value) -> String {
    let Some(map) = tool_call.as_object() else {
        return String::new();
    };
    match map.get("function") {
        Some(Value::Object(function)) => python_str(function.get("name")).unwrap_or_default(),
        // `fn` exists but is not a mapping: `str(tc.get("name") or "")`.
        Some(_) => python_str(map.get("name")).unwrap_or_default(),
        None => python_str(map.get("name")).unwrap_or_default(),
    }
}

/// Python's `str(value or "")` applied to a JSON leaf: falsy leaves collapse to
/// the empty string, and a non-string leaf renders as `str()` would.
fn python_str(value: Option<&Value>) -> Option<String> {
    let value = value?;
    let falsy = match value {
        Value::Null => true,
        Value::Bool(value) => !*value,
        Value::Number(number) => number.as_f64() == Some(0.0),
        Value::String(text) => text.is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Object(items) => items.is_empty(),
    };
    if falsy {
        return Some(String::new());
    }
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Bool(true) => Some("True".into()),
        other => Some(other.to_string()),
    }
}

/// True when this conversation already invoked a terminal kanban tool.
///
/// PARITY: `session_called_kanban_terminal` (upstream lines 49-66): assistant
/// messages are scanned through `tool_calls` (missing or null counts as no
/// calls) and tool messages through their `name`; every other role, and every
/// non-mapping entry, is skipped.
pub fn session_called_kanban_terminal(messages: Option<&[Value]>) -> bool {
    let Some(messages) = messages.filter(|messages| !messages.is_empty()) else {
        return false;
    };
    for message in messages {
        let Some(map) = message.as_object() else {
            continue;
        };
        match map.get("role").and_then(Value::as_str) {
            Some("assistant") => {
                let tool_calls: &[Value] = match map.get("tool_calls") {
                    Some(Value::Array(calls)) => calls,
                    _ => &[],
                };
                if tool_calls
                    .iter()
                    .any(|call| TERMINAL_KANBAN_TOOLS.contains(&tool_call_name(call).as_str()))
                {
                    return true;
                }
            }
            Some("tool") => {
                let name = python_str(map.get("name")).unwrap_or_default();
                if TERMINAL_KANBAN_TOOLS.contains(&name.as_str()) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Keyword arguments of `build_kanban_stop_nudge` (upstream lines 69-108).
#[derive(Debug, Clone, Copy, Default)]
pub struct KanbanStopNudgeOptions<'a> {
    pub messages: Option<&'a [Value]>,
    pub attempts: i64,
    /// `None` means "use [`DEFAULT_MAX_ATTEMPTS`]", matching the keyword
    /// default in the source signature.
    pub max_attempts: Option<i64>,
    pub task_id: Option<&'a str>,
}

impl<'a> KanbanStopNudgeOptions<'a> {
    /// Options with the source's keyword defaults (`attempts=0`,
    /// `max_attempts=2`).
    pub fn new(messages: Option<&'a [Value]>) -> Self {
        Self {
            messages,
            attempts: 0,
            max_attempts: None,
            task_id: None,
        }
    }

    pub fn with_attempts(mut self, attempts: i64) -> Self {
        self.attempts = attempts;
        self
    }

    pub fn with_max_attempts(mut self, max_attempts: i64) -> Self {
        self.max_attempts = Some(max_attempts);
        self
    }

    pub fn with_task_id(mut self, task_id: Option<&'a str>) -> Self {
        self.task_id = task_id;
        self
    }
}

/// Return a synthetic follow-up when a kanban worker exits without a terminal
/// tool.
///
/// PARITY: `build_kanban_stop_nudge` (upstream lines 69-108). `None` when the
/// guard must not fire: not a kanban worker, the session already completed or
/// blocked, or the nudge budget is exhausted. The task label falls back through
/// the explicit argument, `HERMES_KANBAN_TASK`, then the literal `"this task"`.
pub fn build_kanban_stop_nudge(options: KanbanStopNudgeOptions<'_>) -> Option<String> {
    if !kanban_stop_nudge_enabled() {
        return None;
    }
    if options.attempts >= options.max_attempts.unwrap_or(DEFAULT_MAX_ATTEMPTS) {
        return None;
    }
    if session_called_kanban_terminal(options.messages) {
        return None;
    }
    // `(task_id or os.environ.get("HERMES_KANBAN_TASK") or "").strip() or "this task"`
    let candidate = options
        .task_id
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            env::var("HERMES_KANBAN_TASK")
                .ok()
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default();
    let trimmed = candidate.trim();
    let task_id = if trimmed.is_empty() {
        "this task"
    } else {
        trimmed
    };
    Some(format!(
        "[System: You are a Hermes kanban worker. A plain-text reply is NOT a \
         terminal state for the board.\n\n\
         Task `{task_id}` is still `running`. Ending now without a board tool \
         causes a protocol violation (clean exit with no \
         `kanban_complete` / `kanban_block`).\n\n\
         Do this immediately in your next response — do not narrate intent:\n\
         1. Finish any remaining deliverable (write the required file(s) now).\n\
         2. Call `kanban_complete(summary=..., artifacts=[...])` if the work \
         is done, OR `kanban_block(reason=...)` if you are blocked.\n\n\
         Never end a turn with only a promise of future action. Repeated \
         protocol violations will block this task and require manual intervention.]"
    ))
}
