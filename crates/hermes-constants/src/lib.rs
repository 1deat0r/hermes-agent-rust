//! `hermes-constants` — 1:1 Rust port of `hermes_constants.py`
//! (Nous Research Hermes Agent, pinned @ b9aa928).
//!
//! Upstream contract: "Import-safe module with no dependencies — can be
//! imported from anywhere without risk of circular imports." This crate is
//! the dependency-free root of the workspace: every other crate may depend
//! on it; it depends on nothing in the workspace.
//!
//! Port status: **foundational subset**, P1. See PLAN.md §5 for the
//! per-function parity matrix. Deferred surfaces (node bootstrap, profile
//! home, reasoning-config resolution, `apply_ipv4_preference`,
//! `partial_update_hint`) ship with their owning subsystems.

pub mod home;
pub mod modules;
pub mod paths;
pub mod platform;
pub mod probe;
pub mod reasoning;
pub mod styles;
pub mod values;
pub mod venv;

// The upstream tests monkeypatch process-global environment/config state.
// Share one lock across Rust unit-test modules so those cases remain isolated
// when lib tests run in parallel.
#[cfg(test)]
pub(crate) static TEST_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub use home::{
    apply_subprocess_home_env, get_default_hermes_root, get_hermes_home,
    get_hermes_home_override, get_process_hermes_home, get_real_home,
    get_subprocess_home, is_profile_home, iter_real_home_candidates,
    norm_home_path, profile_home_path, reset_hermes_home_override,
    set_hermes_home_override, EnvMap, OverrideToken, Platform,
};
pub use modules::{is_first_party_module, FIRST_PARTY_MODULE_ROOTS};
pub use paths::{
    display_hermes_home, get_bundled_skills_dir, get_config_path, get_env_path,
    get_hermes_dir, get_optional_mcps_dir, get_optional_skills_dir,
    get_skills_dir, secure_parent_dir,
};
pub use platform::{
    candidate_node_command_names, is_container, is_termux, is_wsl,
    iter_hermes_node_dirs, translate_cwd_for_wsl_backend, windows_path_to_wsl,
    wsl_unc_path_to_posix,
};
pub use reasoning::{
    canonical_model_variants, parse_reasoning_effort,
    resolve_per_model_reasoning_effort, EffortInput, ReasoningConfig,
    ReasoningOverrideMap, VALID_REASONING_EFFORTS,
};
pub use styles::{DEFAULT_INDICATOR_STYLE, INDICATOR_STYLES};
pub use values::{
    AI_GATEWAY_BASE_URL, FINISH_REASON_LENGTH, OPENROUTER_BASE_URL,
    OPENROUTER_MODELS_URL, PARTIAL_STREAM_STUB_ID,
};
pub use venv::{venv_bin_dir, venv_python_path};
