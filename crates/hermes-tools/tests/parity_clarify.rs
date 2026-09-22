//! Parity oracles for tools/clarify_tool.py @ 5d59366, mirroring
//! upstream tests/tools/test_clarify_tool.py (single + batch + labels +
//! schema). The platform callback is injected directly (Rust has no
//! monkeypatching); batch-capable callbacks use ClarifyCallback::Batch.

use serde_json::{json, Value};

use hermes_tools::clarify::{
    clarify_tool, clean_answer, flatten_choice, is_timeout, mark_recommended, normalize_questions,
    parse_multi_select_response, strip_recommended, ClarifyCallback, MAX_CHOICES, MAX_QUESTIONS,
    TIMEOUT_RESPONSE,
};

fn single(answer: &str) -> ClarifyCallback {
    let answer = answer.to_string();
    ClarifyCallback::Single(std::sync::Arc::new(move |_, _, _| answer.clone()))
}

fn ask(question: &str, choices: Option<Vec<Value>>, multi: bool, cb: ClarifyCallback) -> Value {
    serde_json::from_str(&clarify_tool(question, choices, multi, None, Some(cb))).unwrap()
}

// ── single-question path ───────────────────────────────────────────────

#[test]
fn simple_question_with_callback() {
    let result = ask("What color?", None, false, single("blue"));
    assert_eq!(result["question"], "What color?");
    assert!(result["choices_offered"].is_null());
    assert_eq!(result["user_response"], "blue");
}

#[test]
fn no_callback_returns_error() {
    let result: Value =
        serde_json::from_str(&clarify_tool("What do you want?", None, false, None, None)).unwrap();
    assert!(result.get("error").is_some());
    assert!(result["error"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("not available"));
}

#[test]
fn missing_question_text_errors_with_batch_guidance() {
    let result: Value =
        serde_json::from_str(&clarify_tool("", None, false, None, Some(single("x")))).unwrap();
    assert!(result["error"].as_str().unwrap().contains("questions="));
}

#[test]
fn choices_trimmed_to_max() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen_cb = std::sync::Arc::clone(&seen);
    let cb = ClarifyCallback::Single(std::sync::Arc::new(move |_, choices, _| {
        *seen_cb.lock().unwrap() = choices.unwrap_or_default();
        "picked".to_string()
    }));
    let many: Vec<Value> = ["a", "b", "c", "d", "e", "f", "g"]
        .iter()
        .map(|s| json!(s))
        .collect();
    ask("Pick one", Some(many), false, cb);
    assert_eq!(seen.lock().unwrap().len(), MAX_CHOICES);
}

#[test]
fn choices_converted_to_strings_with_recommended_first() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen_cb = std::sync::Arc::clone(&seen);
    let cb = ClarifyCallback::Single(std::sync::Arc::new(move |_, choices, _| {
        *seen_cb.lock().unwrap() = choices.unwrap_or_default();
        "answer".to_string()
    }));
    ask("Pick", Some(vec![json!(1), json!(2), json!(3)]), false, cb);
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            "1 (Recommended)".to_string(),
            "2".to_string(),
            "3".to_string()
        ]
    );
}

#[test]
fn callback_exception_returns_error() {
    let cb = ClarifyCallback::Single(std::sync::Arc::new(|_, _, _| {
        panic!("User cancelled");
    }));
    let result: Value =
        serde_json::from_str(&clarify_tool("Question?", None, false, None, Some(cb))).unwrap();
    assert!(result.get("error").is_some());
    assert!(result["error"]
        .as_str()
        .unwrap()
        .contains("Failed to get user input"));
}

#[test]
fn user_response_stripped() {
    let result = ask("Q?", None, false, single("  response with spaces  \n"));
    assert_eq!(result["user_response"], "response with spaces");
}

// ── labels ─────────────────────────────────────────────────────────────

#[test]
fn first_choice_is_labelled_and_answer_strips_it() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen_cb = std::sync::Arc::clone(&seen);
    let cb = ClarifyCallback::Single(std::sync::Arc::new(move |_, choices, _| {
        *seen_cb.lock().unwrap() = choices.unwrap_or_default();
        "a (Recommended)".to_string()
    }));
    let result = ask("Pick", Some(vec![json!("a"), json!("b")]), false, cb);
    assert_eq!(seen.lock().unwrap()[0], "a (Recommended)");
    assert_eq!(result["user_response"], "a");
}

#[test]
fn single_choice_is_not_labelled_and_label_not_doubled() {
    assert_eq!(
        mark_recommended(&["only".to_string()]),
        vec!["only".to_string()]
    );
    assert_eq!(
        mark_recommended(&["a (Recommended)".to_string(), "b".to_string()]),
        vec!["a (Recommended)".to_string(), "b".to_string()]
    );
}

#[test]
fn flatten_unwraps_label_first() {
    assert_eq!(
        flatten_choice(&json!({"description": "D", "label": "L"})),
        "L"
    );
    assert_eq!(flatten_choice(&json!({"name": "raw", "value": "1"})), "");
    assert_eq!(flatten_choice(&json!(["a", "", "b"])), "a  b");
}

// ── multi-select ───────────────────────────────────────────────────────

#[test]
fn multi_select_true_returns_list() {
    let result = ask(
        "Pick",
        Some(vec![json!("a"), json!("b")]),
        true,
        single("[\"a (Recommended)\", \"b\"]"),
    );
    assert_eq!(result["user_response"], json!(["a", "b"]));
}

#[test]
fn multi_select_comma_string_parses() {
    assert_eq!(
        parse_multi_select_response(&json!("a, b ,c")),
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
}

#[test]
fn clean_answer_strips_labels() {
    assert_eq!(
        clean_answer(Some(&json!("x (Recommended)")), false),
        json!("x")
    );
    assert!(clean_answer(None, false) == json!(""));
}

// ── batch ──────────────────────────────────────────────────────────────

fn batch_cb(answers: serde_json::Map<String, Value>) -> ClarifyCallback {
    ClarifyCallback::Batch(std::sync::Arc::new(move |_, _| {
        serde_json::to_string(&json!({"answers": answers})).unwrap()
    }))
}

#[test]
fn batch_takes_precedence_over_question() {
    let result: Value = serde_json::from_str(&clarify_tool(
        "ignored?",
        None,
        false,
        Some(&json!([{"question": "Q1?"}])),
        Some(batch_cb(serde_json::Map::new())),
    ))
    .unwrap();
    assert!(result.get("responses").is_some());
    assert!(result.get("question").is_none());
}

#[test]
fn batch_rejects_more_than_five_and_blank_text() {
    let many: Vec<Value> = (0..6)
        .map(|i| json!({"question": format!("Q{i}?")}))
        .collect();
    let result: Value = serde_json::from_str(&clarify_tool(
        "",
        None,
        false,
        Some(&json!(many)),
        Some(single("x")),
    ))
    .unwrap();
    assert!(result["error"].as_str().unwrap().contains("at most 5"));
    let result: Value = serde_json::from_str(&clarify_tool(
        "",
        None,
        false,
        Some(&json!([{"question": "  "}])),
        Some(single("x")),
    ))
    .unwrap();
    assert!(result["error"].as_str().unwrap().contains("non-empty text"));
    let result: Value = serde_json::from_str(&clarify_tool(
        "",
        None,
        false,
        Some(&json!("nope")),
        Some(single("x")),
    ))
    .unwrap();
    assert!(result["error"]
        .as_str()
        .unwrap()
        .contains("must be an array"));
}

#[test]
fn batch_empty_list_falls_back_to_single_question() {
    let result = ask("Fallback?", None, false, single("ans"));
    assert_eq!(result["question"], "Fallback?");
    assert_eq!(result["user_response"], "ans");
}

#[test]
fn batch_ids_stable_and_model_id_echoed() {
    let (normalized, error) = normalize_questions(&json!([
        {"id": "model-9", "question": "Q1?", "choices": ["a", "b"]},
        {"question": "Q2?"},
    ]));
    assert!(error.is_none());
    let normalized = normalized.unwrap();
    assert_eq!(normalized[0].qid, "q0");
    assert_eq!(normalized[0].id.as_deref(), Some("model-9"));
    assert_eq!(
        normalized[0].choices_offered,
        Some(vec!["a".to_string(), "b".to_string()])
    );
    assert_eq!(normalized[1].qid, "q1");
    assert!(normalized[1].id.is_none());
}

#[test]
fn batch_callback_receives_list_once() {
    let calls = std::sync::Arc::new(std::sync::Mutex::new(0usize));
    let calls_cb = std::sync::Arc::clone(&calls);
    let cb = ClarifyCallback::Batch(std::sync::Arc::new(move |_, questions| {
        *calls_cb.lock().unwrap() += 1;
        assert_eq!(questions.len(), 2);
        serde_json::to_string(&json!({"answers": {"q0": "a", "q1": "b"}})).unwrap()
    }));
    let result: Value = serde_json::from_str(&clarify_tool(
        "Title",
        None,
        false,
        Some(&json!([{"question": "Q1?"}, {"question": "Q2?"}])),
        Some(cb),
    ))
    .unwrap();
    assert_eq!(*calls.lock().unwrap(), 1);
    assert_eq!(result["responses"][0]["user_response"], "a");
    assert_eq!(result["responses"][1]["user_response"], "b");
    assert!(result.get("timed_out").is_none());
}

#[test]
fn batch_timed_out_flag_passthrough_with_partials() {
    let cb = ClarifyCallback::Batch(std::sync::Arc::new(|_, _| {
        serde_json::to_string(&json!({"answers": {"q0": "a"}, "timed_out": true})).unwrap()
    }));
    let result: Value = serde_json::from_str(&clarify_tool(
        "",
        None,
        false,
        Some(&json!([{"question": "Q1?"}, {"question": "Q2?"}])),
        Some(cb),
    ))
    .unwrap();
    assert_eq!(result["timed_out"], true);
    assert_eq!(result["responses"][0]["user_response"], "a");
    assert_eq!(result["responses"][1]["user_response"], "");
}

#[test]
fn batch_multi_select_answer_parsed_to_list() {
    let cb = ClarifyCallback::Batch(std::sync::Arc::new(|_, _| {
        serde_json::to_string(&json!({"answers": {"q0": "[\"x\", \"y\"]"}})).unwrap()
    }));
    let result: Value = serde_json::from_str(&clarify_tool(
        "",
        None,
        false,
        Some(&json!([{"question": "Q?", "choices": ["x", "y"], "multi_select": true}])),
        Some(cb),
    ))
    .unwrap();
    assert_eq!(result["responses"][0]["user_response"], json!(["x", "y"]));
}

#[test]
fn legacy_loop_aborts_on_timeout_and_keeps_partials() {
    let cb = ClarifyCallback::Single(std::sync::Arc::new(|_, _, _| TIMEOUT_RESPONSE.to_string()));
    let result: Value = serde_json::from_str(&clarify_tool(
        "",
        None,
        false,
        Some(&json!([{"question": "Q1?"}, {"question": "Q2?"}])),
        Some(cb),
    ))
    .unwrap();
    assert_eq!(result["timed_out"], true);
    assert_eq!(result["responses"][0]["user_response"], "");
    assert_eq!(result["responses"][1]["user_response"], "");
}

#[test]
fn legacy_loop_skip_continues() {
    let cb = ClarifyCallback::Single(std::sync::Arc::new(|q, _, _| {
        if q == "Q1?" {
            "".to_string()
        } else {
            "b".to_string()
        }
    }));
    let result: Value = serde_json::from_str(&clarify_tool(
        "",
        None,
        false,
        Some(&json!([{"question": "Q1?"}, {"question": "Q2?"}])),
        Some(cb),
    ))
    .unwrap();
    assert!(result.get("timed_out").is_none());
    assert_eq!(result["responses"][0]["user_response"], "");
    assert_eq!(result["responses"][1]["user_response"], "b");
}

#[test]
fn single_question_result_shape_unchanged() {
    // The legacy single path keeps its flat shape (no `responses` key).
    let result = ask("Q?", Some(vec![json!("a")]), false, single("a"));
    assert_eq!(result["question"], "Q?");
    assert!(result.get("responses").is_none());
}

// ── schema ─────────────────────────────────────────────────────────────

#[test]
fn schema_advertises_batching() {
    let schema = hermes_tools::clarify::CLARIFY_SCHEMA.clone();
    assert_eq!(schema["name"], "clarify");
    assert_eq!(schema["parameters"]["required"], json!(["questions"]));
    assert_eq!(
        schema["parameters"]["properties"]["questions"]["maxItems"],
        MAX_QUESTIONS
    );
    assert!(schema["description"]
        .as_str()
        .unwrap()
        .contains("INDEPENDENT"));
}

#[test]
fn timeout_sentinel_and_max_constants() {
    assert_eq!(MAX_CHOICES, 4);
    assert_eq!(MAX_QUESTIONS, 5);
    assert!(is_timeout(None));
    assert!(!is_timeout(Some(&json!("x"))));
    assert!(strip_recommended("a (recommended)") == "a");
}
