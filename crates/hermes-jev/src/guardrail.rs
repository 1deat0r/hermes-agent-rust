//! Confidence-gated output guardrail + secret scrubber.
//!
//! The user's fifth Jev use case: parallel Noul checks screen an output
//! before it ships. High-confidence safe ships; low-confidence or risky
//! escalates to an LLM or human. Ports the scrubber from live
//! `agent/compression_scored_prune.py::scrub` (key-shaped secrets are
//! redacted before Jev ever sees them) plus the fail-OPEN verdict shape
//! of the live gates.

use regex::Regex;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::questions::{validate_noul_answer, NoulQuestion, Question};
use crate::transport::{post_system_one, JEV_MODEL};

/// Question ids for the parallel safety screen.
pub const HALLUCINATION_QUESTION_ID: &str = "has_hallucination";
pub const POLICY_QUESTION_ID: &str = "violates_policy";
/// Max output chars sent per guardrail request.
pub const MAX_OUTPUT_CHARS: usize = 6000;

/// Guardrail verdict: P(hallucination) and P(policy violation).
#[derive(Debug, Clone, PartialEq)]
pub struct GuardrailVerdict {
    pub hallucination_probability: f64,
    pub policy_violation_probability: f64,
}

impl GuardrailVerdict {
    /// Safe to ship: both risks below `risk_threshold`.
    pub fn is_safe(&self, risk_threshold: f64) -> bool {
        self.hallucination_probability < risk_threshold
            && self.policy_violation_probability < risk_threshold
    }
}

/// Screen `output` against `evidence` with two parallel Nouls.
/// `None` on any failure (keyless, transport, invalid): the caller
/// escalates to heavy review. Never throws.
pub fn screen_output(output: &str, evidence: &str) -> Option<GuardrailVerdict> {
    screen_output_with_model(output, evidence, JEV_MODEL)
}

/// Same as [`screen_output`] with an explicit model id.
pub fn screen_output_with_model(
    output: &str,
    evidence: &str,
    model: &str,
) -> Option<GuardrailVerdict> {
    let mut questions = BTreeMap::new();
    questions.insert(
        HALLUCINATION_QUESTION_ID.to_string(),
        Question::Noul(NoulQuestion {
            instructions: Value::String(
                "Does the output state anything not supported by `evidence`?".to_string(),
            ),
            criteria_true: Some("claims with no basis in the evidence".to_string()),
            criteria_false: Some("every claim is grounded in the evidence".to_string()),
        }),
    );
    questions.insert(
        POLICY_QUESTION_ID.to_string(),
        Question::Noul(NoulQuestion {
            instructions: Value::String(
                "Does the output break policy: secrets, destructive commands, or disallowed content?"
                    .to_string(),
            ),
            criteria_true: Some("contains a policy violation".to_string()),
            criteria_false: Some("clean of policy violations".to_string()),
        }),
    );
    let state = json!({
        "output": scrub_text(output).chars().take(MAX_OUTPUT_CHARS).collect::<String>(),
        "evidence": scrub_text(evidence).chars().take(MAX_OUTPUT_CHARS).collect::<String>(),
    });
    let answers = post_system_one(&state, &questions, model).ok()?;
    let hallucination = validate_noul_answer(answers.get(HALLUCINATION_QUESTION_ID)?)?.noul;
    let policy = validate_noul_answer(answers.get(POLICY_QUESTION_ID)?)?.noul;
    Some(GuardrailVerdict {
        hallucination_probability: hallucination,
        policy_violation_probability: policy,
    })
}

/// Key-shaped secret patterns, ported from live
/// `agent/compression_scored_prune.py::SECRET_PATTERNS`.
pub const SECRET_PATTERNS: &[&str] = &[
    r"apikey_[A-Za-z0-9_]+",
    r"sk-[A-Za-z0-9]{8,}",
    r"xox[bpas]-[A-Za-z0-9-]+",
    r"ghp_[A-Za-z0-9]+",
    r"gsk_[A-Za-z0-9]+",
    r"AKIA[A-Z0-9]{16}",
    r"[Bb]earer [A-Za-z0-9._~+\-/=]+",
];

static SECRET_RES: OnceLock<Vec<Regex>> = OnceLock::new();

fn secret_res() -> &'static Vec<Regex> {
    SECRET_RES.get_or_init(|| {
        SECRET_PATTERNS
            .iter()
            .map(|pat| Regex::new(pat).expect("secret pattern compiles"))
            .collect()
    })
}

/// Redact key-shaped secrets so Jev never sees pasted credentials.
pub fn scrub_text(text: &str) -> String {
    let mut out = text.to_string();
    for rx in secret_res() {
        out = rx.replace_all(&out, "[redacted-secret]").into_owned();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_verdict_passes_threshold() {
        let verdict = GuardrailVerdict {
            hallucination_probability: 0.05,
            policy_violation_probability: 0.01,
        };
        assert!(verdict.is_safe(0.2));
    }

    #[test]
    fn risky_verdict_fails_threshold() {
        let verdict = GuardrailVerdict {
            hallucination_probability: 0.6,
            policy_violation_probability: 0.01,
        };
        assert!(!verdict.is_safe(0.2));
    }
}
