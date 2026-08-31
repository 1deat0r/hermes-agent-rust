//! Project verification subsystem.
//!
//! PARITY: `agent/verify/__init__.py` @ b9aa928 (whole module — the
//! re-export surface). Ported from superagent-ai/grok-cli's verify
//! subsystem (scoped): static run-recipe detection, a persisted
//! environment manifest, and a smoke-test runner used by the
//! `hermes verify` CLI command.
//!
//! Sources:
//! - https://github.com/superagent-ai/grok-cli/blob/main/src/verify/recipes.ts
//! - https://github.com/superagent-ai/grok-cli/blob/main/src/verify/environment.ts

pub mod environment;
pub mod recipes;
pub mod runner;

pub use environment::{load_manifest, load_or_detect, manifest_path, save_manifest};
pub use recipes::{detect_package_manager, detect_recipe, Recipe};
pub use runner::{run_verify, PhaseResult, ReadinessResult, VerifyResult};
