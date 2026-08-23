//! Xiaomi MiMo provider profile.
//!
//! PARITY: `plugins/model-providers/xiaomi/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("xiaomi");
    profile.aliases = vec!["mimo".into(), "xiaomi-mimo".into()];
    profile.env_vars = vec!["XIAOMI_API_KEY".into()];
    profile.base_url = "https://api.xiaomimimo.com/v1".into();
    profile.supports_health_check = false;
    profile.supports_vision = true;
    profile.supports_vision_tool_messages = false;
    profile
}
