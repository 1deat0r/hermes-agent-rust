//! Qwen Portal provider profile.
//!
//! PARITY: `plugins/model-providers/qwen-oauth/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("qwen-oauth");
    profile.aliases = vec!["qwen".into(), "qwen-portal".into(), "qwen-cli".into()];
    profile.env_vars = vec!["QWEN_API_KEY".into()];
    profile.base_url = "https://portal.qwen.ai/v1".into();
    profile.auth_type = "oauth_external".into();
    profile.default_max_tokens = Some(65_536);
    // PARITY: QwenProfile owns message normalization, high-resolution image
    // requests, and top-level session metadata instead of extra_body metadata.
    profile.qwen_portal = true;
    profile
}
