//! MiniMax provider profiles.
//!
//! PARITY: `plugins/model-providers/minimax/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("minimax");
    profile.aliases = vec!["mini-max".into()];
    profile.api_mode = "anthropic_messages".into();
    profile.env_vars = vec!["MINIMAX_API_KEY".into()];
    profile.base_url = "https://api.minimax.io/anthropic".into();
    profile.auth_type = "api_key".into();
    profile.default_aux_model = "MiniMax-M3".into();
    // PARITY: MiniMaxProfile owns the M3 OpenAI-compatible reasoning hook.
    profile.minimax_reasoning = true;
    profile
}

pub(crate) fn china_profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("minimax-cn");
    profile.aliases = vec!["minimax-china".into(), "minimax_cn".into()];
    profile.api_mode = "anthropic_messages".into();
    profile.env_vars = vec!["MINIMAX_CN_API_KEY".into()];
    profile.base_url = "https://api.minimaxi.com/anthropic".into();
    profile.auth_type = "api_key".into();
    profile.default_aux_model = "MiniMax-M3".into();
    profile.minimax_reasoning = true;
    profile
}

pub(crate) fn oauth_profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("minimax-oauth");
    profile.aliases = vec!["minimax_oauth".into(), "minimax-oauth-io".into()];
    profile.api_mode = "anthropic_messages".into();
    profile.display_name = "MiniMax (OAuth)".into();
    profile.description = "MiniMax via OAuth browser flow — no API key required".into();
    profile.signup_url = "https://api.minimax.io/".into();
    profile.base_url = "https://api.minimax.io/anthropic".into();
    profile.auth_type = "oauth_external".into();
    profile.default_aux_model = "MiniMax-M2.7".into();
    profile.minimax_reasoning = true;
    profile
}
