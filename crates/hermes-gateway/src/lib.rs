//! hermes-gateway — 1:1 Rust port of gateway/ (@ b9aa928).
//!
//! Phase 4 crate, opened early as a dependency-free leaf exactly like
//! `hermes-cli`: the first module (`cwd_placeholder`) is pure stdlib logic
//! upstream, so it lands without dragging the messaging-platform transport
//! layers in. Everything here must stay below the agent/tools layers the
//! gateway eventually imports.

pub mod cwd_placeholder;
