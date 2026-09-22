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
pub mod audio_container;
pub mod binary_extensions;
pub mod blueprints;
pub mod browser_camofox_state;
pub mod browser_dialog_tool;
pub mod budget_config;
pub mod computer_use_schema;
pub mod credential_files;
pub mod daemon_pool;
pub mod debug_helpers;
pub mod delegation_output_schema;
pub mod desktop_ui;
pub mod env_probe;
pub mod fal_common;
pub mod file_safety;
pub mod file_state;
pub mod focus_pane_tool;
pub mod html5_entities;
pub mod interrupt;
pub mod mcp_dashboard_oauth;
pub mod mcp_schema_cache;
pub mod mcp_stdio_watchdog;
pub mod open_preview_tool;
pub mod path_security;
pub mod read_extract;
pub mod read_preview_tool;
pub mod read_terminal_tool;
pub mod skill_provenance;
pub mod slash_confirm;
pub mod terminal_hints;
pub mod thread_context;
pub mod threat_patterns;
pub mod todo_tool;
pub mod tool_backend_helpers;
pub mod tool_output_limits;
pub mod tool_result_storage;
pub mod tts_text_normalize;
pub mod website_policy;
pub mod working_diff;

/// Availability helper for session_search's check_fn (hermes_state db home).
pub fn session_search_check_expr() -> bool {
    // The state DB is considered available when the bundled SQLite opens.
    // Keep this a simple probe rather than importing state internals.
    true
}
pub mod clarify;
pub mod close_terminal_tool;
pub mod registry;
pub mod schema_sanitizer;
pub mod session_search;

pub use ansi_strip::{sanitize_display_text, strip_ansi, strip_unicode_tags};
pub use binary_extensions::{is_binary_extension, BINARY_EXTENSIONS};
pub use budget_config::{budget_for_context_window, BudgetConfig, BudgetThreshold};
pub use clarify::{register_clarify, set_clarify_callback};
pub use file_safety::{
    build_write_approval_paths, build_write_denied_paths, build_write_denied_prefixes,
    classify_container_mirror_target, classify_sandbox_mirror_target, classify_write_denial,
    get_container_mirror_warning, get_nt_namespace_error, get_read_block_error,
    get_safe_write_roots, get_sandbox_mirror_warning, get_write_denied_error, hermes_dirs,
    hermes_home_path, hermes_root_path, is_nt_namespace_path, is_write_approval_required,
    is_write_denied, raise_if_read_blocked, resolve_active_profile_name, MirrorInfo,
};
pub use file_state::{
    check_stale, forget_task, get_registry, guard_disabled, known_reads, lock_path, note_write,
    record_read, writes_since, FileStateRegistry, PathGuard,
};
pub use path_security::validate_within_dir;
pub use read_extract::{
    anydoc, anydoc_cache_is_unset, anydoc_retry_seconds, coverage_note_from_texts, extract_anydoc,
    extract_anydoc_bytes, extract_document_bytes, extract_document_text, hosted_ocr_available,
    is_extractable_document, max_anydoc_bytes, needs_ocr_warning, parse_pdftotext_output,
    pdf_coverage_note, pdf_coverage_note_from_bytes, pdf_page_texts, reset_anydoc_cache,
    reset_hosted_ocr_for_test, set_anydoc_provider, set_anydoc_retry_seconds, set_max_anydoc_bytes,
    set_pdftotext_override_for_test, wire_hosted_ocr, AnydocConverter, AnydocError,
    ExtractionError, ANYDOC_EXTENSIONS, EXTRACTABLE_EXTENSIONS, MAX_DOCUMENT_BYTES,
    PDF_GAP_MAP_MAX_ENTRIES,
};
pub use registry::{
    registry, tool_error, tool_result, CheckFnCache, ToolEntry, ToolHandler, ToolRegistry,
    ToolResult,
};
pub use schema_sanitizer::{
    collapse_const_unions, sanitize_property_key, sanitize_tool_schemas, strip_nullable_unions,
    strip_pattern_and_format, strip_slash_enum, unrename_tool_args,
};
pub use session_search::{register_session_search, session_search};
pub use tool_result_storage::{enforce_turn_budget, generate_preview, maybe_persist_tool_result};
pub use tts_text_normalize::{
    normalize_symbols_for_tts, prepare_spoken_text, smooth_whitespace_for_tts,
    strip_markdown_for_tts, strip_nonspoken_blocks,
};
