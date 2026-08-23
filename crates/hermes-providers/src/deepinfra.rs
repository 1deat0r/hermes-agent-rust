//! DeepInfra provider profile.
//!
//! PARITY: `plugins/model-providers/deepinfra/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("deepinfra");
    profile.aliases = vec!["deep-infra".into(), "deepinfra-ai".into()];
    profile.display_name = "DeepInfra".into();
    profile.description = "DeepInfra — 100+ open models, pay-per-use".into();
    profile.signup_url = "https://deepinfra.com/dash/api_keys".into();
    profile.env_vars = vec!["DEEPINFRA_API_KEY".into(), "DEEPINFRA_BASE_URL".into()];
    profile.base_url = "https://api.deepinfra.com/v1/openai".into();
    profile.auth_type = "api_key".into();
    profile.default_aux_model = "deepseek-ai/DeepSeek-V4-Flash".into();
    // PARITY: DeepInfraProfile.default_vision_model() owns live vision-model
    // discovery so the future vision resolver stays provider-agnostic.
    profile.deepinfra_vision = true;
    profile
}
