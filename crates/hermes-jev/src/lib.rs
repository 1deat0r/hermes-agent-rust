//! Jev System One decision layer.
//!
//! Live Hermes runs Jev (TypeSafe System One, model `jev-latest`) as the
//! fast decider on agent hot paths: fast/full aux routing
//! (`agent/auxiliary_client.py` ~798-945), the memory-nudge gate
//! (`agent/turn_context.py` ~659-841), the review spawn/skill gates
//! (`agent/jev_review_gate.py`), the shared choice transport
//! (`agent/jev_choice.py`), and scored compaction
//! (`agent/compression_scored_prune.py`). None of these exist at pin
//! `5d59366`, so this crate is an **additive** layer: typed
//! Choice/Noul/Score request/response types plus the four workflow
//! helpers (router, tool selection, stop hook, guardrail), all
//! config-gated default OFF, keyless-safe, and never throwing into a
//! hot path.
//!
//! Transport contract (TypeSafe docs `api.md` @ 2026-09-20):
//! `POST https://api.typesafe.ai/v1/systemone` with
//! `{"model": "jev-latest", "state": ..., "questions": {...}}`,
//! key ONLY from `TYPESAFE_API_KEY` read at call time — never logged,
//! printed, or stored. Retries on 429/529/503, raises on every other
//! failure so callers fail safe to legacy behaviour.
//!
//! Validation ports `validate_choice` from `agent/jev_choice.py` exactly:
//! the choice is one of the offered ids, probabilities cover exactly
//! those ids, every number is finite in [0, 1], probabilities sum to
//! ~1 (tolerance 0.02), and the top probability is the choice.

pub mod guardrail;
pub mod questions;
pub mod router;
pub mod stop_hook;
pub mod tool_route;
pub mod transport;

pub use questions::{
    ChoiceAnswer, ChoiceQuestion, NoulAnswer, NoulQuestion, Question, ScoreAnswer, ScoreQuestion,
};
pub use transport::{
    JevError, API_KEY_ENV, JEV_MODEL, MAX_ATTEMPTS, MAX_CHOICE_OPTIONS, MAX_SCORE_LEVELS,
    REQUEST_TIMEOUT_SECS, RETRY_STATUSES, SYSTEMONE_URL,
};
