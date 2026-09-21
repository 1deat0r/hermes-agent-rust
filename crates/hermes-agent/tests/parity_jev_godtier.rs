//! God-tier Jev workflow tests, agent-seam side (tier: unit).
//!
//! Oracles: live `agent/jev_review_gate.py`, `agent/turn_context.py`
//! nudge gate, `agent/auxiliary_client.py` Jev routing, and
//! `agent/compression_scored_prune.py` — all newer than pin `5d59366`,
//! so these seams are additive, not 1:1 ports.

use hermes_agent::jev_router;
use serde_json::{json, Value};
use std::env;

fn without_key(f: impl FnOnce()) {
    let prior = env::var(hermes_jev::API_KEY_ENV).ok();
    unsafe {
        env::remove_var(hermes_jev::API_KEY_ENV);
    }
    f();
    if let Some(value) = prior {
        unsafe {
            env::set_var(hermes_jev::API_KEY_ENV, value);
        }
    }
}

#[test]
fn review_gates_default_off_fire_blind() {
    without_key(|| {
        let state = hermes_jev::review_gate::spawn_state("hi", "hello", &[], false, false);
        assert!(hermes_jev::review_gate::should_spawn_review(
            false, 0.99, &state
        ));
        assert!(hermes_jev::review_gate::keep_skill_review(
            false, 0.99, &state
        ));
    });
}

#[test]
fn review_gates_keyless_fail_open_with_zero_network() {
    without_key(|| {
        let state = hermes_jev::review_gate::spawn_state(
            "ship the fix",
            "done",
            &["write_file".to_string()],
            true,
            true,
        );
        // Keyless: MissingKey before any POST — blind behaviour holds.
        assert!(hermes_jev::review_gate::should_spawn_review(
            true, 0.5, &state
        ));
        assert!(hermes_jev::review_gate::keep_skill_review(
            true, 0.5, &state
        ));
    });
}

#[test]
fn nudge_gate_disabled_fires_blind() {
    assert!(hermes_jev::memory_nudge::gate_memory_nudge(
        false, 0.99, "anything", 10
    ));
}

#[test]
fn nudge_gate_keyless_fails_open() {
    without_key(|| {
        assert!(hermes_jev::memory_nudge::gate_memory_nudge(
            true,
            0.5,
            "remember this",
            12
        ));
    });
}

#[test]
fn aux_routing_stays_off_without_opt_in() {
    assert!(!jev_router::task_prefers_jev_routing(
        "approval",
        &json!({})
    ));
    assert!(!jev_router::task_prefers_jev_routing(
        "compression",
        &json!({"prefer_jev_routing": false})
    ));
    assert!(jev_router::task_prefers_jev_routing(
        "title_generation",
        &json!({"prefer_jev_routing": true})
    ));
}

#[test]
fn aux_route_keyless_keeps_legacy() {
    without_key(|| {
        assert_eq!(
            jev_router::ask_jev_route("approval", "openai-codex", "gpt-5.5", Some(500)),
            None
        );
        // Ineligible tasks never route, key or not.
        assert_eq!(jev_router::ask_jev_route("unknown", "p", "m", None), None);
    });
}

#[test]
fn skill_suggest_empty_roster_is_not_failure() {
    let suggestion = hermes_jev::skill_route::suggest("do it", &[]).expect("empty ok");
    assert!(suggestion.names.is_empty());
}

#[test]
fn skill_suggest_keyless_returns_none_for_live_roster() {
    without_key(|| {
        let roster = vec![hermes_jev::skill_route::SkillEntry {
            name: "code-review".to_string(),
            description: "review code changes".to_string(),
        }];
        assert!(hermes_jev::skill_route::suggest("review my diff", &roster).is_none());
    });
}

#[test]
fn triage_degenerate_labels_never_fire() {
    without_key(|| {
        let one = vec![hermes_jev::triage::TriageLabel {
            id: "bug".to_string(),
            description: None,
        }];
        assert!(hermes_jev::triage::triage_item("crash", &one, &[]).is_none());
    });
}

#[test]
fn error_values_carry_no_key_material() {
    for error in [
        hermes_jev::JevError::MissingKey,
        hermes_jev::JevError::HttpStatus(401),
        hermes_jev::JevError::BadResponse,
    ] {
        let message = error.to_string();
        assert!(!message.contains("Bearer"));
        assert!(!message.contains('='));
    }
}

#[test]
fn jev_router_state_shapes_match_live() {
    // Live state carries task/size/provider/model ids, never raw content.
    assert_eq!(jev_router::route_size_bucket(Some(100)), "s");
    assert_eq!(jev_router::route_size_bucket(Some(1999)), "s");
    assert_eq!(jev_router::route_size_bucket(Some(2000)), "m");
    assert_eq!(jev_router::route_size_bucket(Some(19999)), "m");
    assert_eq!(jev_router::route_size_bucket(Some(20000)), "l");
}

#[test]
fn review_state_builder_shapes() {
    let tools = vec!["read_file".to_string(), "terminal".to_string()];
    let state: Value = hermes_jev::review_gate::spawn_state("fix it", "done", &tools, true, false);
    assert_eq!(state["tools_used"], json!(["read_file", "terminal"]));
    assert_eq!(state["review_memory"], json!(true));
    assert_eq!(state["review_skills"], json!(false));
}
