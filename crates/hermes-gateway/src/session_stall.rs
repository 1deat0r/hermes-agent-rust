//! Gateway session stall notification policy (#72016 item 2).
//!
//! PARITY: `gateway/session_stall.py` @ b9aa928 (whole module).
//!
//! Consumes the shared activity observation contract from
//! `agent.session_activity` / `AIAgent.get_activity_summary()` (#72039) as
//! the **single progress source**. This module owns only the notify-once
//! policy for "pending inbound + stale progress"; it does not invent a
//! parallel progress clock from turn-start or inbound event timestamps.
//!
//! Boundaries (kept separate upstream):
//! - `gateway/shutdown_watchdog.py` — process / event-loop liveness
//! - `gateway/delivery_ledger.py` — outbound delivery obligations
//! - Pending inbound here is a stall *policy gate* (queued follow-up
//!   exists), not an outbound obligation and not a progress timestamp.

use serde_json::Value;

/// Python `float(x)` over a JSON scalar: numbers directly, numeric strings
/// via parse. Bools are rejected (Python `float(True)` is `1.0`, but the
/// snapshot contract never emits bools here and serde separates them).
fn json_number_like(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Return true when a stall warning should be sent for this session.
///
/// PARITY: `should_emit_session_stall_notification` (upstream lines 26-41).
pub fn should_emit_session_stall_notification(
    timeout_seconds: f64,
    idle_seconds: Option<f64>,
    has_pending_inbound: bool,
    already_notified: bool,
) -> bool {
    if timeout_seconds <= 0.0 {
        return false;
    }
    if !has_pending_inbound {
        return false;
    }
    if already_notified {
        return false;
    }
    let Some(idle_seconds) = idle_seconds else {
        return false;
    };
    idle_seconds >= timeout_seconds
}

/// Return true when a prior stall notice may be cleared (episode ended).
///
/// PARITY: `should_clear_session_stall_notification` (upstream lines
/// 44-55). Unknown progress holds the latch — observation gaps are never
/// treated as recovery.
pub fn should_clear_session_stall_notification(
    timeout_seconds: f64,
    idle_seconds: Option<f64>,
    has_pending_inbound: bool,
) -> bool {
    if !has_pending_inbound {
        return true;
    }
    if timeout_seconds <= 0.0 {
        return true;
    }
    let Some(idle_seconds) = idle_seconds else {
        return false;
    };
    idle_seconds < timeout_seconds
}

/// User-facing stall warning (ASCII minutes; matches issue #72016 copy).
///
/// PARITY: `format_session_stall_notification` (upstream lines 58-65).
/// `int(idle_seconds // 60)` is a floor divide, clamped to a minimum of 1
/// minute.
pub fn format_session_stall_notification(idle_seconds: f64) -> String {
    let mins = ((idle_seconds / 60.0).floor() as i64).max(1);
    format!("⚠️ Agent session appears stalled (last activity {mins} min ago). Try /new to reset.")
}

/// Idle seconds from a shared activity snapshot only (#72039 contract).
///
/// PARITY: `resolve_session_idle_seconds_from_activity` (upstream lines
/// 68-115). Prefers `seconds_since_activity` when present and finite;
/// otherwise derives from `last_activity_at` / `last_activity_ts`. Returns
/// `None` when there is no usable progress timestamp — callers must not
/// fall back to turn-start or pending-inbound clocks.
pub fn resolve_session_idle_seconds_from_activity(
    activity: Option<&serde_json::Map<String, Value>>,
    now: Option<f64>,
) -> Option<f64> {
    let activity = activity?;
    if activity.is_empty() {
        // Python `if not activity:` — an empty mapping is also falsy.
        return None;
    }

    if let Some(elapsed) = activity.get("seconds_since_activity") {
        if !elapsed.is_null() {
            // try: float(elapsed) — non-numeric values fall through to the
            // last_activity_at / last_activity_ts arm (the except arm keeps
            // going); non-finite floats fall through too. `float()` accepts
            // numeric strings, so string values parse as well.
            let idle = match json_number_like(elapsed) {
                Some(idle) if idle.is_finite() => Some(idle),
                _ => None,
            };
            if let Some(idle) = idle {
                return Some(if idle < 0.0 { 0.0 } else { idle });
            }
        }
    }

    let ts = activity
        .get("last_activity_at")
        .filter(|v| !v.is_null())
        .or_else(|| activity.get("last_activity_ts").filter(|v| !v.is_null()));
    let Some(ts) = ts else {
        return None;
    };
    // try: float(ts) — a failure here returns None outright.
    let when = match json_number_like(ts) {
        Some(when) if when.is_finite() => when,
        _ => return None,
    };

    let clock = now.unwrap_or_else(|| hermes_time::now().timestamp() as f64);
    let idle = clock - when;
    Some(if idle < 0.0 { 0.0 } else { idle })
}
