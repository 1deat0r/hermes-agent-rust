//! GitHub Copilot ACP provider profile.
//!
//! PARITY: `plugins/model-providers/copilot-acp/__init__.py` @ b9aa928.
//! copilot-acp uses an external ACP subprocess rather than the standard REST
//! transport; its profile retains the registry metadata for that integration.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("copilot-acp");
    profile.aliases = vec!["github-copilot-acp".into(), "copilot-acp-agent".into()];
    // PARITY: the upstream profile explicitly uses chat_completions routing;
    // the ACP subprocess owns the external transport details.
    profile.api_mode = "chat_completions".into();
    // PARITY: the upstream subprocess manages authentication and exposes no
    // environment variables to the standard provider loader.
    profile.base_url = "acp://copilot".into();
    profile.auth_type = "external_process".into();
    // PARITY: CopilotACPProfile.fetch_models() always returns None because
    // model listing is handled by the ACP subprocess.
    profile.models_fetch_disabled = true;
    profile
}
