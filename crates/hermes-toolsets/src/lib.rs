//! hermes-toolsets — 1:1 Rust port of toolsets.py + toolset_distributions.py
//! (591 upstream LOC combined).
//!
//! Toolset composition/resolution: named groups of tools that can include
//! other toolsets, plus probability-weighted distributions used by data
//! generation runs. Port target: upstream @ 5d59366.
//!
//! Registry seam: plugin/overlay tools merge through the live
//! `hermes-tools` registry singleton (ToolRegistry) — `get_toolset`,
//! `resolve_toolset` and the memo key read it the way upstream reads
//! `tools.registry`. The one unported upstream dependency is the gateway
//! `platform_registry` behind `_plugin_platform_bundle` (see toolsets.rs
//! PORT SEAMS).

pub mod data;
pub mod distributions;
pub mod jev_tool_route;
pub mod model_tools;
pub mod toolsets;

pub use distributions::{
    get_distribution, inject_distribution, list_distributions, print_distribution_info,
    remove_distribution, sample_toolsets_from_distribution,
    sample_toolsets_from_distribution_verbose, validate_distribution,
};
pub use model_tools::{
    clear_tool_defs_cache, coerce_tool_args, compute_tool_definitions, get_tool_definitions,
    last_resolved_tool_names, sanitize_tool_error,
};
pub use toolsets::{
    bundle_non_core_tools, clear_resolve_toolset_memo, create_custom_toolset, get_all_toolsets,
    get_toolset, get_toolset_info, get_toolset_names, remove_custom_toolset,
    resolve_multiple_toolsets, resolve_toolset, resolve_toolset_hit, validate_toolset,
};
