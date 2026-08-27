//! Agent-layer parity surfaces from `agent/`.

pub mod auxiliary_client;
pub mod config;
pub mod credential_pool;
pub mod credential_store;
pub mod errors;
pub mod interrupt_compat;
pub mod kanban_stop;
pub mod lmstudio_reasoning;
pub mod managed_scope;
pub mod manual_compression_feedback;
pub mod message_content;
pub mod portal_tags;
pub mod reasoning_summaries;
pub mod tool_result_classification;
pub mod turn_retry_state;
pub mod verify_hooks;
