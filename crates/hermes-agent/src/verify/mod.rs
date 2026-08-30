//! Project verification subsystem — run-recipe detection and the
//! environment manifest.
//!
//! PARITY: `agent/verify/` @ b9aa928 — PARTIAL: `recipes.py` and
//! `environment.py` are ported; `runner.py` (the smoke-test runner) and
//! the `__init__` re-exports that surface it stay PENDING.
//!
//! Ported from superagent-ai/grok-cli's verify subsystem (scoped).

pub mod environment;
pub mod recipes;
