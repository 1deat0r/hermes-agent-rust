//! Fireworks AI provider profile.
//!
//! PARITY: `plugins/model-providers/fireworks/__init__.py` @ b9aa928.
//! Fireworks serves OpenAI-compatible chat completions for direct catalog IDs.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("fireworks");
    profile.aliases = vec!["fireworks-ai".into(), "fw".into()];
    profile.display_name = "Fireworks AI".into();
    profile.description = "Fireworks AI — OpenAI-compatible direct model API".into();
    profile.signup_url = "https://app.fireworks.ai/settings/users/api-keys".into();
    profile.env_vars = vec!["FIREWORKS_API_KEY".into()];
    profile.base_url = "https://api.fireworks.ai/inference/v1".into();
    profile.auth_type = "api_key".into();
    // PARITY: upstream imports hermes_cli.__version__; the future CLI seam
    // can replace this pinned b9aa928 value at runtime.
    profile.default_headers.insert(
        "HTTP-Referer".into(),
        "https://hermes-agent.nousresearch.com".into(),
    );
    profile
        .default_headers
        .insert("X-Title".into(), "Hermes Agent".into());
    profile
        .default_headers
        .insert("User-Agent".into(), "HermesAgent/0.20.0".into());
    profile.default_aux_model = "accounts/fireworks/models/glm-5p2".into();
    profile.fallback_models = vec![
        "accounts/fireworks/models/kimi-k2p6".into(),
        "accounts/fireworks/models/glm-5p2".into(),
        "accounts/fireworks/models/kimi-k2p7-code".into(),
    ];
    profile
}
