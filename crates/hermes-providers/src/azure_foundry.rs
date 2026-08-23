//! Microsoft Foundry provider profile.
//!
//! PARITY: `plugins/model-providers/azure-foundry/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("azure-foundry");
    profile.aliases = vec!["azure".into(), "azure-ai-foundry".into(), "azure-ai".into()];
    profile.display_name = "Azure Foundry".into();
    profile.description =
        "Microsoft Foundry - OpenAI-compatible endpoint (user-supplied base URL)".into();
    profile.signup_url = "https://ai.azure.com/".into();
    profile.env_vars = vec![
        "AZURE_FOUNDRY_API_KEY".into(),
        "AZURE_FOUNDRY_BASE_URL".into(),
    ];
    // Per-resource endpoints are supplied by the user during setup.
    profile.base_url = "".into();
    profile.auth_type = "api_key".into();
    profile
}
