//! Jev review/nudge seam for the agent turn path.
//!
//! Re-exports the [`hermes_jev`] review-gate and memory-nudge surfaces
//! next to the turn loop so callers gate forks and nudges in one place.
//! Config-gated default OFF at the caller; every verdict fails OPEN to
//! the blind (pre-Jev) behaviour and never throws.

pub use hermes_jev::memory_nudge::{
    ask_nudge_score, ask_nudge_score_with_model, clamp_threshold, gate_memory_nudge,
    DEFAULT_THRESHOLD as NUDGE_DEFAULT_THRESHOLD, MAX_MESSAGE_CHARS as NUDGE_MAX_MESSAGE_CHARS,
    NUDGE_INSTRUCTIONS, NUDGE_QUESTION_ID,
};
pub use hermes_jev::review_gate::{
    keep_skill_review, keep_skill_review_with_model, should_spawn_review,
    should_spawn_review_with_model, skill_state, spawn_state, ReviewVerdict,
    DEFAULT_FLOOR as REVIEW_DEFAULT_FLOOR, MAX_TEXT_CHARS as REVIEW_MAX_TEXT_CHARS,
    MAX_TOOLS as REVIEW_MAX_TOOLS, REVIEW, SKIP,
};
