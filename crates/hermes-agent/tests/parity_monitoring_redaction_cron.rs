//! Parity tests for `agent/monitoring/redaction.py` and
//! `agent/monitoring/cron_health.py` (pure projections) @ b9aa928.
//!
//! Upstream has no dedicated test files (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle.

use std::sync::{Arc, Mutex};

use hermes_agent::monitoring::emitter::{get_emitter, reset_emitter_for_tests};
use serde_json::json;

use hermes_agent::monitoring::cron_health::{
    classify_cron_error, emit_execution_state, job_key, project_execution_event,
};
use hermes_agent::monitoring::redaction::redact_for_export;

// ── redaction ────────────────────────────────────────────────────────────

#[test]
fn none_passes_through() {
    assert_eq!(redact_for_export(None), None);
}

#[test]
fn bearer_and_token_shapes_are_redacted() {
    // PARITY @ 5d59366 (live oracle): the force-pass masks the bearer token
    // shape; the egress sweep (20-char floor) leaves the masked form alone.
    // The old `[redacted]` fold was the pre-refactor inline sweep (removed
    // upstream when secrets moved to `redact_for_egress`).
    let out = redact_for_export(Some("Authorization: Bearer abc123.def_ghi")).unwrap();
    assert!(!out.contains("abc123"), "{out}");
    assert!(out.contains("Bearer ***"), "{out}");

    for token in ["sk-abcdefghijklmnop", "ghp_abcdefghijkl", "xoxb-1234567890"] {
        let out = redact_for_export(Some(&format!("token is {token} end"))).unwrap();
        // The base redactor (force=true) masks the token first — possibly
        // to a partial-mask form the shape regexes then pass through; the
        // contract is only that the raw secret never survives.
        assert!(!out.contains(token), "{token}: {out}");
    }
}

#[test]
fn pii_is_classified_not_dropped() {
    let out = redact_for_export(Some("contact me at User.Name+tag@example.co.uk please")).unwrap();
    assert!(out.contains("[email]"), "{out}");
    assert!(!out.contains("User.Name"), "{out}");

    let out = redact_for_export(Some(
        "job ran for 550e8400-e29b-41d4-a716-446655440000 seconds-ish",
    ))
    .unwrap();
    assert!(out.contains("[id]"), "{out}");

    let out = redact_for_export(Some("call +1 415 555 0123 today")).unwrap();
    assert!(out.contains("[phone]"), "{out}");
}

#[test]
fn scrub_is_unconditional_including_masks() {
    // PARITY @ 5d59366 (live oracle): the `\*{3,}` literal sweep is gone
    // upstream (it lived in the removed inline sweep); asterisk runs pass
    // through when the force-pass finds no secret shape.
    let out = redact_for_export(Some("value: ****")).unwrap();
    assert_eq!(out, "value: ****", "{out}");
    // Empty string stays empty (still Some — only None maps to None).
    assert_eq!(redact_for_export(Some("")), Some(String::new()));
}

// ── cron_health: pure projections ────────────────────────────────────────

#[test]
fn job_key_is_a_stable_sha256_prefix() {
    let key = job_key(Some(&json!("my-cron-job")));
    assert!(key.starts_with("sha256:"));
    assert_eq!(key.len(), "sha256:".len() + 24);
    assert_eq!(job_key(Some(&json!("my-cron-job"))), key, "stable");
    assert_ne!(job_key(Some(&json!("other-job"))), key);
    // Falsy inputs hash the literal "unknown".
    assert_eq!(job_key(None), job_key(Some(&json!("unknown"))));
    assert_eq!(
        job_key(Some(&json!(null))),
        job_key(Some(&json!("unknown")))
    );
}

#[test]
fn classify_cron_error_vocabulary() {
    assert_eq!(
        classify_cron_error(Some(&json!("401 Unauthorized for resource"))),
        "auth_failed"
    );
    assert_eq!(
        classify_cron_error(Some(&json!("Forbidden"))),
        "auth_failed"
    );
    assert_eq!(
        classify_cron_error(Some(&json!("refresh token expired"))),
        "auth_failed"
    );
    assert_eq!(
        classify_cron_error(Some(&json!("HTTP 429: rate limit hit"))),
        "rate_limited"
    );
    assert_eq!(
        classify_cron_error(Some(&json!("quota exceeded"))),
        "rate_limited"
    );
    assert_eq!(
        classify_cron_error(Some(&json!("request timeout"))),
        "timeout"
    );
    assert_eq!(
        classify_cron_error(Some(&json!("connection refused"))),
        "network_error"
    );
    assert_eq!(
        classify_cron_error(Some(&json!("dns lookup failed"))),
        "network_error"
    );
    assert_eq!(
        classify_cron_error(Some(&json!("dispatch queue full"))),
        "dispatch_failed"
    );
    assert_eq!(
        classify_cron_error(Some(&json!("owner exited mid-run"))),
        "interrupted"
    );
    assert_eq!(
        classify_cron_error(Some(&json!("empty response from model"))),
        "empty_response"
    );
    assert_eq!(
        classify_cron_error(Some(&json!("missing schedule key"))),
        "invalid_config"
    );
    assert_eq!(
        classify_cron_error(Some(&json!("exploded spectacularly"))),
        "unknown"
    );
    // `str(raw or "")` — None/empty classify as unknown.
    assert_eq!(classify_cron_error(None), "unknown");
    assert_eq!(classify_cron_error(Some(&json!(""))), "unknown");
}

#[test]
fn project_execution_event_normalizes_vocabularies() {
    // Everything present and known.
    let record = json!({
        "status": "completed",
        "source": "builtin",
        "job_id": "nightly",
        "started_at": "2026-08-31T10:00:00Z",
        "finished_at": "2026-08-31T10:00:05Z",
    });
    let event = project_execution_event(&record, Some("delivered"));
    assert_eq!(event.status, "completed");
    assert_eq!(event.source, "builtin");
    assert_eq!(event.duration_ms, Some(5000));
    assert_eq!(event.delivery_outcome.as_deref(), Some("delivered"));
    assert_eq!(event.error_class, None, "no error class on success");

    // Unknown source becomes external; unknown status wins the error class.
    let record = json!({
        "status": "went sideways",
        "source": "some-plugin",
        "job_id": "j-2",
        "error": "403 forbidden",
    });
    let event = project_execution_event(&record, Some("whatever"));
    assert_eq!(event.status, "unknown");
    assert_eq!(event.source, "external");
    assert_eq!(event.error_class.as_deref(), Some("auth_failed"));
    assert_eq!(
        event.delivery_outcome, None,
        "unknown outcome drops to None"
    );
}

#[test]
fn duration_edges() {
    // claimed_at used when started_at missing; negative clamps to 0.
    let record = json!({
        "claimed_at": "2026-08-31T10:00:10Z",
        "finished_at": "2026-08-31T10:00:02Z",
    });
    let event = project_execution_event(&record, None);
    assert_eq!(event.duration_ms, Some(0));
    // Missing endpoints -> None.
    let event = project_execution_event(&json!({"status": "ok"}), None);
    assert_eq!(event.duration_ms, None);
}

#[test]
fn emit_execution_state_is_a_silent_noop_on_falsy_input() {
    // Best-effort: no record, no emit, no panic.
    emit_execution_state(None, None);
    emit_execution_state(Some(&json!(null)), None);
}

#[test]
fn terminal_states_flush_through_the_singleton() {
    // End-to-end through the process-wide emitter: attach a subscriber,
    // emit a terminal state, and observe the flushed event.
    reset_emitter_for_tests(None);
    let sink: Arc<Mutex<Vec<serde_json::Value>>> = Arc::default();
    let sink_cb = Arc::clone(&sink);
    {
        let emitter = get_emitter();
        emitter.subscribe(Arc::new(move |batch: &[serde_json::Value]| {
            sink_cb.lock().unwrap().extend(batch.iter().cloned());
        }));
        let record = json!({
            "status": "failed",
            "source": "direct",
            "job_id": "nightly",
            "error": "401 unauthorized",
        });
        emit_execution_state(Some(&record), Some("failed"));
    }
    reset_emitter_for_tests(None);
    let sink = sink.lock().unwrap();
    assert_eq!(sink.len(), 1, "terminal state crossed the barrier");
    assert_eq!(sink[0]["event"], "cron_execution");
    assert_eq!(sink[0]["status"], "failed");
    assert_eq!(sink[0]["error_class"], "auth_failed");
    assert!(sink[0]["job_key"].as_str().unwrap().starts_with("sha256:"));
}

/// PARITY @ 5d59366: delivery outcomes extended (+queued, +suppressed_acked).
#[test]
fn extended_delivery_outcomes_pass_through() {
    use hermes_agent::monitoring::cron_health::project_execution_event;
    use serde_json::json;
    for outcome in ["queued", "suppressed_acked"] {
        let event = project_execution_event(
            &json!({"status": "completed", "job_id": "j"}),
            Some(outcome),
        );
        assert_eq!(event.delivery_outcome.as_deref(), Some(outcome));
    }
    // Unknown outcomes still drop to None.
    let event = project_execution_event(
        &json!({"status": "completed", "job_id": "j"}),
        Some("bogus"),
    );
    assert_eq!(event.delivery_outcome, None);
}
