//! Memory-nudge gate: one Noul over the user turn.
//!
//! Ports live `agent/turn_context.py` lines 659-841: when a memory-nudge
//! interval trips, Jev votes P(turn worth consolidating into persistent
//! memory). Default OFF, threshold 0.5, fails OPEN (True = blind
//! behaviour). State is scrubbed and capped — never raw transcripts.

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::guardrail::scrub_text;
use crate::questions::{validate_noul_answer, NoulQuestion, Question};
use crate::transport::{post_system_one, JEV_MODEL};

/// Question id for the nudge decision.
pub const NUDGE_QUESTION_ID: &str = "nudge";
/// Default threshold (live `_JEV_NUDGE_DEFAULT_THRESHOLD`).
pub const DEFAULT_THRESHOLD: f64 = 0.5;
/// Max user-message chars sent as state.
pub const MAX_MESSAGE_CHARS: usize = 500;

/// Nudge instructions (live `_JEV_NUDGE_INSTRUCTIONS`).
pub const NUDGE_INSTRUCTIONS: &str = "This turn's user message adds anything worth consolidating into persistent memory (a durable fact, preference, or decision the assistant should remember across sessions)";

/// Clamp a threshold knob to [0, 1]; anything malformed -> default.
pub fn clamp_threshold(value: Option<f64>) -> f64 {
    match value {
        Some(v) if v.is_finite() => v.clamp(0.0, 1.0),
        _ => DEFAULT_THRESHOLD,
    }
}

/// Ask Jev one `noul` nudge question; `None` on keyless/failure.
pub fn ask_nudge_score(user_message: &str, turns_since_nudge: i64) -> Option<f64> {
    ask_nudge_score_with_model(user_message, turns_since_nudge, JEV_MODEL)
}

/// Same as [`ask_nudge_score`] with an explicit model id.
pub fn ask_nudge_score_with_model(
    user_message: &str,
    turns_since_nudge: i64,
    model: &str,
) -> Option<f64> {
    let question = Question::Noul(NoulQuestion {
        instructions: Value::String(NUDGE_INSTRUCTIONS.to_string()),
        criteria_true: None,
        criteria_false: None,
    });
    let mut questions = BTreeMap::new();
    questions.insert(NUDGE_QUESTION_ID.to_string(), question);
    let state = json!({
        "user_message": scrub_text(user_message).chars().take(MAX_MESSAGE_CHARS).collect::<String>(),
        "turns_since_nudge": turns_since_nudge,
    });
    let answers = post_system_one(&state, &questions, model).ok()?;
    Some(validate_noul_answer(answers.get(NUDGE_QUESTION_ID)?)?.noul)
}

/// Nudge verdict. Never throws; fails OPEN (True) when disabled or Jev
/// is unreachable — a missed consolidation beats a lost turn.
pub fn gate_memory_nudge(
    enabled: bool,
    threshold: f64,
    user_message: &str,
    turns_since_nudge: i64,
) -> bool {
    if !enabled {
        return true;
    }
    match ask_nudge_score(user_message, turns_since_nudge) {
        Some(score) => score >= clamp_threshold(Some(threshold)),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_gate_fires_blind() {
        assert!(gate_memory_nudge(false, 0.99, "anything", 10));
    }

    #[test]
    fn threshold_clamps() {
        assert_eq!(clamp_threshold(Some(2.0)), 1.0);
        assert_eq!(clamp_threshold(Some(-1.0)), 0.0);
        assert_eq!(clamp_threshold(Some(f64::NAN)), DEFAULT_THRESHOLD);
        assert_eq!(clamp_threshold(None), DEFAULT_THRESHOLD);
    }
}
