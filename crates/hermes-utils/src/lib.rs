//! `hermes-utils` — 1:1 Rust port of `utils.py` (Nous Research Hermes Agent,
//! pinned @ b9aa928).
//!
//! Upstream contract: "Shared utility functions for hermes-agent." This crate
//! is the second workspace root: it may depend on nothing in the workspace
//! (currently used by nothing in the workspace; hermes-constants remains the
//! dependency-free base).
//!
//! Port status: **complete** ✅ (P1). PARITY notes in each module. Known
//! documented divergences (see PLAN.md §5):
//! - `atomic_yaml_write` / roundtrip YAML rendering uses serde_yaml, not
//!   PyYAML/ruamel; value schema and atomicity match, byte-formatting differs.
//! - `atomic_roundtrip_yaml_update` preserves comments for scalar leaf
//!   updates; complex/multi-line updates fall back to full rewrite.

pub mod atomic;
pub mod json;
pub mod proxy;
pub mod truthy;
pub mod urls;
pub mod yaml;

pub use atomic::{
    atomic_json_write, atomic_replace, atomic_write_text, preserve_file_mode,
    preserve_file_owner, restore_file_mode, restore_file_owner,
    warn_if_credential_file_broadly_readable,
};
pub use json::{safe_json_loads, safe_json_loads_typed};
pub use proxy::{normalize_proxy_env_vars, normalize_proxy_url, PROXY_ENV_KEYS};
pub use truthy::{
    env_bool, env_float, env_int, env_var_enabled, is_truthy, is_truthy_value,
    TruthyValue, TRUTHY_STRINGS,
};
pub use urls::{base_url_host_matches, base_url_hostname, model_forces_max_completion_tokens};
pub use yaml::{
    atomic_roundtrip_yaml_update, atomic_yaml_write, fast_safe_load, render_yaml,
    roundtrip_update_text,
};
