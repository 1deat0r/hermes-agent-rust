//! Content-free cron service-health and execution telemetry projection.
//!
//! PARITY: `agent/monitoring/cron_health.py` @ b9aa928 — PARTIAL: the pure
//! projections (`classify_cron_error`, `_job_key`, `_parse_time`,
//! `_duration_ms`, `project_execution_event`, `emit_execution_state`) are
//! ported; `build_cron_health_snapshot` / `_is_overdue` stay PENDING until
//! `cron.jobs` / `cron.scheduler` and `gateway_health.GatewayMetric` port
//! (they are thin readers over those stores).
//!
//! Statuses, sources, and delivery outcomes are normalized to the known
//! vocabularies; job keys are content-free SHA-256 prefixes so raw job ids
//! never reach the monitoring plane.

use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;

use super::emitter;
use super::events::CronExecutionEvent;

/// PARITY: `_KNOWN_STATUSES` (upstream line 21).
const KNOWN_STATUSES: [&str; 5] = ["claimed", "running", "completed", "failed", "unknown"];
/// PARITY: `_KNOWN_SOURCES` (upstream line 22).
const KNOWN_SOURCES: [&str; 3] = ["builtin", "direct", "external"];
/// PARITY: `_KNOWN_DELIVERY_OUTCOMES` (upstream line 26 @ 5d59366).
const KNOWN_DELIVERY_OUTCOMES: [&str; 6] = [
    "queued",
    "delivered",
    "failed",
    "suppressed",
    "suppressed_acked",
    "not_configured",
];

/// PARITY: `_job_key` (upstream line 37 @ 5d59366) — now an alias:
/// `_job_key = _safe_instance_id` (same `sha256:<24 hex>` shape, never the
/// raw id). Delegates so the two can never drift.
pub fn job_key(raw: Option<&Value>) -> String {
    super::gateway_health::safe_instance_id(raw)
}

/// PARITY: `classify_cron_error` (upstream lines 36-63) — a first-match
/// cascade over the lowercased error text.
pub fn classify_cron_error(raw: Option<&Value>) -> String {
    let text = match raw {
        Some(Value::String(s)) => s.to_lowercase(),
        Some(other) if !other.is_null() => other.to_string().to_lowercase(),
        _ => String::new(),
    };
    let contains = |needles: &[&str]| needles.iter().any(|n| text.contains(n));
    let has_word = |words: &[&str]| words.iter().any(|word| simple_word_match(&text, word));

    if has_word(&[
        "authentication",
        "authenticated",
        "authenticate",
        "authorization",
        "authorized",
        "authorize",
        "unauthorized",
        "forbidden",
    ]) || has_word(&["bearer"])
        || has_word(&["access token", "api token", "refresh token"])
        || contains(&["401", "403"])
    {
        return "auth_failed".to_string();
    }
    if contains(&["rate limit", "429", "quota"]) {
        return "rate_limited".to_string();
    }
    if contains(&["timeout", "timed out"]) {
        return "timeout".to_string();
    }
    if contains(&["network", "connection", "dns", "socket", "unreachable"]) {
        return "network_error".to_string();
    }
    if contains(&["dispatch", "executor"]) {
        return "dispatch_failed".to_string();
    }
    if contains(&["interrupt", "owner exited", "restarted"]) {
        return "interrupted".to_string();
    }
    if contains(&["empty response"]) {
        return "empty_response".to_string();
    }
    if contains(&["config", "missing", "invalid"]) {
        return "invalid_config".to_string();
    }
    "unknown".to_string()
}

/// `\b<needle>\b` via a small scan (avoids compiling a regex per call in
/// the hot classify path).
fn simple_word_match(text: &str, word: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = text[start..].find(word) {
        let abs = start + pos;
        let before_ok = abs == 0
            || !text[..abs]
                .chars()
                .next_back()
                .map(|c| c.is_alphanumeric() || c == '_')
                .unwrap_or(false);
        let after = abs + word.len();
        let after_ok = after >= text.len()
            || !text[after..]
                .chars()
                .next()
                .map(|c| c.is_alphanumeric() || c == '_')
                .unwrap_or(false);
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

/// PARITY: `_parse_time` (upstream lines 66-70) —
/// `datetime.fromisoformat(str(raw))`, None on failure or falsy input.
/// Naive timestamps are returned without a zone (the caller decides the
/// timeline, mirroring upstream's naive/aware distinction).
fn parse_time(raw: Option<&Value>) -> Option<ParsedTime> {
    let text = match raw {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        _ => return None,
    };
    if let Ok(dt) = DateTime::parse_from_rfc3339(&text) {
        return Some(ParsedTime {
            utc: dt.with_timezone(&Utc).timestamp_millis(),
            naive: None,
        });
    }
    for format in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%d"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(&text, format) {
            return Some(ParsedTime {
                utc: 0,
                naive: Some(naive.and_utc().timestamp_millis()),
            });
        }
    }
    None
}

#[derive(Debug, Clone, Copy)]
struct ParsedTime {
    utc: i64,
    naive: Option<i64>,
}

impl ParsedTime {
    fn millis(&self) -> i64 {
        self.naive.unwrap_or(self.utc)
    }
}

/// PARITY: `_duration_ms` (upstream lines 73-85) — finished −
/// started/claimed, clamped at zero, `None` when either endpoint is
/// missing.
fn duration_ms(record: &Value) -> Option<i64> {
    let obj = record.as_object()?;
    let start = parse_time(obj.get("started_at"))
        .or_else(|| parse_time(obj.get("claimed_at")))
        .map(|t| t.millis())?;
    let finish = parse_time(obj.get("finished_at")).map(|t| t.millis())?;
    Some((finish - start).max(0))
}

/// PARITY: `project_execution_event` (upstream lines 88-110) — vocabulary
/// normalization, the external-source fallback, and error classification
/// only for failed/unknown statuses.
pub fn project_execution_event(
    record: &Value,
    delivery_outcome: Option<&str>,
) -> CronExecutionEvent {
    let obj = record.as_object();
    let status = obj
        .and_then(|o| o.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_lowercase();
    let status = if KNOWN_STATUSES.contains(&status.as_str()) {
        status
    } else {
        "unknown".to_string()
    };
    let source = obj
        .and_then(|o| o.get("source"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_lowercase();
    let source = if source == "unknown" {
        source
    } else if KNOWN_SOURCES.contains(&source.as_str()) {
        source
    } else {
        // Unknown non-empty sources are "external", not "unknown" — the
        // upstream branch order matters (`if source not in KNOWN and
        // source != "unknown"`).
        "external".to_string()
    };
    let outcome = delivery_outcome
        .map(|o| o.to_lowercase())
        .filter(|o| KNOWN_DELIVERY_OUTCOMES.contains(&o.as_str()));

    let mut event =
        CronExecutionEvent::new(status.clone(), job_key(obj.and_then(|o| o.get("job_id"))));
    event.source = source;
    event.duration_ms = duration_ms(record);
    event.delivery_outcome = outcome;
    event.error_class = if status == "failed" || status == "unknown" {
        Some(classify_cron_error(obj.and_then(|o| o.get("error"))))
    } else {
        None
    };
    event
}

/// Best-effort lifecycle emit; terminal states synchronously cross the
/// queue barrier.
///
/// PARITY: `emit_execution_state` (upstream lines 113-128). All failures
/// degrade to a debug log — telemetry can never break the caller.
pub fn emit_execution_state(record: Option<&Value>, delivery_outcome: Option<&str>) {
    let Some(record) = record else {
        return;
    };
    if record.is_null() {
        return;
    }
    let event = project_execution_event(record, delivery_outcome);
    let target = emitter::get_emitter();
    target.emit(&event);
    if event.status == "completed" || event.status == "failed" || event.status == "unknown" {
        target.flush(1.0);
    }
}
