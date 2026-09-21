//! Agent-layer parity surfaces from `agent/`.
//!
//! PARITY: `agent/__init__.py` @ 5d59366 — "Agent internals extracted from
//! run_agent.py so it stays focused on AIAgent." The `jiter_preload` eager
//! import arm (`from . import jiter_preload as _jiter_preload`, private
//! alias) is [`jiter_preload`]; visibility divergence: `pub mod` (Rust
//! integration tests are external crates — `pub(crate)` would break them).
//! Rust has no import-time side effects, so the bottom-of-module eager call
//! is PENDING with the future binary entry point, which must call
//! [`jiter_preload::preload_jiter_native_extension`] explicitly at startup.

pub mod auxiliary_client;
pub mod billing_links;
pub mod bounded_response;
pub mod config;
pub mod credential_pool;
pub mod credential_store;
pub mod errors;
pub mod interrupt_compat;
pub mod iteration_budget;
pub mod jev_review;
pub mod jev_router;
pub mod jiter_preload;
pub mod kanban_stop;
pub mod lmstudio_reasoning;
pub mod managed_scope;
pub mod manual_compression_feedback;
pub mod markdown_tables;
pub mod message_content;
pub mod monitoring;
pub mod portal_tags;
pub mod proxy_sources;
pub mod reactions;
pub mod reasoning_summaries;
pub mod run_agent;
pub mod secret_sources;
pub mod ssl_guard;
pub mod think_scrubber;
pub mod tool_result_classification;
pub mod trajectory;
pub mod turn_retry_state;
pub mod verify;
pub mod verify_hooks;
