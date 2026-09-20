//! Jev tool-selection seam for the agent inner loop.
//!
//! Re-exports the [`hermes_jev::tool_route`] decision surface next to the
//! toolset registry so loop callers resolve names and ask Jev in one
//! place. Config-gated default OFF at the caller; selection never
//! throws and returns `None` (keep legacy policy) on any failure.

pub use hermes_jev::tool_route::{
    select_next_tool, select_next_tool_with_model, tool_question, ToolDecision, MAX_LOG_CHARS,
    NO_TOOL_OPTION, TOOL_QUESTION_ID,
};
