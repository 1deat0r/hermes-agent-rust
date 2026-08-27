// Tier: unit — mirrors tests/agent/test_turn_retry_state.py.

use hermes_agent::turn_retry_state::{TurnRetryState, TURN_RETRY_STATE_FIELDS};

const EXPECTED_FIELDS: [&str; 20] = [
    "codex_auth_retry_attempted",
    "anthropic_auth_retry_attempted",
    "nous_auth_retry_attempted",
    "nous_paid_entitlement_refresh_attempted",
    "copilot_auth_retry_attempted",
    "copilot_stale_cred_retry_attempted",
    "vertex_auth_retry_attempted",
    "thinking_sig_retry_attempted",
    "invalid_encrypted_content_retry_attempted",
    "image_shrink_retry_attempted",
    "multimodal_tool_content_retry_attempted",
    "oauth_1m_beta_retry_attempted",
    "llama_cpp_grammar_retry_attempted",
    "primary_recovery_attempted",
    "has_retried_429",
    "auth_failover_attempted",
    "restart_with_compressed_messages",
    "restart_with_length_continuation",
    "restart_with_rebuilt_messages",
    "restart_with_redirected_messages",
];

#[test]
fn field_set_matches_contract() {
    let declared: std::collections::BTreeSet<&str> = EXPECTED_FIELDS.iter().copied().collect();
    let actual: std::collections::BTreeSet<&str> =
        TURN_RETRY_STATE_FIELDS.iter().copied().collect();
    assert_eq!(actual, declared);
    assert_eq!(TurnRetryState::field_names().count(), 20);
}

#[test]
fn iteration_follows_the_declaration_order() {
    // Upstream `__iter__` yields dataclass field order, which the loop's debug
    // dumps and tests rely on.
    let names: Vec<&str> = TurnRetryState::default().iter().map(|(n, _)| n).collect();
    assert_eq!(names, EXPECTED_FIELDS.to_vec());
}

#[test]
fn guards_are_independently_mutable() {
    let state = TurnRetryState {
        codex_auth_retry_attempted: true,
        restart_with_compressed_messages: true,
        ..TurnRetryState::default()
    };

    assert!(state.codex_auth_retry_attempted);
    assert!(state.restart_with_compressed_messages);
    // Untouched guards stay false.
    assert!(!state.has_retried_429);
    assert!(!state.anthropic_auth_retry_attempted);
}

#[test]
fn every_field_defaults_to_false_and_reports_its_value() {
    let state = TurnRetryState::default();
    assert!(state.iter().all(|(_, value)| !value));
    let state = TurnRetryState {
        auth_failover_attempted: true,
        ..TurnRetryState::default()
    };
    let flagged: Vec<&str> = state.iter().filter(|(_, v)| *v).map(|(n, _)| n).collect();
    assert_eq!(flagged, vec!["auth_failover_attempted"]);
}
