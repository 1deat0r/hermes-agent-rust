//! Actual Computer provider profile.
//!
//! PARITY: `plugins/model-providers/actual/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("actual");
    profile.aliases = vec![
        "actual-computer".into(),
        "actualcomputer".into(),
        "aci".into(),
    ];
    profile.display_name = "Actual Computer".into();
    profile.description =
        "Actual Computer - hosted inference via api.actual.inc, or local offline inference via ACTUAL_BASE_URL".into();
    profile.signup_url = "https://actual.inc".into();
    profile.env_vars = vec!["ACTUAL_API_KEY".into(), "ACTUAL_BASE_URL".into()];
    profile.base_url = "https://api.actual.inc/v1".into();
    profile.auth_type = "api_key".into();
    profile.api_mode = "codex_responses".into();
    // PARITY: ActualProfile.fetch_models() owns environment-aware URL
    // normalization and optional-auth catalog discovery.
    profile.actual_catalog = true;
    profile
}
