//! Google Gemini provider profile.
//!
//! PARITY: `plugins/model-providers/gemini/__init__.py` @ b9aa928.
//! The profile uses the native Gemini thinking-config translation hook.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("gemini");
    profile.aliases = vec![
        "google".into(),
        "google-gemini".into(),
        "google-ai-studio".into(),
    ];
    profile.api_mode = "chat_completions".into();
    profile.env_vars = vec!["GOOGLE_API_KEY".into(), "GEMINI_API_KEY".into()];
    profile.base_url = "https://generativelanguage.googleapis.com/v1beta".into();
    profile.auth_type = "api_key".into();
    profile.default_aux_model = "gemini-3.6-flash".into();
    // PARITY: GeminiProfile.build_extra_body() supplies native or OpenAI-
    // compatibility thinking_config depending on the resolved base URL.
    profile.gemini_thinking = true;
    profile
}
