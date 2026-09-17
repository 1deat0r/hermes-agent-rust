//! Actual Computer provider profile.
//!
//! PARITY: `plugins/model-providers/actual/__init__.py` @ 5d59366.
//! Hosted at api.actual.inc; local inference via model.base_url. Reasoning
//! via `thinking_toggle_extras` over ACTUAL_RELAY_EFFORTS (always-emit
//! toggle). The macOS certifi client-kwargs hook and the config.yaml URL
//! normalization (`hermes_cli.auth`) are above-crate seams — recorded, not
//! guessed.
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
        "Actual Computer - hosted inference via api.actual.inc, or local offline inference via model.base_url in config.yaml".into();
    profile.signup_url = "https://actual.inc".into();
    profile.env_vars = vec!["ACTUAL_API_KEY".into()];
    profile.base_url = "https://api.actual.inc/v1".into();
    profile.auth_type = "api_key".into();
    profile.api_mode = "chat_completions".into();
    // PARITY: ActualProfile reasoning via thinking-toggle extras; the
    // certifi client hook (macOS-only, above-crate `certifi` import) and
    // `normalize_actual_base_url` (hermes_cli.auth) stay deferred seams.
    profile.actual_reasoning = true;
    // PARITY: ActualProfile.fetch_models() normalizes the URL then
    // delegates to super() — modeled by actual_catalog (env-aware hook).
    profile.actual_catalog = true;
    profile
}
