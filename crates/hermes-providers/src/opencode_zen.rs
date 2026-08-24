//! OpenCode provider profiles (Zen + Go).
//!
//! PARITY: `plugins/model-providers/opencode-zen/__init__.py` lines 1–7 and
//! 130–147 at upstream commit `b9aa928`. Per-model `api_mode` routing from
//! lines 3–7 remains intentionally deferred to the transport layer.

use crate::base::ProviderProfile;

pub(crate) fn zen_profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("opencode-zen");
    // PARITY: OpenCode Zen profile lines 130–136.
    profile.aliases = vec!["opencode".into(), "opencode_zen".into(), "zen".into()];
    profile.env_vars = vec!["OPENCODE_ZEN_API_KEY".into()];
    profile.base_url = "https://opencode.ai/zen/v1".into();
    profile.default_aux_model = "gemini-3-flash".into();
    profile
}

pub(crate) fn go_profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("opencode-go");
    // PARITY: OpenCode Go profile lines 138–143.
    profile.aliases = vec!["opencode_go".into(), "go".into(), "opencode-go-sub".into()];
    profile.env_vars = vec!["OPENCODE_GO_API_KEY".into()];
    profile.base_url = "https://opencode.ai/zen/go/v1".into();
    profile.default_aux_model = "glm-5".into();
    profile.opencode_go_reasoning = true;
    profile
}
