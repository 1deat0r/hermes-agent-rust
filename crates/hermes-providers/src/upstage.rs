//! Upstage Solar provider profile.
//!
//! PARITY: `plugins/model-providers/upstage/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("upstage");
    profile.aliases = vec!["solar".into()];
    profile.display_name = "Upstage Solar".into();
    profile.description = "Upstage (Solar API)".into();
    profile.signup_url = "https://console.upstage.ai/api-keys".into();
    profile.env_vars = vec!["UPSTAGE_API_KEY".into(), "UPSTAGE_BASE_URL".into()];
    profile.base_url = "https://api.upstage.ai/v1".into();
    profile.auth_type = "api_key".into();
    profile.fallback_models = vec!["solar-pro3".into()];
    // PARITY: UpstageProfile owns Solar's deny-list and top-level
    // reasoning_effort mapping.
    profile.upstage_reasoning = true;
    profile
}
