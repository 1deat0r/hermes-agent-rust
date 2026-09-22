//! Clarify tool: structured multiple-choice / open-ended questions to the user.
//!
//! PARITY: `tools/clarify_tool.py` @ 5d59366 (whole module, 311 lines).
//! Schema, validation and a thin dispatcher; the UI lives in a
//! platform-provided callback (cli.py, gateway/run.py, tui_gateway).
//!
//! Callback model: upstream negotiates by signature inspection
//! (`multi_select` / `questions` kwargs). Rust has no introspection,
//! so the platform registers either a [`ClarifyCallback::Single`]
//! (one question per call — the legacy loop shape) or a
//! [`ClarifyCallback::Batch`] (whole normalized batch in one call,
//! replying `{"answers": {qid: raw}, "timed_out"?}` as JSON).

use std::cell::RefCell;
use std::sync::Arc;

use once_cell::sync::Lazy;
use serde_json::{json, Value};

use crate::registry::{registry, tool_error, CheckFn, ToolHandler, ToolResult};

/// Maximum predefined choices (the UI appends an "Other" row).
///
/// PARITY: `MAX_CHOICES` (upstream line 9).
pub const MAX_CHOICES: usize = 4;
/// Independent questions per batch call.
///
/// PARITY: `MAX_QUESTIONS` (upstream line 10).
pub const MAX_QUESTIONS: usize = 5;
/// Canonical timeout sentinel. Treated like `None` ("the user walked
/// away"): the batch loop aborts remaining questions.
///
/// PARITY: `TIMEOUT_RESPONSE` (upstream lines 13-14).
pub const TIMEOUT_RESPONSE: &str = "The user did not provide a response within the time limit. \
     Use your best judgement to make the choice and proceed.";
/// Applied to the first choice (not per-surface) so every adapter
/// renders it identically.
///
/// PARITY: `RECOMMENDED_LABEL` (upstream line 16).
pub const RECOMMENDED_LABEL: &str = "(Recommended)";
const UNAVAILABLE: &str = "Clarify tool is not available in this execution context.";

/// Platform-provided user-interaction callbacks.
#[derive(Clone)]
pub enum ClarifyCallback {
    /// One question per call: `(question, choices, multi_select) -> raw`.
    Single(Arc<dyn Fn(&str, Option<Vec<String>>, bool) -> String + Send + Sync>),
    /// Whole normalized batch in one call: `(title, questions) -> JSON`.
    Batch(Arc<dyn Fn(&str, Vec<NormalizedQuestion>) -> String + Send + Sync>),
}

thread_local! {
    static CLARIFY_CALLBACK: RefCell<Option<ClarifyCallback>> = const { RefCell::new(None) };
}

/// Set the platform interaction callback for this thread (the agent
/// runner injects this before dispatch, matching the Python
/// `kwargs["callback"]`).
pub fn set_clarify_callback(cb: ClarifyCallback) {
    CLARIFY_CALLBACK.with(|slot| *slot.borrow_mut() = Some(cb));
}

/// Legacy helper: register a single-question callback.
pub fn set_clarify_callback_fn<F>(cb: F)
where
    F: Fn(&str, Option<Vec<String>>, bool) -> String + Send + Sync + 'static,
{
    set_clarify_callback(ClarifyCallback::Single(Arc::new(cb)));
}

pub fn clear_clarify_callback() {
    CLARIFY_CALLBACK.with(|slot| *slot.borrow_mut() = None);
}

/// Coerce one choice to display text. Dict unwrap order `label` >
/// `description` > `text` > `title` (`name`/`value` excluded: raw
/// component enums, not labels). No match → "" and dropped.
///
/// PARITY: `_flatten_choice` (upstream lines 20-32). Lists join with
/// single spaces WITHOUT pre-filtering empties (then trim) — `"a  b"`
/// for `["a", "", "b"]`, exactly like `" ".join`.
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
        Value::Array(items) => items
            .iter()
            .map(flatten_choice)
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string(),
        other => other.to_string().trim().to_string(),
    }
}

/// Suffix the first choice (schema says best-first) with the recommended
/// label; idempotent, and a lone choice is left untouched.
///
/// PARITY: `mark_recommended` (upstream lines 35-41).
pub fn mark_recommended(choices: &[String]) -> Vec<String> {
    if choices.len() < 2 {
        return choices.to_vec();
    }
    let first = choices[0].trim().to_string();
    // Already labelled (or lone choice): leave untouched — idempotent.
    if first != strip_recommended(&first) {
        return choices.to_vec();
    }
    let mut out = vec![format!("{first} {RECOMMENDED_LABEL}")];
    out.extend(choices[1..].iter().cloned());
    out
}

/// Remove the recommendation label so presentation never leaks into
/// `user_response`.
///
/// PARITY: `strip_recommended` (upstream lines 44-49).
pub fn strip_recommended(text: &str) -> String {
    let stripped = text.trim();
    if stripped
        .to_lowercase()
        .ends_with(&RECOMMENDED_LABEL.to_lowercase())
    {
        return stripped[..stripped.len() - RECOMMENDED_LABEL.len()]
            .trim()
            .to_string();
    }
    stripped.to_string()
}

/// Parse a JSON string when it decodes to the expected shape, else None.
///
/// PARITY: `_json_as` (upstream lines 69-76).
fn json_as_array(raw: &str) -> Option<Vec<Value>> {
    match serde_json::from_str::<Value>(raw) {
        Ok(Value::Array(items)) => Some(items),
        _ => None,
    }
}

fn json_as_object(raw: &str) -> Option<serde_json::Map<String, Value>> {
    match serde_json::from_str::<Value>(raw) {
        Ok(Value::Object(map)) => Some(map),
        _ => None,
    }
}

/// Parse a list / JSON array / comma-separated reply into stripped
/// non-empty strings.
///
/// PARITY: `_parse_multi_select_response` (upstream lines 78-86).
pub fn parse_multi_select_response(raw_response: &Value) -> Vec<String> {
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
                if let Some(parsed) = json_as_array(raw) {
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
            // Upstream splits non-list values on commas too.
            raw.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
    }
}

/// Strip presentation (the label, multi-select JSON) from a locked answer.
///
/// PARITY: `_clean_answer` (upstream lines 89-91).
pub fn clean_answer(raw: Option<&Value>, multi: bool) -> Value {
    match raw {
        None => Value::String(String::new()),
        Some(raw) if multi => Value::Array(
            parse_multi_select_response(raw)
                .into_iter()
                .map(|s| Value::String(strip_recommended(&s)))
                .collect(),
        ),
        Some(raw) => {
            let text = match raw {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            Value::String(strip_recommended(&text))
        }
    }
}

/// Flatten, drop empties, cap at MAX_CHOICES; None when nothing
/// survives (open-ended).
///
/// PARITY: `_clean_choices` (upstream lines 94-97).
pub fn clean_choices(choices: &[Value]) -> Option<Vec<String>> {
    let cleaned: Vec<String> = choices
        .iter()
        .map(flatten_choice)
        .filter(|s| !s.is_empty())
        .collect();
    let capped: Vec<String> = cleaned.into_iter().take(MAX_CHOICES).collect();
    if capped.is_empty() {
        None
    } else {
        Some(capped)
    }
}

/// Timeout check: None or the exact sentinel means the user walked away.
///
/// PARITY: `_is_timeout` (upstream lines 100-101).
pub fn is_timeout(raw: Option<&Value>) -> bool {
    match raw {
        None => true,
        Some(Value::String(s)) => s.trim() == TIMEOUT_RESPONSE,
        _ => false,
    }
}

/// One validated batch question: stable wire id, echoed model id,
/// decorated + bare choices, multi-select flag.
///
/// PARITY: the normalized entry (upstream lines 106-136).
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedQuestion {
    pub qid: String,
    pub id: Option<String>,
    pub question: String,
    pub choices: Option<Vec<String>>,
    pub choices_offered: Option<Vec<String>>,
    pub multi_select: bool,
}

impl NormalizedQuestion {
    fn to_json(&self) -> Value {
        json!({
            "qid": self.qid,
            "id": self.id,
            "question": self.question,
            "choices": self.choices,
            "choices_offered": self.choices_offered,
            "multi_select": self.multi_select,
        })
    }
}

/// Validate the `questions` batch param → `(normalized, error)`; an
/// empty list gives `(None, None)` (fall back to single-question).
///
/// PARITY: `_normalize_questions` (upstream lines 106-136).
pub fn normalize_questions(questions: &Value) -> (Option<Vec<NormalizedQuestion>>, Option<String>) {
    let Some(items) = questions.as_array() else {
        return (
            None,
            Some("questions must be an array of question objects.".to_string()),
        );
    };
    if items.is_empty() {
        return (None, None);
    }
    if items.len() > MAX_QUESTIONS {
        return (
            None,
            Some(format!("questions supports at most {MAX_QUESTIONS} items.")),
        );
    }
    let mut normalized = Vec::new();
    for (index, item) in items.iter().enumerate() {
        // Tolerate bare-string items: LLMs sometimes send ["Q1?", "Q2?"].
        let item_obj: Value = match item {
            Value::String(s) => json!({"question": s}),
            Value::Object(_) => item.clone(),
            _ => {
                return (
                    None,
                    Some(format!(
                        "questions[{index}] must be an object with a 'question'."
                    )),
                )
            }
        };
        let text = item_obj
            .get("question")
            .map(|v| match v {
                Value::String(s) => s.trim().to_string(),
                Value::Null => String::new(),
                other => other.to_string().trim().to_string(),
            })
            .unwrap_or_default();
        if text.is_empty() {
            return (
                None,
                Some(format!(
                    "questions[{index}].question must be non-empty text."
                )),
            );
        }
        let raw_choices = item_obj.get("choices");
        let choices = match raw_choices {
            None | Some(Value::Null) => None,
            Some(Value::Array(list)) => clean_choices(list),
            _ => {
                return (
                    None,
                    Some(format!("questions[{index}].choices must be a list.")),
                )
            }
        };
        // NOTE: upstream checks `choices = _clean_choices(choices)` then
        // `mark_recommended(list(choices)) if choices else None` — a
        // present-but-emptied list becomes None (open-ended).
        let decorated = choices.clone().map(|c| mark_recommended(&c));
        let offered = choices.clone();
        let multi = item_obj
            .get("multi_select")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            && choices.is_some();
        let id = item_obj
            .get("id")
            .map(|v| match v {
                Value::String(s) => s.trim().to_string(),
                Value::Null => String::new(),
                other => other.to_string().trim().to_string(),
            })
            .filter(|s| !s.is_empty());
        normalized.push(NormalizedQuestion {
            qid: format!("q{index}"),
            id,
            question: text,
            choices: decorated,
            choices_offered: offered,
            multi_select: multi,
        });
    }
    (Some(normalized), None)
}

/// Batch result JSON; unanswered → "". The top-level `timed_out` flag
/// (present only when true) tells the agent whether blanks are
/// deliberate skips or the user walking away.
///
/// PARITY: `_batch_result` (upstream lines 139-152).
pub fn batch_result(
    normalized: &[NormalizedQuestion],
    answers: &serde_json::Map<String, Value>,
    timed_out: bool,
) -> String {
    let mut responses = Vec::new();
    for entry in normalized {
        let raw = answers.get(&entry.qid);
        let user_response = match raw {
            Some(_) => clean_answer(raw, entry.multi_select),
            None => Value::String(String::new()),
        };
        let mut response = serde_json::Map::new();
        if let Some(id) = &entry.id {
            response.insert("id".to_string(), Value::String(id.clone()));
        }
        response.insert(
            "question".to_string(),
            Value::String(entry.question.clone()),
        );
        response.insert(
            "choices_offered".to_string(),
            entry
                .choices_offered
                .clone()
                .map(|c| Value::Array(c.into_iter().map(Value::String).collect()))
                .unwrap_or(Value::Null),
        );
        response.insert("user_response".to_string(), user_response);
        responses.push(Value::Object(response));
    }
    let mut result = serde_json::Map::new();
    result.insert("responses".to_string(), Value::Array(responses));
    if timed_out {
        result.insert("timed_out".to_string(), Value::Bool(true));
    }
    serde_json::to_string(&Value::Object(result)).unwrap_or_default()
}

/// Ask one question or a batch (`questions` wins when non-empty).
///
/// PARITY: `clarify_tool` (upstream lines 182-230).
pub fn clarify_tool(
    question: &str,
    choices: Option<Vec<Value>>,
    multi_select: bool,
    questions: Option<&Value>,
    callback: Option<ClarifyCallback>,
) -> String {
    if let Some(questions) = questions {
        let (normalized, error) = normalize_questions(questions);
        if let Some(error) = error {
            return tool_error(&error, &[]);
        }
        if let Some(normalized) = normalized {
            let Some(callback) = callback else {
                return tool_error(
                    "Clarify tool is not available in this execution context.",
                    &[],
                );
            };
            return match run_batch(&normalized, &callback, question) {
                Ok(output) => output,
                Err(error) => tool_error(&format!("Failed to get user input: {error}"), &[]),
            };
        }
        // Empty questions array → fall through to single-question path.
    }
    if question.trim().is_empty() {
        return tool_error(
            "No question provided. Pass questions=[{question: '...', choices?: [...], \
             multi_select?: bool}, ...] — a single question is a one-entry array.",
            &[],
        );
    }
    let question = question.trim();
    let choices = match choices {
        Some(list) => clean_choices(&list),
        None => None,
    };
    let Some(callback) = callback else {
        return tool_error(
            "Clarify tool is not available in this execution context.",
            &[],
        );
    };
    // The bare list goes back to the agent; "(Recommended)" is presentation only.
    let shown = choices.clone().map(|c| mark_recommended(&c));
    let raw_response = match &callback {
        ClarifyCallback::Single(cb) => {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                cb(question, shown, multi_select)
            })) {
                Ok(response) => response,
                Err(_) => {
                    return tool_error("Failed to get user input: callback raised", &[]);
                }
            }
        }
        ClarifyCallback::Batch(cb) => {
            // A batch-capable callback asked for one question: loop once
            // through the batch path with a single normalized entry.
            let entry = NormalizedQuestion {
                qid: "q0".to_string(),
                id: None,
                question: question.to_string(),
                choices: shown.clone(),
                choices_offered: choices.clone(),
                multi_select: multi_select && choices.is_some(),
            };
            return run_batch(&[entry], &callback, question).unwrap_or_else(|error| {
                tool_error(&format!("Failed to get user input: {error}"), &[])
            });
        }
    };
    let user_response = if multi_select && choices.is_some() {
        clean_answer(Some(&Value::String(raw_response)), true)
    } else {
        clean_answer(Some(&Value::String(raw_response)), false)
    };
    serde_json::to_string(&json!({
        "question": question,
        "choices_offered": choices,
        "user_response": user_response,
    }))
    .unwrap_or_default()
}

/// Dispatch a validated batch. Batch-capable callbacks get the whole
/// list once; legacy callbacks are looped per question (empty = skip,
/// timeout = abort keeping partials).
///
/// PARITY: `_run_batch` (upstream lines 155-179).
fn run_batch(
    normalized: &[NormalizedQuestion],
    callback: &ClarifyCallback,
    title: &str,
) -> Result<String, String> {
    if let ClarifyCallback::Batch(cb) = callback {
        let raw = cb(title, normalized.to_vec());
        if is_timeout(Some(&Value::String(raw.clone()))) {
            return Ok(batch_result(normalized, &serde_json::Map::new(), true));
        }
        // Dict or JSON string (the tui_gateway bridge only carries
        // strings); falsy/unparseable → cancel-all.
        let parsed: Option<serde_json::Map<String, Value>> = if raw.trim().is_empty() {
            None
        } else {
            json_as_object(&raw)
        };
        match parsed {
            Some(map) => {
                let answers = map
                    .get("answers")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                let timed_out = map
                    .get("timed_out")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                Ok(batch_result(normalized, &answers, timed_out))
            }
            None => Ok(batch_result(normalized, &serde_json::Map::new(), false)),
        }
    } else if let ClarifyCallback::Single(cb) = callback {
        let mut answers = serde_json::Map::new();
        let mut timed_out = false;
        for entry in normalized {
            let raw = cb(&entry.question, entry.choices.clone(), entry.multi_select);
            if is_timeout(Some(&Value::String(raw.clone()))) {
                timed_out = true;
                break;
            }
            answers.insert(entry.qid.clone(), Value::String(raw));
        }
        Ok(batch_result(normalized, &answers, timed_out))
    } else {
        Err("no callback".to_string())
    }
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
        let choices = args.get("choices").and_then(Value::as_array).cloned();
        let multi_select = args
            .get("multi_select")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let questions = args.get("questions").cloned();
        let callback = CLARIFY_CALLBACK.with(|slot| slot.borrow().clone());
        ToolResult::Text(clarify_tool(
            question,
            choices,
            multi_select,
            questions.as_ref(),
            callback,
        ))
    }
}

pub static CLARIFY_SCHEMA: Lazy<Value> = Lazy::new(|| {
    json!({
        "name": "clarify",
        "description": "Ask the user one or more questions when you need a decision, clarification, or feedback before proceeding. Pass every question in `questions` (1-5 entries) — a single question is a one-entry array, and several INDEPENDENT questions belong in ONE call (one form beats a chain of clarify calls; if one answer would change another question, ask separately). Per question: single-select (up to 4 choices — put your recommended option FIRST, the UI marks it '(Recommended)' and auto-appends an 'Other' free-text row), multi-select (multi_select=true), or open-ended (omit choices). Options go ONLY in `choices`, never enumerated inside the question text (choices render as pickable rows; options written into the question are dead prose the user can't click). Result: {responses: [...]} in question order (plus timed_out=true if the user stopped part-way). Prefer deciding low-stakes questions yourself; don't use this for dangerous-command confirmation (the terminal tool handles that).",
        "parameters": {
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": MAX_QUESTIONS,
                    "description": "The question(s). Each: question text (options excluded), optional choices (recommended first; omit for free-text), optional multi_select. Responses come back in question order with the question text echoed.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": {"type": "string"},
                            "choices": {
                                "type": "array",
                                "items": {"type": "string"},
                                "maxItems": MAX_CHOICES,
                            },
                            "multi_select": {"type": "boolean"},
                        },
                        "required": ["question"],
                    },
                },
            },
            "required": ["questions"],
        },
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
