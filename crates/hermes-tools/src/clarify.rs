//! Clarify tool — interactive clarifying questions.
//!
//! PARITY: tools/clarify_tool.py @ b9aa928 (266 LOC, ported 1:1).
//!
//! The actual user-interaction lives in the platform layer (CLI / gateway);
//! this module defines the schema, validation, and a thin dispatcher that
//! delegates to a platform-provided callback. The callback is injected via
//! a thread-local slot (mirroring the Python runner injecting `callback=` in
//! dispatch kwargs).

use std::cell::RefCell;
use std::sync::Arc;

/// Platform-provided user-interaction callback.
pub type ClarifyCallback = dyn Fn(&str, Option<Vec<String>>, bool) -> String + Send + Sync;

use once_cell::sync::Lazy;
use serde_json::{json, Value};

use crate::registry::{registry, tool_error, CheckFn, ToolHandler, ToolResult};

/// Maximum number of predefined choices the agent can offer. A 5th "Other
/// (type your answer)" option is always appended by the UI.
pub const MAX_CHOICES: usize = 4;

thread_local! {
    static CLARIFY_CALLBACK: RefCell<Option<Arc<ClarifyCallback>>> =
        RefCell::new(None);
}

/// Set the platform interaction callback for this thread (the agent runner
/// injects this before dispatch, matching the Python `kwargs["callback"]`).
pub fn set_clarify_callback<F>(cb: F)
where
    F: Fn(&str, Option<Vec<String>>, bool) -> String + Send + Sync + 'static,
{
    CLARIFY_CALLBACK.with(|slot| *slot.borrow_mut() = Some(Arc::new(cb)));
}

pub fn clear_clarify_callback() {
    CLARIFY_CALLBACK.with(|slot| *slot.borrow_mut() = None);
}

/// Coerce a single choice into its user-facing display string (dict unwrap).
pub fn flatten_choice(c: &Value) -> String {
    match c {
        Value::Null => String::new(),
        Value::String(s) => s.trim().to_string(),
        Value::Object(map) => {
            for key in ["label", "description", "text", "title"] {
                if let Some(Value::String(v)) = map.get(key) {
                    let v = v.trim();
                    if !v.is_empty() {
                        return v.to_string();
                    }
                }
            }
            String::new()
        }
        Value::Array(items) => {
            let joined: Vec<String> = items.iter().map(flatten_choice).filter(|s| !s.is_empty()).collect();
            joined.join(" ").trim().to_string()
        }
        other => other.to_string().trim().to_string(),
    }
}

/// Parse a multi-select response into a list of cleaned choice strings.
pub fn parse_multi_select_response(raw_response: Value) -> Vec<String> {
    match raw_response {
        Value::Array(items) => items
            .iter()
            .map(|r| match r {
                Value::String(s) => s.trim().to_string(),
                other => other.to_string().trim().to_string(),
            })
            .filter(|s| !s.is_empty())
            .collect(),
        Value::String(raw) => {
            let raw = raw.trim();
            if raw.starts_with('[') {
                if let Ok(Value::Array(parsed)) = serde_json::from_str::<Value>(raw) {
                    return parsed
                        .iter()
                        .map(|p| match p {
                            Value::String(s) => s.trim().to_string(),
                            other => other.to_string().trim().to_string(),
                        })
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
            raw.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
        other => {
            let raw = other.to_string().trim().to_string();
            if raw.is_empty() {
                Vec::new()
            } else {
                vec![raw]
            }
        }
    }
}

/// Ask the user a question, optionally with multiple-choice options.
///
/// PARITY: tools/clarify_tool.py clarify_tool @ b9aa928
pub fn clarify_tool(
    question: &str,
    choices: Option<Vec<Value>>,
    multi_select: bool,
) -> String {
    let question = question.trim();
    if question.is_empty() {
        return tool_error("Question text is required.", &[]);
    }

    // Validate and trim choices.
    let mut choices: Option<Vec<String>> = choices.map(|cs| {
        let mut flat: Vec<String> = cs.iter().map(flatten_choice).filter(|s| !s.is_empty()).collect();
        if flat.len() > MAX_CHOICES {
            flat.truncate(MAX_CHOICES);
        }
        if flat.is_empty() {
            None
        } else {
            Some(flat)
        }
            .unwrap_or_default()
    });
    // Choices is "None"/open-ended when the original was empty after flatten.
    let choices_have = choices.as_ref().map(|c| !c.is_empty()).unwrap_or(false);
    if !choices_have {
        choices = None;
    }

    let raw_response = CLARIFY_CALLBACK.with(|slot| {
        let guard = slot.borrow();
        let cb = guard.as_ref()?;
        let questions = question.to_string();
        let choices_clone = choices.clone();
        let out = cb(&questions, choices_clone, multi_select);
        Some(out)
    });
    let Some(raw_response) = raw_response else {
        return tool_error("Clarify tool is not available in this execution context.", &[]);
    };

    let user_response: Value = if multi_select && choices.is_some() {
        let parsed = parse_multi_select_response(Value::String(raw_response.clone()));
        Value::Array(parsed.into_iter().map(Value::String).collect())
    } else {
        Value::String(raw_response.trim().to_string())
    };

    serde_json::to_string(&json!({
        "question": question,
        "choices_offered": choices,
        "user_response": user_response,
    }))
    .expect("json")
}

pub struct ClarifyCheck;
impl CheckFn for ClarifyCheck {
    fn check(&self) -> bool {
        true
    }
}

struct ClarifyHandler;
impl ToolHandler for ClarifyHandler {
    fn call(&self, args: Value, _: Option<&str>, _: Option<&str>) -> ToolResult {
        let question = args.get("question").and_then(Value::as_str).unwrap_or("");
        let choices = args
            .get("choices")
            .and_then(Value::as_array)
            .cloned();
        let multi_select = args.get("multi_select").and_then(Value::as_bool).unwrap_or(false);
        ToolResult::Text(clarify_tool(question, choices, multi_select))
    }
}

pub static CLARIFY_SCHEMA: Lazy<Value> = Lazy::new(|| {
    json!({
        "name": "clarify",
        "description": "Ask the user a question when you need clarification, feedback, or a decision before proceeding. Supports three modes:\n\n1. **Single-select multiple choice** — provide up to 4 choices. The user picks one or types their own answer via a 5th 'Other' option.\n2. **Multi-select multiple choice** — set multi_select=true. The user can select multiple options via checkboxes. user_response will be a list of selected choices.\n3. **Open-ended** — omit choices entirely. The user types a free-form response.\n\nCRITICAL: when you are offering options, put each option ONLY in the `choices` array — NEVER enumerate the options inside the `question` text. The UI renders `choices` as selectable rows; options written into the question string render as dead prose the user can't pick. Right: question='Which deployment target?', choices=['staging', 'prod']. Wrong: question='Which target? 1) staging 2) prod', choices=[].",
        "parameters": {
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question itself, and ONLY the question (e.g. 'Which deployment target?'). Do NOT embed the answer options here — pass them as separate elements in `choices`."
                },
                "choices": {
                    "type": "array",
                    "items": {"type": "string"},
                    "maxItems": MAX_CHOICES,
                    "description": "REQUIRED whenever you are presenting selectable options: each distinct option is its own array element (up to 4). The UI renders these as pickable rows and auto-appends an 'Other (type your answer)' option. Omit this parameter entirely ONLY for a genuinely open-ended free-text question."
                },
                "multi_select": {
                    "type": "boolean",
                    "description": "When true, the user can select MULTIPLE options (like checkboxes). The user_response will be a list of selected choices. When false (default), single selection (radio). Has no effect when choices is omitted (open-ended question)."
                }
            },
            "required": ["question"]
        }
    })
});

/// Register the clarify tool into the registry singleton.
pub fn register_clarify() {
    registry()
        .register(
            "clarify",
            "clarify",
            CLARIFY_SCHEMA.clone(),
            Arc::new(ClarifyHandler),
            Some(Arc::new(ClarifyCheck)),
            Some("check_clarify_requirements"),
            vec![],
            None,
            Some("❓".to_string()),
            None,
            None,
            None,
            false,
        )
        .expect("register clarify");
}
