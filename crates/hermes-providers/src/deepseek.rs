//! DeepSeek provider profile.
//!
//! PARITY: `plugins/model-providers/deepseek/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("deepseek");
    profile.aliases = vec!["deepseek-chat".into()];
    profile.display_name = "DeepSeek".into();
    profile.description = "DeepSeek — native DeepSeek API".into();
    profile.signup_url = "https://platform.deepseek.com/".into();
    profile.env_vars = vec!["DEEPSEEK_API_KEY".into()];
    profile.base_url = "https://api.deepseek.com/v1".into();
    profile.fallback_models = vec!["deepseek-v4-pro".into(), "deepseek-v4-flash".into()];
    profile.default_aux_model = "deepseek-v4-flash".into();
    // PARITY: DeepSeekProfile.build_api_kwargs_extras owns the V4+ thinking
    // body and top-level reasoning_effort mapping.
    profile.deepseek_reasoning = true;
    profile
}
