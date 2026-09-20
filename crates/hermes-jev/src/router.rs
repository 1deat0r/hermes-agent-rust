//! Multi-agent router: Choice over handler ids.
//!
//! The user's first Jev use case: a master agent routes a prompt to one
//! specialized agent via a single Choice question instead of a slow LLM
//! triage turn. Mirrors the live aux fast/full routing shape
//! (`agent/auxiliary_client.py::_ask_jev_route`): capped scrubbed state,
//! explicit option ids, validated answer, `None` on any failure so the
//! caller keeps legacy behaviour.

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::questions::{validate_choice_answer, ChoiceQuestion, Question};
use crate::transport::{post_system_one, JEV_MODEL};

/// Question id for the routing decision.
pub const ROUTE_QUESTION_ID: &str = "route";
/// Max prompt chars sent as routing state (sketch, never a transcript).
pub const MAX_PROMPT_CHARS: usize = 2000;

/// One routable handler: id + rubric description.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteOption {
    pub id: String,
    pub description: Option<String>,
}

/// Routing verdict: winning handler id + its probability.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteDecision {
    pub handler: String,
    pub probability: f64,
    pub confidence: f64,
}

/// Build the routing question over `options` (2..=255 per the API).
pub fn route_question(_prompt_summary: &str, options: &[RouteOption]) -> Option<Question> {
    if options.len() < 2 || options.len() > crate::transport::MAX_CHOICE_OPTIONS {
        return None;
    }
    let mut criteria = BTreeMap::new();
    for option in options {
        criteria.insert(option.id.clone(), option.description.clone());
    }
    Some(Question::Choice(ChoiceQuestion {
        instructions: Value::String(
            "Which specialized handler should take this request? Pick exactly one.".to_string(),
        ),
        criteria,
    }))
}

/// Route `prompt` to one handler. Returns `None` when disabled-adjacent
/// conditions hold (fewer than 2 options, keyless, transport error,
/// invalid answer): the caller keeps its legacy route. Never throws.
pub fn route_request(prompt: &str, options: &[RouteOption]) -> Option<RouteDecision> {
    route_request_with_model(prompt, options, JEV_MODEL)
}

/// Same as [`route_request`] with an explicit model id (tests pin one).
pub fn route_request_with_model(
    prompt: &str,
    options: &[RouteOption],
    model: &str,
) -> Option<RouteDecision> {
    let question = route_question(prompt, options)?;
    let mut questions = BTreeMap::new();
    questions.insert(ROUTE_QUESTION_ID.to_string(), question);
    let state = json!({
        "prompt": crate::guardrail::scrub_text(prompt).chars().take(MAX_PROMPT_CHARS).collect::<String>(),
    });
    let answers = post_system_one(&state, &questions, model).ok()?;
    let ids: Vec<&str> = options.iter().map(|o| o.id.as_str()).collect();
    let parsed = validate_choice_answer(&ids, answers.get(ROUTE_QUESTION_ID)?)?;
    Some(RouteDecision {
        handler: parsed.choice.clone(),
        probability: parsed.probabilities.get(&parsed.choice).copied()?,
        confidence: parsed.confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Vec<RouteOption> {
        vec![
            RouteOption {
                id: "research_agent".to_string(),
                description: Some("web research and fact finding".to_string()),
            },
            RouteOption {
                id: "coding_agent".to_string(),
                description: Some("code changes and debugging".to_string()),
            },
        ]
    }

    #[test]
    fn single_option_is_not_routable() {
        let one = &options()[..1];
        assert!(route_question("hi", one).is_none());
        // Keyless or not, a degenerate roster never routes.
        assert!(route_request("hi", one).is_none());
    }
}
