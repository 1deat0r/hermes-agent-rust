//! xAI (Grok) provider profile.
//!
//! PARITY: `plugins/model-providers/xai/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("xai");
    profile.aliases = vec!["grok".into(), "x-ai".into(), "x.ai".into()];
    profile.api_mode = "codex_responses".into();
    profile.env_vars = vec!["XAI_API_KEY".into()];
    profile.base_url = "https://api.x.ai/v1".into();
    profile.auth_type = "api_key".into();
    // PARITY: upstream imports hermes_cli.__version__ (0.20.0 at b9aa928).
    // Keep the pinned value until the future hermes-cli crate supplies the
    // runtime version to statically linked provider profiles.
    profile
        .default_headers
        .insert("User-Agent".into(), "Hermes-Agent/0.20.0".into());
    profile
}
