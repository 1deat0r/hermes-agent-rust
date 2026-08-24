//! Custom/Ollama local provider profile.
//!
//! PARITY: `plugins/model-providers/custom/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("custom");
    profile.aliases = vec![
        "ollama".into(),
        "local".into(),
        "vllm".into(),
        "llamacpp".into(),
        "llama.cpp".into(),
        "llama-cpp".into(),
    ];
    profile.default_max_tokens = Some(65_536);
    // PARITY: CustomProfile owns both the user-configured catalog guard
    // and the Ollama/OpenAI-compatible reasoning wire hook.
    profile.custom_provider = true;
    profile
}
