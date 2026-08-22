//! Session activity observation contract (#72016 / #72039) — the durable
//! `last_activity_at` / `last_activity_description` / `last_activity_provenance`
//! columns live on `sessions`, and this module provides the shared helpers
//! used by the SessionDB activity writers and readers.
//!
//! PARITY: agent/session_activity.py @ b9aa928 (106 LOC, ported 1:1).

/// Max length of bounded activity description text.
pub const ACTIVITY_DESCRIPTION_MAX: usize = 120;

/// Durable SessionDB activity heartbeat cadence (seconds between writes per
/// session). Deliberately a code constant — no configuration can turn the
/// heartbeat into a high-frequency writer.
pub const SESSION_ACTIVITY_HEARTBEAT_MIN_INTERVAL_SECONDS: f64 = 60.0;

/// Where a durable/in-memory activity stamp came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityProvenance {
    Unknown,
    AgentCompression,
    AgentCompressionTimeout,
    AgentCompressionCooldown,
}

impl ActivityProvenance {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityProvenance::Unknown => "unknown",
            ActivityProvenance::AgentCompression => "agent.compression",
            ActivityProvenance::AgentCompressionTimeout => "agent.compression_timeout",
            ActivityProvenance::AgentCompressionCooldown => "agent.compression_cooldown",
        }
    }

    fn parse(value: &str) -> Option<ActivityProvenance> {
        match value {
            "unknown" => Some(ActivityProvenance::Unknown),
            "agent.compression" => Some(ActivityProvenance::AgentCompression),
            "agent.compression_timeout" => Some(ActivityProvenance::AgentCompressionTimeout),
            "agent.compression_cooldown" => Some(ActivityProvenance::AgentCompressionCooldown),
            _ => None,
        }
    }
}

/// Clamp free-form activity text to the shared description budget.
// PARITY: agent/session_activity.py bound_activity_description @ b9aa928
pub fn bound_activity_description(description: Option<&str>) -> String {
    let text = description.unwrap_or("").trim();
    if text.chars().count() <= ACTIVITY_DESCRIPTION_MAX {
        return text.to_string();
    }
    // Text is left-prefix truncated with U+2026 as the final character,
    // exactly like the Python slice `text[:MAX-1] + "…"` (characters, not
    // bytes, since Python str is a char sequence).
    let prefix: String = text.chars().take(ACTIVITY_DESCRIPTION_MAX - 1).collect();
    format!("{prefix}\u{2026}")
}

/// Return a known provenance, or `Unknown` when unset/unrecognized.
// PARITY: agent/session_activity.py normalize_activity_provenance @ b9aa928
pub fn normalize_activity_provenance(provenance: Option<&str>) -> ActivityProvenance {
    let value = provenance.unwrap_or("").trim();
    ActivityProvenance::parse(value).unwrap_or(ActivityProvenance::Unknown)
}

/// Build the shared activity snapshot dict (cast to JSON values).
///
/// `now` is injectable for deterministic tests; upstream defaults to
/// `time.time()`.
// PARITY: agent/session_activity.py build_activity_snapshot @ b9aa928
pub fn build_activity_snapshot(
    last_activity_at: Option<f64>,
    last_activity_description: Option<&str>,
    last_activity_provenance: Option<&str>,
    now: Option<f64>,
) -> serde_json::Value {
    use serde_json::{json, Map, Value};
    let when = last_activity_at;
    let clock = now.unwrap_or_else(super::state::now);
    let desc = bound_activity_description(last_activity_description);
    let prov = normalize_activity_provenance(last_activity_provenance);
    let elapsed = when.map(|w| py_round1(clock - w));
    let prov_s = prov.as_str().to_string();
    let mut map = Map::new();
    map.insert("last_activity_at".into(), when.map(Value::from).unwrap_or(Value::Null));
    map.insert("last_activity_description".into(), json!(desc));
    map.insert("last_activity_provenance".into(), json!(prov_s));
    map.insert("seconds_since_activity".into(), elapsed.map(Value::from).unwrap_or(Value::Null));
    map.insert("last_activity_ts".into(), when.map(Value::from).unwrap_or(Value::Null));
    map.insert("last_activity_desc".into(), json!(desc));
    map.insert("description".into(), json!(desc));
    map.insert("provenance".into(), json!(prov_s));
    Value::Object(map)
}

/// Python's `round(v, 1)`: nearest multiple of 0.1 with ties-to-even —
/// Rust's f64::round() is half-away-from-zero and would diverge at exact
/// .x5 boundaries (Python floats store the same binary values, so
/// replicating its tie rule keeps `seconds_since_activity` identical).
fn py_round1(v: f64) -> f64 {
    let scaled = v * 10.0;
    let floor = scaled.floor();
    let frac = scaled - floor;
    let bump = frac > 0.5 || (frac == 0.5 && (floor as i64) % 2 != 0);
    let rounded = if bump { floor + 1.0 } else { floor };
    rounded / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_description_at_120_chars() {
        let long = "x".repeat(200);
        let bound = bound_activity_description(Some(&long));
        assert_eq!(bound.chars().count(), 120);
        assert!(bound.ends_with('\u{2026}'));
    }

    #[test]
    fn normalizes_unknown_and_known() {
        assert_eq!(normalize_activity_provenance(None), ActivityProvenance::Unknown);
        assert_eq!(normalize_activity_provenance(Some("")), ActivityProvenance::Unknown);
        assert_eq!(
            normalize_activity_provenance(Some("agent.compression")),
            ActivityProvenance::AgentCompression
        );
        assert_eq!(normalize_activity_provenance(Some("bogus")), ActivityProvenance::Unknown);
    }

    #[test]
    fn snapshot_shape() {
        let snap = build_activity_snapshot(Some(1_700_000_000.0), Some("compressing"), None, Some(1_700_000_010.0));
        let o = snap.as_object().unwrap();
        assert_eq!(o["last_activity_at"], serde_json::json!(1_700_000_000.0));
        assert_eq!(o["last_activity_description"], serde_json::json!("compressing"));
        assert_eq!(o["last_activity_provenance"], serde_json::json!("unknown"));
        assert_eq!(o["seconds_since_activity"], serde_json::json!(10.0));
        assert_eq!(o["description"], serde_json::json!("compressing"));
        assert_eq!(o["provenance"], serde_json::json!("unknown"));
    }
}
