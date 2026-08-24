//! Ollama Cloud provider profile.
//!
//! PARITY: plugins/model-providers/ollama-cloud/__init__.py @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("ollama-cloud");
    profile.aliases = vec!["ollama_cloud".into()];
    profile.env_vars = vec!["OLLAMA_API_KEY".into()];
    profile.base_url = "https://ollama.com/v1".into();
    profile.default_aux_model = "nemotron-3-nano:30b".into();
    // PARITY: OllamaCloudProfile emits top-level reasoning_effort only when
    // the transport confirms the model's native thinking capability.
    profile.ollama_cloud_reasoning = true;
    profile
}
