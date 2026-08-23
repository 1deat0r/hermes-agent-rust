//! OpenAI Codex (Responses API) provider profile.
//!
//! PARITY: `plugins/model-providers/openai-codex/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("openai-codex");
    profile.aliases = vec!["codex".into(), "openai_codex".into()];
    profile.api_mode = "codex_responses".into();
    profile.base_url = "https://chatgpt.com/backend-api/codex".into();
    profile.auth_type = "oauth_external".into();
    profile
}
