//! Native Anthropic provider profile.
//!
//! PARITY: `plugins/model-providers/anthropic/__init__.py` @ b9aa928.
//! Anthropic uses native Messages API discovery with `x-api-key` auth.

use crate::base::{ModelsFetchMode, ProviderProfile};

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("anthropic");
    profile.aliases = vec!["claude".into(), "claude-oauth".into(), "claude-code".into()];
    profile.api_mode = "anthropic_messages".into();
    profile.env_vars = vec![
        "ANTHROPIC_API_KEY".into(),
        "ANTHROPIC_TOKEN".into(),
        "CLAUDE_CODE_OAUTH_TOKEN".into(),
    ];
    profile.base_url = "https://api.anthropic.com".into();
    profile.auth_type = "api_key".into();
    profile.default_aux_model = "claude-haiku-4-5-20251001".into();
    // PARITY: AnthropicProfile.fetch_models() uses the native endpoint and
    // x-api-key/anthropic-version headers instead of the Bearer default.
    profile.models_fetch_mode = ModelsFetchMode::Anthropic;
    profile
}
