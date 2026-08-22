//! hermes-toolsets — 1:1 Rust port of toolsets.py + toolset_distributions.py
//! (2,956 upstream LOC combined).
//!
//! Toolset composition/resolution: named groups of tools that can include
//! other toolsets, plus probability-weighted distributions used by data
//! generation runs. Port target: upstream @ b9aa928.
//!
//! DEFERRED REGISTRY SEAM: upstream merges plugin/overlay tools via
//! `tools.registry` (TOOLSETS overlays, plugin platforms, MCP aliases). The
//! registry crate ships with the tools layer (P2); until then the registry
//! lookup returns empty and the static view is used for every path (the
//! `include_registry` parameter is accepted but behaves as False). The seam
//! is named `registry_lookup` in toolsets.rs so the tools crate can wire it
//! without touching callers.

pub mod data;
pub mod distributions;
pub mod toolsets;

pub use distributions::{
    get_distribution, list_distributions, sample_toolsets_from_distribution,
    validate_distribution,
};
pub use toolsets::{
    bundle_non_core_tools, create_custom_toolset, get_all_toolsets, get_toolset,
    get_toolset_info, get_toolset_names, resolve_multiple_toolsets, resolve_toolset,
    validate_toolset,
};
