//! Typed System One question/answer shapes.
//!
//! Ports the request/response contract in TypeSafe docs `api.md` plus the
//! validation rules in live `agent/jev_choice.py::validate_choice` (choice
//! in ids, probability keys exactly the ids, finite numbers in [0, 1],
//! sum ~1 within 0.02, top probability is the choice).

use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

/// Tolerance for the probability-sum check (live `validate_choice`).
pub const PROB_SUM_TOLERANCE: f64 = 0.02;

/// One typed question inside a `questions` map.
#[derive(Debug, Clone, PartialEq)]
pub enum Question {
    Choice(ChoiceQuestion),
    Noul(NoulQuestion),
    Score(ScoreQuestion),
}

/// Choice: pick one option from a defined set (max 255 per the API).
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceQuestion {
    pub instructions: Value,
    /// option id -> rubric description (`None` = no extra detail).
    pub criteria: BTreeMap<String, Option<String>>,
}

/// Noul: yes/no question; answer is P(yes) in [0, 1] (no confidence).
#[derive(Debug, Clone, PartialEq)]
pub struct NoulQuestion {
    pub instructions: Value,
    pub criteria_true: Option<String>,
    pub criteria_false: Option<String>,
}

/// Score: position on ordered levels (2..=10 per the API).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreQuestion {
    pub instructions: Value,
    pub criteria: Vec<String>,
}

/// Validated choice answer.
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceAnswer {
    pub choice: String,
    pub probabilities: BTreeMap<String, f64>,
    pub confidence: f64,
}

/// Validated noul answer: P(yes).
#[derive(Debug, Clone, PartialEq)]
pub struct NoulAnswer {
    pub noul: f64,
}

/// Validated score answer.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreAnswer {
    pub score: f64,
    pub legend: BTreeMap<String, String>,
    pub probabilities: BTreeMap<String, f64>,
    pub confidence: f64,
}

impl Question {
    /// Serialize one question to its wire shape.
    pub fn to_json(&self) -> Value {
        match self {
            Question::Choice(q) => {
                let criteria: Map<String, Value> = q
                    .criteria
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            v.clone().map(Value::String).unwrap_or(Value::Null),
                        )
                    })
                    .collect();
                json!({
                    "type": "choice",
                    "instructions": q.instructions,
                    "criteria": criteria,
                })
            }
            Question::Noul(q) => {
                let mut obj = Map::new();
                obj.insert("type".to_string(), Value::String("noul".to_string()));
                obj.insert("instructions".to_string(), q.instructions.clone());
                if q.criteria_true.is_some() || q.criteria_false.is_some() {
                    let mut criteria = Map::new();
                    if let Some(t) = &q.criteria_true {
                        criteria.insert("true".to_string(), Value::String(t.clone()));
                    }
                    if let Some(f) = &q.criteria_false {
                        criteria.insert("false".to_string(), Value::String(f.clone()));
                    }
                    obj.insert("criteria".to_string(), Value::Object(criteria));
                }
                Value::Object(obj)
            }
            Question::Score(q) => {
                json!({
                    "type": "score",
                    "instructions": q.instructions,
                    "criteria": q.criteria,
                })
            }
        }
    }
}

fn number_in_unit(value: &Value) -> Option<f64> {
    let n = value.as_f64()?;
    if n.is_finite() && (0.0..=1.0).contains(&n) {
        Some(n)
    } else {
        None
    }
}

fn read_probabilities(value: Option<&Value>) -> Option<BTreeMap<String, f64>> {
    let map = value?.as_object()?;
    let mut out = BTreeMap::new();
    for (k, v) in map {
        out.insert(k.clone(), number_in_unit(v)?);
    }
    Some(out)
}

/// Validate a choice answer exactly like live `validate_choice`.
///
/// `ids` are the offered criteria ids. Returns the parsed answer on
/// success, `None` on any violation.
pub fn validate_choice_answer(ids: &[&str], answer: &Value) -> Option<ChoiceAnswer> {
    let obj = answer.as_object()?;
    let choice = obj.get("choice")?.as_str()?;
    if !ids.contains(&choice) {
        return None;
    }
    let probabilities = read_probabilities(obj.get("probabilities"))?;
    let want: BTreeMap<String, f64> = ids.iter().map(|s| (s.to_string(), 0.0)).collect();
    if probabilities.keys().collect::<Vec<_>>() != want.keys().collect::<Vec<_>>() {
        return None;
    }
    let confidence = number_in_unit(obj.get("confidence")?)?;
    let sum: f64 = probabilities.values().sum();
    if (sum - 1.0).abs() >= PROB_SUM_TOLERANCE {
        return None;
    }
    let top = probabilities.values().fold(0.0_f64, |a, b| a.max(*b));
    // Live allows 1e-6 slack so float ties still validate.
    if probabilities.get(choice).copied().unwrap_or(f64::NAN) < top - 1e-6 {
        return None;
    }
    Some(ChoiceAnswer {
        choice: choice.to_string(),
        probabilities,
        confidence,
    })
}

/// Validate a noul answer: finite `noul` in [0, 1].
pub fn validate_noul_answer(answer: &Value) -> Option<NoulAnswer> {
    let noul = number_in_unit(answer.as_object()?.get("noul")?)?;
    Some(NoulAnswer { noul })
}

/// Validate a score answer: finite `score`, legend, per-level
/// probabilities summing to ~1, confidence in [0, 1].
pub fn validate_score_answer(answer: &Value) -> Option<ScoreAnswer> {
    let obj = answer.as_object()?;
    let score = obj.get("score")?.as_f64()?;
    if !score.is_finite() || score < 0.0 {
        return None;
    }
    let legend_obj = obj.get("legend")?.as_object()?;
    let mut legend = BTreeMap::new();
    for (k, v) in legend_obj {
        legend.insert(k.clone(), v.as_str()?.to_string());
    }
    if legend.len() < 2 {
        return None;
    }
    let probabilities = read_probabilities(obj.get("probabilities"))?;
    if probabilities.keys().collect::<Vec<_>>() != legend.keys().collect::<Vec<_>>() {
        return None;
    }
    let sum: f64 = probabilities.values().sum();
    if (sum - 1.0).abs() >= PROB_SUM_TOLERANCE {
        return None;
    }
    let confidence = number_in_unit(obj.get("confidence")?)?;
    Some(ScoreAnswer {
        score,
        legend,
        probabilities,
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice_ids() -> Vec<&'static str> {
        vec!["FAST", "FULL"]
    }

    #[test]
    fn valid_choice_passes() {
        let answer = json!({
            "type": "choice",
            "choice": "FAST",
            "probabilities": {"FAST": 0.8, "FULL": 0.2},
            "confidence": 0.6,
        });
        let parsed = validate_choice_answer(&choice_ids(), &answer).expect("valid");
        assert_eq!(parsed.choice, "FAST");
    }

    #[test]
    fn choice_outside_ids_fails() {
        let answer = json!({
            "choice": "CHEAP",
            "probabilities": {"CHEAP": 1.0},
            "confidence": 1.0,
        });
        assert!(validate_choice_answer(&choice_ids(), &answer).is_none());
    }
}
