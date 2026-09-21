//! Triage labeling: one Choice + parallel Nouls per item.
//!
//! The last Jev-shaped gap: every queue in the harness (issue triage,
//! message routing, approval queues) is really "pick exactly one label,
//! flag every risk". One batched request per item — a Choice over the
//! label set plus one Noul per independent risk flag — keeps it a
//! single round trip with code-owned thresholds.

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::guardrail::scrub_text;
use crate::questions::{
    validate_choice_answer, validate_noul_answer, ChoiceQuestion, NoulQuestion, Question,
};
use crate::transport::{post_system_one, JEV_MODEL};

/// Question ids for the triage decision.
pub const LABEL_QUESTION_ID: &str = "label";
/// Max item text chars sent as state.
pub const MAX_ITEM_CHARS: usize = 3000;

/// One triage label: id + rubric description.
#[derive(Debug, Clone, PartialEq)]
pub struct TriageLabel {
    pub id: String,
    pub description: Option<String>,
}

/// One risk flag: question id + yes/no question.
#[derive(Debug, Clone, PartialEq)]
pub struct RiskFlag {
    pub id: String,
    pub instructions: String,
}

/// Triage verdict: winning label + per-flag probabilities.
#[derive(Debug, Clone, PartialEq)]
pub struct TriageVerdict {
    pub label: String,
    pub label_probability: f64,
    pub label_confidence: f64,
    pub flags: BTreeMap<String, f64>,
}

impl TriageVerdict {
    /// Every raised flag (P(flag) >= `flag_threshold`).
    pub fn raised_flags(&self, flag_threshold: f64) -> Vec<&str> {
        self.flags
            .iter()
            .filter(|(_, p)| **p >= flag_threshold)
            .map(|(id, _)| id.as_str())
            .collect()
    }
}

/// Build the triage question set (Choice + one Noul per flag).
pub fn triage_questions(
    labels: &[TriageLabel],
    flags: &[RiskFlag],
) -> Option<BTreeMap<String, Question>> {
    if labels.len() < 2 || labels.len() > crate::transport::MAX_CHOICE_OPTIONS {
        return None;
    }
    let mut questions = BTreeMap::new();
    questions.insert(
        LABEL_QUESTION_ID.to_string(),
        Question::Choice(ChoiceQuestion {
            instructions: Value::String(
                "Which single label best categorizes this item? Pick exactly one.".to_string(),
            ),
            criteria: labels
                .iter()
                .map(|l| (l.id.clone(), l.description.clone()))
                .collect(),
        }),
    );
    for flag in flags {
        questions.insert(
            flag.id.clone(),
            Question::Noul(NoulQuestion {
                instructions: Value::String(flag.instructions.clone()),
                criteria_true: None,
                criteria_false: None,
            }),
        );
    }
    Some(questions)
}

/// Triage one item: label + risk flags in a single request. `None` on
/// any failure (keyless, transport, invalid): the caller keeps its
/// legacy queue behaviour. Never throws.
pub fn triage_item(
    item: &str,
    labels: &[TriageLabel],
    flags: &[RiskFlag],
) -> Option<TriageVerdict> {
    triage_item_with_model(item, labels, flags, JEV_MODEL)
}

/// Same as [`triage_item`] with an explicit model id.
pub fn triage_item_with_model(
    item: &str,
    labels: &[TriageLabel],
    flags: &[RiskFlag],
    model: &str,
) -> Option<TriageVerdict> {
    let questions = triage_questions(labels, flags)?;
    let state = json!({
        "item": scrub_text(item).chars().take(MAX_ITEM_CHARS).collect::<String>(),
    });
    let answers = post_system_one(&state, &questions, model).ok()?;
    let ids: Vec<&str> = labels.iter().map(|l| l.id.as_str()).collect();
    let parsed = validate_choice_answer(&ids, answers.get(LABEL_QUESTION_ID)?)?;
    let mut flag_probs = BTreeMap::new();
    for flag in flags {
        flag_probs.insert(
            flag.id.clone(),
            validate_noul_answer(answers.get(&flag.id)?)?.noul,
        );
    }
    Some(TriageVerdict {
        label: parsed.choice.clone(),
        label_probability: parsed.probabilities.get(&parsed.choice).copied()?,
        label_confidence: parsed.confidence,
        flags: flag_probs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels() -> Vec<TriageLabel> {
        vec![
            TriageLabel {
                id: "bug".to_string(),
                description: Some("broken behaviour".to_string()),
            },
            TriageLabel {
                id: "feature".to_string(),
                description: Some("new capability".to_string()),
            },
        ]
    }

    #[test]
    fn single_label_is_not_triaged() {
        let one = &labels()[..1];
        assert!(triage_questions(one, &[]).is_none());
        assert!(triage_item("x", one, &[]).is_none());
    }

    #[test]
    fn raised_flags_threshold() {
        let verdict = TriageVerdict {
            label: "bug".to_string(),
            label_probability: 0.9,
            label_confidence: 0.8,
            flags: BTreeMap::from([("urgent".to_string(), 0.95), ("security".to_string(), 0.1)]),
        };
        assert_eq!(verdict.raised_flags(0.8), vec!["urgent"]);
    }
}
