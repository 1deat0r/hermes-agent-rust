//! Parity tests for the Jev System One layer (tier: unit).
//!
//! Oracles: TypeSafe docs `api.md` / `primitives/{choice,noul,score}.md`
//! (@ 2026-09-20) for the wire shapes, and the live harness semantics in
//! `agent/jev_choice.py::validate_choice`,
//! `agent/auxiliary_client.py::_validate_jev_route_answer`,
//! `agent/compression_scored_prune.py::scrub`, and
//! `agent/jev_review_gate.py` fail-OPEN verdicts (all newer than pin
//! `5d59366`, so this crate is additive, not a 1:1 port).

use hermes_jev::{
    guardrail,
    questions::{validate_choice_answer, validate_noul_answer, validate_score_answer, Question},
    router, stop_hook, tool_route,
    transport::{self, JevError},
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;

// ---- choice validation (ports validate_choice exactly) ----

fn fast_full_ids() -> Vec<&'static str> {
    vec!["FAST", "FULL"]
}

#[test]
fn choice_valid_answer_passes() {
    let answer = json!({
        "choice": "FAST",
        "probabilities": {"FAST": 0.8, "FULL": 0.2},
        "confidence": 0.6,
    });
    let parsed = validate_choice_answer(&fast_full_ids(), &answer).expect("valid");
    assert_eq!(parsed.choice, "FAST");
    assert_eq!(parsed.probabilities["FAST"], 0.8);
}

#[test]
fn choice_unknown_id_rejected() {
    let answer = json!({
        "choice": "CHEAP",
        "probabilities": {"FAST": 0.5, "FULL": 0.5},
        "confidence": 0.0,
    });
    assert!(validate_choice_answer(&fast_full_ids(), &answer).is_none());
}

#[test]
fn choice_probability_key_mismatch_rejected() {
    let answer = json!({
        "choice": "FAST",
        "probabilities": {"FAST": 1.0},
        "confidence": 1.0,
    });
    assert!(validate_choice_answer(&fast_full_ids(), &answer).is_none());
}

#[test]
fn choice_non_finite_rejected() {
    let answer = json!({
        "choice": "FAST",
        "probabilities": {"FAST": 0.8, "FULL": f64::NAN},
        "confidence": 0.6,
    });
    assert!(validate_choice_answer(&fast_full_ids(), &answer).is_none());
}

#[test]
fn choice_out_of_range_rejected() {
    let answer = json!({
        "choice": "FAST",
        "probabilities": {"FAST": 1.2, "FULL": -0.2},
        "confidence": 0.6,
    });
    assert!(validate_choice_answer(&fast_full_ids(), &answer).is_none());
}

#[test]
fn choice_bad_sum_rejected() {
    let answer = json!({
        "choice": "FAST",
        "probabilities": {"FAST": 0.8, "FULL": 0.1},
        "confidence": 0.6,
    });
    assert!(validate_choice_answer(&fast_full_ids(), &answer).is_none());
}

#[test]
fn choice_top_must_equal_choice() {
    let answer = json!({
        "choice": "FULL",
        "probabilities": {"FAST": 0.8, "FULL": 0.2},
        "confidence": 0.6,
    });
    assert!(validate_choice_answer(&fast_full_ids(), &answer).is_none());
}

#[test]
fn choice_bool_is_not_a_number() {
    let answer = json!({
        "choice": "FAST",
        "probabilities": {"FAST": true, "FULL": false},
        "confidence": 1.0,
    });
    assert!(validate_choice_answer(&fast_full_ids(), &answer).is_none());
}

// ---- noul / score validation ----

#[test]
fn noul_valid_range_passes() {
    let parsed = validate_noul_answer(&json!({"type": "noul", "noul": 0.93})).expect("valid");
    assert_eq!(parsed.noul, 0.93);
}

#[test]
fn noul_out_of_range_rejected() {
    assert!(validate_noul_answer(&json!({"noul": 1.5})).is_none());
    assert!(validate_noul_answer(&json!({"noul": -0.1})).is_none());
    assert!(validate_noul_answer(&json!({"noul": "yes"})).is_none());
}

#[test]
fn score_valid_answer_passes() {
    let answer = json!({
        "type": "score",
        "score": 1.43,
        "legend": {"0": "Cosmetic", "1": "Workaround", "2": "Blocking"},
        "probabilities": {"0": 0.0, "1": 0.57, "2": 0.43},
        "confidence": 0.35,
    });
    let parsed = validate_score_answer(&answer).expect("valid");
    assert_eq!(parsed.score, 1.43);
    assert_eq!(parsed.legend.len(), 3);
}

#[test]
fn score_single_level_rejected() {
    let answer = json!({
        "score": 0.0,
        "legend": {"0": "Only"},
        "probabilities": {"0": 1.0},
        "confidence": 1.0,
    });
    assert!(validate_score_answer(&answer).is_none());
}

#[test]
fn score_legend_probability_mismatch_rejected() {
    let answer = json!({
        "score": 1.0,
        "legend": {"0": "Low", "1": "High"},
        "probabilities": {"0": 1.0},
        "confidence": 1.0,
    });
    assert!(validate_score_answer(&answer).is_none());
}

// ---- genuine live-Jev fixtures (2026-09-20, model jev-1.13.0) ----
// (above)

// ---- wire shapes (TypeSafe api.md) ----

#[test]
fn live_router_payload_validates() {
    // Genuine answer to: route "fix the login bug in auth.py" over
    // research/coding/billing agents.
    let answer = json!({
        "type": "choice",
        "choice": "coding_agent",
        "confidence": 1.0,
        "probabilities": {"research_agent": 0.0, "coding_agent": 1.0, "billing_agent": 0.0},
    });
    let ids = ["research_agent", "coding_agent", "billing_agent"];
    let parsed = validate_choice_answer(&ids, &answer).expect("live valid");
    assert_eq!(parsed.choice, "coding_agent");
}

#[test]
fn live_tool_payload_validates() {
    let answer = json!({
        "type": "choice",
        "choice": "read_file",
        "confidence": 0.86,
        "probabilities": {"web_search": 0.0, "none_of_these": 0.02, "terminal": 0.08, "read_file": 0.9},
    });
    let ids = ["web_search", "read_file", "terminal", "none_of_these"];
    let parsed = validate_choice_answer(&ids, &answer).expect("live valid");
    assert_eq!(parsed.choice, "read_file");
}

#[test]
fn live_noul_payloads_validate() {
    for noul in [0.34, 0.84, 0.55, 0.17] {
        let parsed =
            validate_noul_answer(&json!({"type": "noul", "noul": noul})).expect("live valid");
        assert_eq!(parsed.noul, noul);
    }
}

#[test]
fn choice_question_serializes_with_type_and_criteria() {
    let q = router::route_question(
        "prompt",
        &[
            router::RouteOption {
                id: "research_agent".to_string(),
                description: Some("research".to_string()),
            },
            router::RouteOption {
                id: "coding_agent".to_string(),
                description: None,
            },
        ],
    )
    .expect("routable");
    let wire = q.to_json();
    assert_eq!(wire["type"], Value::String("choice".to_string()));
    assert_eq!(
        wire["criteria"]["research_agent"],
        Value::String("research".to_string())
    );
    assert!(wire["criteria"]["coding_agent"].is_null());
}

#[test]
fn noul_question_with_criteria_serializes() {
    let q = Question::Noul(hermes_jev::NoulQuestion {
        instructions: Value::String("Is this urgent?".to_string()),
        criteria_true: Some("time-sensitive".to_string()),
        criteria_false: Some("no urgency".to_string()),
    });
    let wire = q.to_json();
    assert_eq!(wire["type"], Value::String("noul".to_string()));
    assert_eq!(
        wire["criteria"]["true"],
        Value::String("time-sensitive".to_string())
    );
}

// ---- transport: keyless never touches the network ----

fn without_key(f: impl FnOnce()) {
    let prior = env::var(transport::API_KEY_ENV).ok();
    unsafe {
        env::remove_var(transport::API_KEY_ENV);
    }
    f();
    if let Some(value) = prior {
        unsafe {
            env::set_var(transport::API_KEY_ENV, value);
        }
    }
}

#[test]
fn keyless_choice_returns_missing_key() {
    without_key(|| {
        let questions = BTreeMap::new();
        assert_eq!(
            transport::post_system_one(&Value::Null, &questions, transport::JEV_MODEL),
            Err(JevError::MissingKey)
        );
    });
}

#[test]
fn blank_key_counts_as_missing() {
    let prior = env::var(transport::API_KEY_ENV).ok();
    unsafe {
        env::set_var(transport::API_KEY_ENV, "   ");
    }
    let questions = BTreeMap::new();
    let result = transport::post_system_one(&Value::Null, &questions, transport::JEV_MODEL);
    if let Some(value) = prior {
        unsafe {
            env::set_var(transport::API_KEY_ENV, value);
        }
    } else {
        unsafe {
            env::remove_var(transport::API_KEY_ENV);
        }
    }
    assert_eq!(result, Err(JevError::MissingKey));
}

#[test]
fn error_messages_never_carry_key_material() {
    for error in [
        JevError::MissingKey,
        JevError::NoTransport,
        JevError::Connection,
        JevError::HttpStatus(401),
        JevError::BadResponse,
        JevError::EmptyQuestion,
    ] {
        let message = error.to_string();
        assert!(!message.contains("Bearer"), "leak in {message:?}");
        assert!(
            !message.contains("TYPESAFE_API_KEY="),
            "leak in {message:?}"
        );
    }
}

// ---- router / tool / stop / guardrail fail-safe shapes ----

#[test]
fn router_degenerate_roster_never_routes() {
    without_key(|| {
        let one = vec![router::RouteOption {
            id: "only".to_string(),
            description: None,
        }];
        assert!(router::route_request("do it", &one).is_none());
        assert!(router::route_request("do it", &[]).is_none());
    });
}

#[test]
fn router_valid_roster_keyless_never_touches_network() {
    without_key(|| {
        let options = vec![
            router::RouteOption {
                id: "research_agent".to_string(),
                description: Some("research".to_string()),
            },
            router::RouteOption {
                id: "coding_agent".to_string(),
                description: Some("code".to_string()),
            },
        ];
        // Keyless: MissingKey before any POST; no network, legacy None.
        assert!(router::route_request("fix the bug", &options).is_none());
    });
}

#[test]
fn choice_near_tie_within_slack_validates() {
    let answer = json!({
        "choice": "FULL",
        "probabilities": {"FAST": 0.5000005, "FULL": 0.4999995},
        "confidence": 0.0,
    });
    assert!(validate_choice_answer(&fast_full_ids(), &answer).is_some());
}

#[test]
fn tool_question_always_offers_no_tool_outcome() {
    let tools = BTreeMap::from([
        ("web_search".to_string(), "search the web".to_string()),
        ("read_file".to_string(), "read a file".to_string()),
    ]);
    let question = tool_route::tool_question("goal", &tools).expect("question");
    match question {
        Question::Choice(q) => {
            assert!(q.criteria.contains_key(tool_route::NO_TOOL_OPTION));
            assert_eq!(q.criteria.len(), 3);
        }
        _ => panic!("tool selection must be a choice"),
    }
}

#[test]
fn tool_empty_roster_never_selects() {
    without_key(|| {
        assert!(tool_route::select_next_tool("goal", "log", &BTreeMap::new()).is_none());
    });
}

#[test]
fn stop_thresholds_split_three_ways() {
    let stop = stop_hook::StopVerdict {
        satisfied_probability: 0.95,
    };
    let go = stop_hook::StopVerdict {
        satisfied_probability: 0.05,
    };
    let mid = stop_hook::StopVerdict {
        satisfied_probability: 0.5,
    };
    assert_eq!(stop_hook::decide_stop(&stop, 0.9, 0.2), Some(true));
    assert_eq!(stop_hook::decide_stop(&go, 0.9, 0.2), Some(false));
    assert_eq!(stop_hook::decide_stop(&mid, 0.9, 0.2), None);
}

#[test]
fn stop_keyless_returns_none() {
    without_key(|| {
        assert!(stop_hook::check_objective_satisfied("goal", "output").is_none());
    });
}

#[test]
fn guardrail_threshold_gates_shipping() {
    let safe = guardrail::GuardrailVerdict {
        hallucination_probability: 0.05,
        policy_violation_probability: 0.01,
    };
    let risky = guardrail::GuardrailVerdict {
        hallucination_probability: 0.05,
        policy_violation_probability: 0.9,
    };
    assert!(safe.is_safe(0.2));
    assert!(!risky.is_safe(0.2));
}

#[test]
fn guardrail_keyless_escalates() {
    without_key(|| {
        assert!(guardrail::screen_output("output", "evidence").is_none());
    });
}

// ---- scrubber (ports compression_scored_prune.SECRET_PATTERNS) ----

#[test]
fn scrubber_redacts_key_shaped_secrets() {
    assert_eq!(
        guardrail::scrub_text("key sk-abcdefgh12345678 here"),
        "key [redacted-secret] here"
    );
    assert_eq!(
        guardrail::scrub_text("token ghp_abcDEF123 here"),
        "token [redacted-secret] here"
    );
    assert_eq!(
        guardrail::scrub_text("Bearer abcDEF123._~-x here"),
        "[redacted-secret] here"
    );
}

#[test]
fn scrubber_leaves_plain_text_alone() {
    let text = "the router picked coding_agent at 0.8 confidence";
    assert_eq!(guardrail::scrub_text(text), text);
}
