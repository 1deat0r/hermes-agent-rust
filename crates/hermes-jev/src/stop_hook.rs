//! Loop-termination stop hook: Noul over the objective.
//!
//! The user's third Jev use case (Limpet-style stop hook): at the end of
//! every loop iteration Jev answers P(objective satisfied). `true` with
//! high confidence exits safely; anything else keeps looping. Ports the
//! fail-OPEN shape of the live `jev_review_gate` verdicts: keyless,
//! transport, or invalid answers return `None` (the caller keeps its
//! legacy iteration policy) and never throw.

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::questions::{validate_noul_answer, NoulQuestion, Question};
use crate::transport::{post_system_one, JEV_MODEL};

/// Question id for the stop decision.
pub const STOP_QUESTION_ID: &str = "objective_satisfied";
/// Max objective/output chars sent as state.
pub const MAX_OBJECTIVE_CHARS: usize = 1000;
pub const MAX_OUTPUT_CHARS: usize = 4000;

/// Stop-hook verdict: P(objective satisfied) in [0, 1].
#[derive(Debug, Clone, PartialEq)]
pub struct StopVerdict {
    pub satisfied_probability: f64,
}

/// Ask P(objective satisfied) for one loop iteration. `None` on any
/// failure: the caller keeps looping under its own budget. Never throws.
pub fn check_objective_satisfied(objective: &str, latest_output: &str) -> Option<StopVerdict> {
    check_objective_satisfied_with_model(objective, latest_output, JEV_MODEL)
}

/// Same as [`check_objective_satisfied`] with an explicit model id.
pub fn check_objective_satisfied_with_model(
    objective: &str,
    latest_output: &str,
    model: &str,
) -> Option<StopVerdict> {
    let question = Question::Noul(NoulQuestion {
        instructions: Value::String(
            "Does the current state completely satisfy the original objective?".to_string(),
        ),
        criteria_true: Some("every requirement in the objective is met".to_string()),
        criteria_false: Some("anything required is still missing or wrong".to_string()),
    });
    let mut questions = BTreeMap::new();
    questions.insert(STOP_QUESTION_ID.to_string(), question);
    let state = json!({
        "objective": crate::guardrail::scrub_text(objective).chars().take(MAX_OBJECTIVE_CHARS).collect::<String>(),
        "latest_output": crate::guardrail::scrub_text(latest_output).chars().take(MAX_OUTPUT_CHARS).collect::<String>(),
    });
    let answers = post_system_one(&state, &questions, model).ok()?;
    let parsed = validate_noul_answer(answers.get(STOP_QUESTION_ID)?)?;
    Some(StopVerdict {
        satisfied_probability: parsed.noul,
    })
}

/// Threshold a verdict into stop/continue.
///
/// `stop_threshold` near 1 stops only on near-certain completion;
/// `continue_threshold` near 0 keeps looping on near-certain
/// incompleteness; the middle returns `None` (undecided — the caller
/// applies its own budget policy).
pub fn decide_stop(
    verdict: &StopVerdict,
    stop_threshold: f64,
    continue_threshold: f64,
) -> Option<bool> {
    if verdict.satisfied_probability >= stop_threshold {
        Some(true)
    } else if verdict.satisfied_probability <= continue_threshold {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_split_stop_continue_undecided() {
        assert_eq!(
            decide_stop(
                &StopVerdict {
                    satisfied_probability: 0.95
                },
                0.9,
                0.2
            ),
            Some(true)
        );
        assert_eq!(
            decide_stop(
                &StopVerdict {
                    satisfied_probability: 0.1
                },
                0.9,
                0.2
            ),
            Some(false)
        );
        assert_eq!(
            decide_stop(
                &StopVerdict {
                    satisfied_probability: 0.5
                },
                0.9,
                0.2
            ),
            None
        );
    }
}
