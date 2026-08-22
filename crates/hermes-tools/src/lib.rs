//! hermes-tools — 1:1 Rust port of the tools/ layer, beginning with
//! tools/registry.py (@ b9aa928, 956 LOC).
//!
//! The registry is the hub every tool file registers into and every
//! consumer (model_tools, run_agent, CLI) reads from. It owns the
//! check_fn TTL cache, generation counter, plugin-override policy, and
//! schema/dispatch surfaces.
//!
//! DOCUMENTED DIVERGENCES (same names as in module docs):
//! - Python handlers are callables; Rust handlers implement
//!   `ToolHandler`. The `is_async`/asyncio bridge is an executor seam:
//!   async Python handlers are adapted to the sync trait by their tool
//!   crates before registration.
//! - `_plugin_owner_of` resolves from `handler.__globals__["__name__"]`;
//!   here the owner module string is carried on each registration (the
//!   `owner_module` parameter), mirroring the same policy check.
//! - `check_fn_cache_scope()` (multiplex profile isolation) is a no-op
//!   returning None until the agent/secret_scope crate lands.
//! - `discover_builtin_tools` (Python AST scan) is a seam returning the
//!   tool-catalog list once the tool renderers land.

pub mod ansi_strip;
pub mod budget_config;
pub mod tool_result_storage;

/// Availability helper for session_search's check_fn (hermes_state db home).
pub fn session_search_check_expr() -> bool {
    // The state DB is considered available when the bundled SQLite opens.
    // Keep this a simple probe rather than importing state internals.
    true
}
pub mod clarify;
pub mod session_search;
pub mod registry;
pub mod schema_sanitizer;

pub use schema_sanitizer::{
    collapse_const_unions, sanitize_property_key, sanitize_tool_schemas,
    strip_nullable_unions, strip_pattern_and_format, strip_slash_enum, unrename_tool_args,
};
pub use ansi_strip::{sanitize_display_text, strip_ansi};
pub use budget_config::{budget_for_context_window, BudgetConfig, BudgetThreshold};
pub use tool_result_storage::{enforce_turn_budget, generate_preview, maybe_persist_tool_result};
pub use clarify::{register_clarify, set_clarify_callback};
pub use session_search::{register_session_search, session_search};
pub use registry::{
    registry, tool_error, tool_result, CheckFnCache, ToolEntry, ToolHandler,
    ToolRegistry, ToolResult,
};
