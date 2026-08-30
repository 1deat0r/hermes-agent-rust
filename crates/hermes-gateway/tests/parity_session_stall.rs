//! Parity tests for `gateway/session_stall.py` @ b9aa928, mirroring the
//! direct policy cases in upstream
//! `tests/gateway/test_session_stall_watchdog.py` (the `GatewayRunner`
//! notify-once cases belong to `gateway.run`, which is not yet ported).

use serde_json::json;

use hermes_gateway::session_stall::{
    format_session_stall_notification, resolve_session_idle_seconds_from_activity,
    should_clear_session_stall_notification, should_emit_session_stall_notification,
};

fn snapshot(activity: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    activity.as_object().unwrap().clone()
}

#[test]
fn should_emit_requires_pending_and_idle() {
    assert!(should_emit_session_stall_notification(
        300.0,
        Some(400.0),
        true,
        false
    ));
    assert!(!should_emit_session_stall_notification(
        300.0,
        Some(400.0),
        false,
        false
    ));
    assert!(!should_emit_session_stall_notification(
        300.0,
        Some(100.0),
        true,
        false
    ));
    assert!(!should_emit_session_stall_notification(
        0.0,
        Some(9999.0),
        true,
        false
    ));
    assert!(!should_emit_session_stall_notification(
        300.0,
        Some(400.0),
        true,
        true
    ));
}

#[test]
fn should_clear_when_pending_gone_or_activity_resumes() {
    assert!(should_clear_session_stall_notification(
        300.0,
        Some(400.0),
        false
    ));
    assert!(should_clear_session_stall_notification(
        300.0,
        Some(10.0),
        true
    ));
    assert!(!should_clear_session_stall_notification(
        300.0,
        Some(400.0),
        true
    ));
}

#[test]
fn should_clear_holds_latch_when_idle_unknown() {
    assert!(!should_clear_session_stall_notification(300.0, None, true));
}

#[test]
fn format_session_stall_notification_minutes() {
    let msg = format_session_stall_notification(125.0);
    assert!(msg.contains("2 min ago"), "{msg}");
    assert!(msg.contains("/new"));
    assert_eq!(
        format_session_stall_notification(30.0)
            .matches("1 min ago")
            .count(),
        1
    );
}

#[test]
fn resolve_idle_uses_shared_activity_snapshot_only() {
    // Upstream builds the snapshot via build_activity_snapshot (the #72039
    // contract); the Rust port reads the same document shape.
    let now = 1_000_000.0;
    let snap = hermes_state::activity::build_activity_snapshot(
        Some(now - 120.0),
        Some("tool: terminal"),
        Some("unknown"),
        Some(now),
    )
    .as_object()
    .unwrap()
    .clone();
    assert_eq!(
        resolve_session_idle_seconds_from_activity(Some(&snap), Some(now)),
        Some(120.0)
    );
    assert_eq!(
        resolve_session_idle_seconds_from_activity(None, Some(now)),
        None
    );
    assert_eq!(
        resolve_session_idle_seconds_from_activity(Some(&serde_json::Map::new()), Some(now)),
        None
    );
}

#[test]
fn resolve_idle_prefers_seconds_since_activity_field() {
    let activity = snapshot(json!({
        "seconds_since_activity": 42.5,
        "last_activity_at": 1.0, // must be ignored when seconds present
    }));
    assert_eq!(
        resolve_session_idle_seconds_from_activity(Some(&activity), Some(999.0)),
        Some(42.5)
    );
}

#[test]
fn resolve_idle_falls_back_to_last_activity_fields() {
    // seconds_since_activity absent -> last_activity_at; then
    // last_activity_ts.
    let via_at = snapshot(json!({ "last_activity_at": 100.0 }));
    assert_eq!(
        resolve_session_idle_seconds_from_activity(Some(&via_at), Some(150.0)),
        Some(50.0)
    );
    let via_ts = snapshot(json!({ "last_activity_ts": 100.0 }));
    assert_eq!(
        resolve_session_idle_seconds_from_activity(Some(&via_ts), Some(150.0)),
        Some(50.0)
    );
}

#[test]
fn resolve_idle_rejects_unusable_progress() {
    // Numeric strings float() fine; non-numeric strings do not.
    let str_idle = snapshot(json!({ "seconds_since_activity": "12.5" }));
    assert_eq!(
        resolve_session_idle_seconds_from_activity(Some(&str_idle), Some(999.0)),
        Some(12.5)
    );
    let bad_idle = snapshot(json!({ "seconds_since_activity": "soon", "last_activity_at": 1.0 }));
    assert_eq!(
        resolve_session_idle_seconds_from_activity(Some(&bad_idle), Some(100.0)),
        Some(99.0),
        "non-numeric seconds falls through to last_activity_at"
    );
    let nonfinite =
        snapshot(json!({ "seconds_since_activity": f64::INFINITY, "last_activity_at": 1.0 }));
    assert_eq!(
        resolve_session_idle_seconds_from_activity(Some(&nonfinite), Some(101.0)),
        Some(100.0),
        "non-finite seconds falls through"
    );
    let bad_only = snapshot(json!({ "seconds_since_activity": "soon" }));
    assert_eq!(
        resolve_session_idle_seconds_from_activity(Some(&bad_only), Some(101.0)),
        None
    );
    let negative = snapshot(json!({ "seconds_since_activity": -5.0 }));
    assert_eq!(
        resolve_session_idle_seconds_from_activity(Some(&negative), Some(999.0)),
        Some(0.0),
        "negative idle clamps to 0.0"
    );
    let bad_ts = snapshot(json!({ "last_activity_at": "not-a-ts" }));
    assert_eq!(
        resolve_session_idle_seconds_from_activity(Some(&bad_ts), Some(101.0)),
        None
    );
}
