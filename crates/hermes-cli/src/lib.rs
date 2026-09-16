//! CLI-layer parity surfaces from `hermes_cli/`.
//!
//! Leaf modules only: everything here is dependency-free at the Hermes level
//! so this crate can sit below the agent, gateway, and tool layers that import
//! it upstream.

pub mod build_info;
pub mod cli_output;
pub mod colors;
pub mod default_soul;
pub mod dashboard_auth;
pub mod git_revision;
pub mod input_sanitize;
pub mod lifecycle;
pub mod model_search;
pub mod secret_prompt;
pub mod setup_hidden_env;
pub mod sqlite_util;
pub mod timefmt;
pub mod timeouts;
pub mod toolset_validation;

/// PARITY: `hermes_cli.__version__` (upstream `hermes_cli/__init__.py` line 17),
/// the single string `scripts/release.py` regex-bumps at release time.
pub const VERSION: &str = "0.21.3";

/// PARITY: `hermes_cli.__release_date__` (upstream line 18), the calendar
/// version `scripts/release.py` rewrites alongside the version.
pub const RELEASE_DATE: &str = "2026.9.14";

/// Resolve the baked-in build SHA through the crate that owns it.
///
/// Convenience re-export so `hermes dump`/banner equivalents have one entry
/// point, matching how upstream imports the two names from the package root.
pub fn build_sha() -> Option<String> {
    build_info::get_build_sha(build_info::DEFAULT_SHORT)
}
