//! NVIDIA NIM provider profile.
//!
//! PARITY: `plugins/model-providers/nvidia/__init__.py` @ 5d59366.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("nvidia");
    profile.aliases = vec![
        "nvidia-nim".into(),
        "nim".into(),
        "build-nvidia".into(),
        "nemotron".into(),
    ];
    profile.env_vars = vec!["NVIDIA_API_KEY".into()];
    profile.display_name = "NVIDIA NIM".into();
    profile.description = "NVIDIA NIM — accelerated inference".into();
    profile.signup_url = "https://build.nvidia.com/".into();
    profile.fallback_models = vec![
        "nvidia/llama-3.1-nemotron-70b-instruct".into(),
        "nvidia/llama-3.3-70b-instruct".into(),
    ];
    profile.base_url = "https://integrate.api.nvidia.com/v1".into();
    profile.default_max_tokens = Some(16_384);
    // PARITY: `NvidiaProviderProfile.prepare_messages` — NVIDIA NIM accepts a
    // stricter ToolMessage schema than most OpenAI-compatible APIs.
    profile.nvidia_tool_strip = true;
    profile
}
