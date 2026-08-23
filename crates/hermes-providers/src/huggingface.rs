//! Hugging Face provider profile.
//!
//! PARITY: `plugins/model-providers/huggingface/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("huggingface");
    profile.aliases = vec!["hf".into(), "hugging-face".into(), "huggingface-hub".into()];
    profile.env_vars = vec!["HF_TOKEN".into()];
    profile.display_name = "HuggingFace".into();
    profile.description = "HuggingFace Inference API".into();
    profile.signup_url = "https://huggingface.co/settings/tokens".into();
    profile.fallback_models = vec![
        "Qwen/Qwen3.5-72B-Instruct".into(),
        "deepseek-ai/DeepSeek-V3.2".into(),
    ];
    profile.base_url = "https://router.huggingface.co/v1".into();
    profile
}
