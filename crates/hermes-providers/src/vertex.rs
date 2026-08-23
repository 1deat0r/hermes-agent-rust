//! Google Vertex AI provider profile.
//!
//! PARITY: `plugins/model-providers/vertex/__init__.py` @ b9aa928.
//! Vertex uses OAuth2 and an OpenAI-compatible endpoint with no REST catalog.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("vertex");
    profile.aliases = vec![
        "google-vertex".into(),
        "vertex-ai".into(),
        "gcp-vertex".into(),
    ];
    profile.api_mode = "chat_completions".into();
    profile.base_url = "https://aiplatform.googleapis.com".into();
    profile.auth_type = "vertex".into();
    profile.default_aux_model = "google/gemini-3.6-flash".into();
    // PARITY: VertexProfile.fetch_models() always returns None because its
    // setup wizard owns the curated model list rather than REST discovery.
    profile.models_fetch_disabled = true;
    // PARITY: VertexProfile always emits Gemini's nested OpenAI-compatible
    // `extra_body.google.thinking_config` shape.
    profile.vertex_thinking = true;
    profile
}
