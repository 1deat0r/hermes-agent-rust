//! Post-turn background review gates.
//!
//! Ports live `agent/jev_review_gate.py` (whole module): two narrow
//! Choice (REVIEW/SKIP) gates, both default OFF with a fail-OPEN
//! fallback to the blind behaviour — a missed review is worse than a
//! wasted fork.
//!
//! 1. Spawn gate: when a memory/skill nudge interval trips, Jev votes on
//!    whether the turn plausibly contains anything worth saving. Below
//!    the confidence floor the ~30K-token review fork never spawns.
//! 2. Skill-scope gate: Jev votes on whether the turn produced anything
//!    skill-worthy. Below the floor the review still spawns but with
//!    skill review disabled (memory half untouched).
//!
//! State is scrubbed and capped — never raw transcripts. Neither
//! function ever throws.

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::guardrail::scrub_text;
use crate::questions::{validate_choice_answer, ChoiceQuestion, Question};
use crate::transport::{post_system_one, JEV_MODEL};

/// Shared choice ids for both gates.
pub const REVIEW: &str = "REVIEW";
pub const SKIP: &str = "SKIP";

/// Caps so the state stays a sketch, never a transcript.
pub const MAX_TEXT_CHARS: usize = 500;
pub const MAX_TOOLS: usize = 12;

/// Default confidence floor.
pub const DEFAULT_FLOOR: f64 = 0.5;

const SPAWN_QUESTION_ID: &str = "review";
const SKILL_QUESTION_ID: &str = "skill_scope";

const SPAWN_CRITERIA_REVIEW: &str = "the turn plausibly contains something durable worth saving";
const SPAWN_CRITERIA_SKIP: &str =
    "chit-chat, status checks, or trivial/failed turns with nothing to keep";
const SKILL_CRITERIA_REVIEW: &str =
    "the turn produced a reusable procedure, workaround, or pitfall";
const SKILL_CRITERIA_SKIP: &str = "chit-chat, one-off answers, or routine tool use with no lesson";

const SPAWN_INSTRUCTIONS: &str = "This turn just completed. A background review fork would replay the conversation (tens of thousands of tokens) to decide whether to save memories or skill updates. REVIEW means the turn plausibly contains something durable — user facts, preferences, decisions, reusable procedures, surprising tool outcomes. SKIP means chit-chat, status checks, or trivial/failed turns with nothing worth keeping.";
const SKILL_INSTRUCTIONS: &str = "This turn's work is being considered for a skill review (saving a reusable procedure for future sessions). REVIEW means the turn produced something skill-worthy — a non-trivial workflow, a workaround for a tricky error, a pitfall that cost time. SKIP means chit-chat, a one-off answer, or routine tool use with no reusable lesson.";

/// Gate verdict: winning choice + its probability.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewVerdict {
    pub choice: String,
    pub probability: f64,
}

fn ask_review(
    question_id: &str,
    criteria_review: &str,
    criteria_skip: &str,
    instructions: &str,
    state: &Value,
    model: &str,
) -> Option<ReviewVerdict> {
    let question = Question::Choice(ChoiceQuestion {
        instructions: Value::String(instructions.to_string()),
        criteria: BTreeMap::from([
            (REVIEW.to_string(), Some(criteria_review.to_string())),
            (SKIP.to_string(), Some(criteria_skip.to_string())),
        ]),
    });
    let mut questions = BTreeMap::new();
    questions.insert(question_id.to_string(), question);
    let answers = post_system_one(state, &questions, model).ok()?;
    let ids = [REVIEW, SKIP];
    let parsed = validate_choice_answer(&ids, answers.get(question_id)?)?;
    if parsed.choice != REVIEW && parsed.choice != SKIP {
        return None;
    }
    let probability = parsed.probabilities.get(&parsed.choice).copied()?;
    Some(ReviewVerdict {
        choice: parsed.choice,
        probability,
    })
}

/// Build the spawn gate's Jev state from a turn snapshot (read-only).
pub fn spawn_state(
    user_message: &str,
    assistant_response: &str,
    tools_used: &[String],
    review_memory: bool,
    review_skills: bool,
) -> Value {
    json!({
        "user_message": scrub_text(user_message).chars().take(MAX_TEXT_CHARS).collect::<String>(),
        "assistant_response": scrub_text(assistant_response).chars().take(MAX_TEXT_CHARS).collect::<String>(),
        "tools_used": tools_used.iter().take(MAX_TOOLS).cloned().collect::<Vec<_>>(),
        "tool_calls": tools_used.len().min(MAX_TOOLS),
        "review_memory": review_memory,
        "review_skills": review_skills,
    })
}

/// Build the skill-scope gate's Jev state (scrubbed, capped).
pub fn skill_state(final_response: &str, tools_used: &[String], tool_iterations: i64) -> Value {
    json!({
        "assistant_response": scrub_text(final_response).chars().take(MAX_TEXT_CHARS).collect::<String>(),
        "tools_used": tools_used.iter().take(MAX_TOOLS).cloned().collect::<Vec<_>>(),
        "tool_calls": tools_used.len().min(MAX_TOOLS),
        "tool_iterations": tool_iterations,
    })
}

/// Spawn-gate verdict. Never throws; fails OPEN (True = legacy spawn).
pub fn should_spawn_review(enabled: bool, floor: f64, state: &Value) -> bool {
    should_spawn_review_with_model(enabled, floor, state, JEV_MODEL)
}

/// Same as [`should_spawn_review`] with an explicit model id.
pub fn should_spawn_review_with_model(
    enabled: bool,
    floor: f64,
    state: &Value,
    model: &str,
) -> bool {
    if !enabled {
        return true;
    }
    match ask_review(
        SPAWN_QUESTION_ID,
        SPAWN_CRITERIA_REVIEW,
        SPAWN_CRITERIA_SKIP,
        SPAWN_INSTRUCTIONS,
        state,
        model,
    ) {
        Some(verdict) => verdict.choice == REVIEW && verdict.probability >= floor,
        None => true,
    }
}

/// Skill-scope verdict. Never throws; fails OPEN (True = keep flag).
pub fn keep_skill_review(enabled: bool, floor: f64, state: &Value) -> bool {
    keep_skill_review_with_model(enabled, floor, state, JEV_MODEL)
}

/// Same as [`keep_skill_review`] with an explicit model id.
pub fn keep_skill_review_with_model(enabled: bool, floor: f64, state: &Value, model: &str) -> bool {
    if !enabled {
        return true;
    }
    match ask_review(
        SKILL_QUESTION_ID,
        SKILL_CRITERIA_REVIEW,
        SKILL_CRITERIA_SKIP,
        SKILL_INSTRUCTIONS,
        state,
        model,
    ) {
        Some(verdict) => verdict.choice == REVIEW && verdict.probability >= floor,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_gates_fire_blind() {
        let state = spawn_state("hi", "hello", &[], false, false);
        assert!(should_spawn_review(false, 0.99, &state));
        assert!(keep_skill_review(false, 0.99, &state));
    }

    #[test]
    fn state_caps_text_and_tools() {
        let tools: Vec<String> = (0..30).map(|i| format!("tool_{i}")).collect();
        let state = spawn_state(&"x".repeat(2000), "y", &tools, true, true);
        assert!(state["user_message"].as_str().expect("str").len() <= MAX_TEXT_CHARS);
        assert_eq!(
            state["tools_used"].as_array().expect("array").len(),
            MAX_TOOLS
        );
    }
}
