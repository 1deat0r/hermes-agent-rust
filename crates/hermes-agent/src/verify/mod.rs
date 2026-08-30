//! Project verification subsystem — run-recipe detection and the
//! environment manifest.
//!
//! PARITY: `agent/verify/` @ b9aa928 — `recipes.py`, `environment.py`, and
//! `runner.py` are ported; only the `__init__` re-export surface and the
//! hermes_cli.verify_cmd consumer remain tied to the CLI crate.
//!
//! Ported from superagent-ai/grok-cli's verify subsystem (scoped).

pub mod environment;
pub mod recipes;
pub mod runner;
