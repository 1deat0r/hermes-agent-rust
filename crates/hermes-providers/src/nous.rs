//! Nous Portal provider profile.
//!
//! PARITY: `plugins/model-providers/nous/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("nous");
    profile.aliases = vec!["nous-portal".into(), "nousresearch".into()];
    profile.display_name = "Nous Research".into();
    profile.description = "Nous Research — Hermes model family".into();
    profile.signup_url = "https://nousresearch.com/".into();
    profile.env_vars = vec!["NOUS_API_KEY".into()];
    profile.base_url = "https://inference-api.nousresearch.com/v1".into();
    profile.auth_type = "oauth_device_code".into();
    profile.fallback_models = vec!["hermes-3-405b".into(), "hermes-3-70b".into()];
    // PARITY: NousProfile owns Portal tags/sticky routing and the disabled
    // reasoning omission rule through this explicit profile capability.
    profile.nous_portal = true;
    profile
}
