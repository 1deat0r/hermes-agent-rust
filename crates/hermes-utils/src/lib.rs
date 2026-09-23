//! `hermes-utils` — 1:1 Rust port of `utils.py` (Nous Research Hermes Agent,
//! pinned @ 5d59366).
//!
//! Upstream contract: "Shared utility functions for hermes-agent." This crate
//! sits at the workspace floor alongside `hermes-constants` (no workspace
//! dependencies) and is consumed by `hermes-tools`, `hermes-cli`, and
//! `hermes-agent`.
//!
//! Port status: **complete** @ 5d59366 (re-cert 2026-09-22). PARITY notes in
//! each module (PORT SEAMS). Key documented divergences (PLAN.md §5):
//! - YAML emission uses this crate's IndentDuzzer-shaped renderer, not
//!   PyYAML/ruamel; value schema, 2-space sequences (#31999) and
//!   atomicity match, byte-formatting differs.
//! - `atomic_roundtrip_yaml_update` preserves comments for scalar leaf
//!   updates (complex values fall back to a full rewrite);
//!   `atomic_roundtrip_yaml_save` merges line-wise and keeps comments.
//! - `env_bool` honors `default` on a missing variable (upstream's default
//!   is unreachable there — no in-tree caller relied on it).
//! - `atomic_write_text`'s `encoding`/`tmp_prefix` knobs and
//!   `_dump_json`'s lone-surrogate retry are unported (UTF-8-only Rust
//!   strings; no callers).
//! - The oracle's caplog sink maps to the `log` facade.

pub mod atomic;
pub mod json;
pub mod proxy;
pub mod truthy;
pub mod urls;
pub mod yaml;

pub use atomic::{
    atomic_json_write, atomic_replace, atomic_write_bytes, atomic_write_text,
    default_new_file_mode, file_signature, force_windows_contended_for_test, fsync_directory,
    preserve_file_mode, preserve_file_owner, reset_owner_seams_for_test,
    reset_replace_hook_for_test, reset_replace_retry_delays_for_test, restore_file_mode,
    restore_file_owner, rewrite_in_place, set_owner_seams_for_test, set_replace_hook_for_test,
    set_replace_retry_delays_for_test, warn_if_credential_file_broadly_readable,
    REPLACE_RETRY_ATTEMPTS,
};
pub use json::{read_json_or_empty, safe_json_loads, safe_json_loads_typed};
pub use proxy::{normalize_proxy_env_vars, normalize_proxy_url, PROXY_ENV_KEYS};
pub use truthy::{
    env_bool, env_float, env_int, env_var_enabled, is_truthy, is_truthy_value, TruthyValue,
    TRUTHY_STRINGS,
};
pub use urls::{
    base_url_host_matches, base_url_hostname, base_url_origin, model_forces_max_completion_tokens,
};
pub use yaml::{
    atomic_roundtrip_yaml_save, atomic_roundtrip_yaml_update, atomic_yaml_write, fast_safe_load,
    render_yaml, roundtrip_update_text,
};
