//! Z.AI / GLM provider profile.
//!
//! PARITY: plugins/model-providers/zai/__init__.py @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("zai");
    profile.aliases = vec!["glm".into(), "z-ai".into(), "z.ai".into(), "zhipu".into()];
    profile.display_name = "Z.AI (GLM)".into();
    profile.description = "Z.AI / GLM — Zhipu AI models".into();
    profile.signup_url = "https://z.ai/".into();
    profile.env_vars = vec![
        "GLM_API_KEY".into(),
        "ZAI_API_KEY".into(),
        "Z_AI_API_KEY".into(),
    ];
    profile.fallback_models = vec!["glm-5.2".into(), "glm-5".into(), "glm-4-9b".into()];
    profile.base_url = "https://api.z.ai/api/paas/v4".into();
    profile.default_aux_model = "glm-4.5-flash".into();
    // PARITY: ZaiProfile owns GLM version gating, thinking toggles, and
    // GLM-5.2's top-level reasoning_effort mapping.
    profile.zai_reasoning = true;
    profile
}
