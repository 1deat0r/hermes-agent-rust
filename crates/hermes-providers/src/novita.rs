//! NovitaAI provider profile.
//!
//! PARITY: `plugins/model-providers/novita/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("novita");
    profile.aliases = vec!["novita-ai".into(), "novitaai".into()];
    profile.display_name = "NovitaAI".into();
    profile.description = "NovitaAI — AI-native cloud for builders and agents".into();
    profile.signup_url = "https://novita.ai/settings/key-management".into();
    profile.env_vars = vec!["NOVITA_API_KEY".into(), "NOVITA_BASE_URL".into()];
    profile.base_url = "https://api.novita.ai/openai/v1".into();
    profile.auth_type = "api_key".into();
    profile.default_aux_model = "deepseek/deepseek-v3-0324".into();
    profile.fallback_models = vec![
        "moonshotai/kimi-k2.5".into(),
        "minimax/minimax-m2.7".into(),
        "zai-org/glm-5".into(),
        "deepseek/deepseek-v3-0324".into(),
        "deepseek/deepseek-r1-0528".into(),
        "qwen/qwen3-235b-a22b-fp8".into(),
    ];
    profile
}
