//! GitHub Copilot / GitHub Models provider profile.
//!
//! PARITY: `plugins/model-providers/copilot/__init__.py` @ b9aa928.
//! Copilot uses a catalog-gated reasoning hook for its chat-completions subset.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("copilot");
    profile.aliases = vec![
        "github-copilot".into(),
        "github-models".into(),
        "github-model".into(),
        "github".into(),
    ];
    profile.env_vars = vec![
        "COPILOT_GITHUB_TOKEN".into(),
        "GH_TOKEN".into(),
        "GITHUB_TOKEN".into(),
    ];
    profile.base_url = "https://api.githubcopilot.com".into();
    profile.auth_type = "copilot".into();
    // PARITY: CopilotProfile.build_api_kwargs_extras() uses the live model
    // catalog to clamp supported reasoning efforts.
    profile.copilot_reasoning = true;
    profile
}
