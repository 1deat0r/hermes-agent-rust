//! High-speed tool selection: Choice over tool names.
//!
//! The user's second Jev use case: inside a long-horizon loop the agent
//! asks "what next?" by passing its execution sketch to Jev and getting
//! back exactly one tool id — no generated syntax, no JSON parse errors.
//! A `none_of_these` outcome is included per the live skill guidance so
//! the loop can stop or escalate instead of forcing a wrong tool.

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::questions::{validate_choice_answer, ChoiceQuestion, Question};
use crate::transport::{post_system_one, JEV_MODEL};

/// Question id for the tool-selection decision.
pub const TOOL_QUESTION_ID: &str = "next_tool";
/// Outcome when no offered tool fits the current state.
pub const NO_TOOL_OPTION: &str = "none_of_these";
/// Max execution-log chars sent as state.
pub const MAX_LOG_CHARS: usize = 4000;

/// Tool-selection verdict: winning tool id + its probability.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDecision {
    pub tool: String,
    pub probability: f64,
    pub confidence: f64,
}

/// Build the tool-selection question (tools + `none_of_these`).
pub fn tool_question(_goal_summary: &str, tools: &BTreeMap<String, String>) -> Option<Question> {
    if tools.is_empty() || tools.len() + 1 > crate::transport::MAX_CHOICE_OPTIONS {
        return None;
    }
    let mut criteria: BTreeMap<String, Option<String>> = tools
        .iter()
        .map(|(name, description)| (name.clone(), Some(description.clone())))
        .collect();
    criteria.insert(
        NO_TOOL_OPTION.to_string(),
        Some("none of the offered tools moves the goal forward".to_string()),
    );
    Some(Question::Choice(ChoiceQuestion {
        instructions: Value::String(
            "Which single tool should run next to advance the goal? Pick exactly one.".to_string(),
        ),
        criteria,
    }))
}

/// Select the next tool from the execution sketch. `None` on any
/// failure (keyless, transport, invalid answer): the caller keeps its
/// legacy policy. Never throws.
pub fn select_next_tool(
    goal: &str,
    execution_log: &str,
    tools: &BTreeMap<String, String>,
) -> Option<ToolDecision> {
    select_next_tool_with_model(goal, execution_log, tools, JEV_MODEL)
}

/// Same as [`select_next_tool`] with an explicit model id.
pub fn select_next_tool_with_model(
    goal: &str,
    execution_log: &str,
    tools: &BTreeMap<String, String>,
    model: &str,
) -> Option<ToolDecision> {
    let question = tool_question(goal, tools)?;
    let mut questions = BTreeMap::new();
    questions.insert(TOOL_QUESTION_ID.to_string(), question);
    let state = json!({
        "goal": crate::guardrail::scrub_text(goal).chars().take(500).collect::<String>(),
        "execution_log": crate::guardrail::scrub_text(execution_log).chars().take(MAX_LOG_CHARS).collect::<String>(),
    });
    let answers = post_system_one(&state, &questions, model).ok()?;
    let mut ids: Vec<&str> = tools.keys().map(String::as_str).collect();
    ids.push(NO_TOOL_OPTION);
    let parsed = validate_choice_answer(&ids, answers.get(TOOL_QUESTION_ID)?)?;
    Some(ToolDecision {
        tool: parsed.choice.clone(),
        probability: parsed.probabilities.get(&parsed.choice).copied()?,
        confidence: parsed.confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("web_search".to_string(), "search the web".to_string()),
            ("read_file".to_string(), "read a file".to_string()),
        ])
    }

    #[test]
    fn question_always_offers_no_tool_outcome() {
        let question = tool_question("goal", &tools()).expect("question");
        match question {
            Question::Choice(q) => {
                assert!(q.criteria.contains_key(NO_TOOL_OPTION));
                assert_eq!(q.criteria.len(), 3);
            }
            _ => panic!("must be a choice"),
        }
    }

    #[test]
    fn empty_roster_never_selects() {
        assert!(select_next_tool("goal", "log", &BTreeMap::new()).is_none());
    }
}
