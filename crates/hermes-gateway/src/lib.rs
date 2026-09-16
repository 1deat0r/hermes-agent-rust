//! hermes-gateway — 1:1 Rust port of gateway/ (@ 5d59366).
//!
//! Phase 4 crate, opened early as a dependency-free leaf exactly like
//! `hermes-cli`: the first module (`cwd_placeholder`) is pure stdlib logic
//! upstream, so it lands without dragging the messaging-platform transport
//! layers in. Everything here must stay below the agent/tools layers the
//! gateway eventually imports.

pub mod builtin_hooks;
pub mod cgroup_cleanup;
pub mod code_skew;
pub mod cwd_placeholder;
pub mod platform_http_limits;
pub mod qqbot;
pub mod readiness;
pub mod rich_sent_store;
pub mod session_stall;
