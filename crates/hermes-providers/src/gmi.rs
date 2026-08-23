//! GMI Cloud provider profile.
//!
//! PARITY: `plugins/model-providers/gmi/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("gmi");
    profile.aliases = vec!["gmi-cloud".into(), "gmicloud".into()];
    profile.display_name = "GMI Cloud".into();
    profile.description = "GMI Cloud — multi-model direct API (slash-form model IDs)".into();
    profile.signup_url = "https://www.gmicloud.ai/".into();
    profile.env_vars = vec!["GMI_API_KEY".into(), "GMI_BASE_URL".into()];
    profile.base_url = "https://api.gmi-serving.com/v1".into();
    profile.auth_type = "api_key".into();
    // PARITY: upstream imports hermes_cli.__version__; the future CLI seam
    // can replace this pinned b9aa928 value at runtime.
    profile
        .default_headers
        .insert("User-Agent".into(), "HermesAgent/0.20.0".into());
    profile.default_aux_model = "google/gemini-3.1-flash-lite-preview".into();
    profile.fallback_models = vec![
        "zai-org/GLM-5.1-FP8".into(),
        "deepseek-ai/DeepSeek-V3.2".into(),
        "moonshotai/Kimi-K2.5".into(),
        "google/gemini-3.1-flash-lite-preview".into(),
        "anthropic/claude-sonnet-5".into(),
        "anthropic/claude-sonnet-4.6".into(),
        "openai/gpt-5.4".into(),
    ];
    profile
}
