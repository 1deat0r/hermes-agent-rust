//! Kimi / Moonshot provider profiles.
//!
//! PARITY: plugins/model-providers/kimi-coding/__init__.py @ b9aa928.

use crate::base::{ProviderProfile, OMIT_TEMPERATURE};

fn profile_with_name(
    name: &str,
    aliases: &[&str],
    env_vars: &[&str],
    base_url: &str,
) -> ProviderProfile {
    let mut profile = ProviderProfile::new(name);
    profile.aliases = aliases.iter().map(|alias| (*alias).into()).collect();
    profile.env_vars = env_vars.iter().map(|env_var| (*env_var).into()).collect();
    profile.base_url = base_url.into();
    profile.fixed_temperature = OMIT_TEMPERATURE;
    profile.default_max_tokens = Some(32_000);
    profile
        .default_headers
        .insert("User-Agent".into(), "hermes-agent/1.0".into());
    profile.default_aux_model = "kimi-k2-turbo-preview".into();
    // PARITY: KimiProfile owns the Coding endpoint confirmation, k3 catalog
    // filtering, and mutually-exclusive reasoning wire shape.
    profile.kimi_coding = true;
    profile
}

pub(crate) fn profile() -> ProviderProfile {
    profile_with_name(
        "kimi-coding",
        &["kimi", "moonshot", "kimi-for-coding"],
        &["KIMI_API_KEY", "KIMI_CODING_API_KEY"],
        "https://api.moonshot.ai/v1",
    )
}

pub(crate) fn china_profile() -> ProviderProfile {
    profile_with_name(
        "kimi-coding-cn",
        &["kimi-cn", "moonshot-cn"],
        &["KIMI_CN_API_KEY"],
        "https://api.moonshot.cn/v1",
    )
}
