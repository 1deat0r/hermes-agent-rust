//! Arcee AI provider profile.
//!
//! PARITY: `plugins/model-providers/arcee/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("arcee");
    profile.aliases = vec!["arcee-ai".into(), "arceeai".into()];
    profile.env_vars = vec!["ARCEEAI_API_KEY".into()];
    profile.base_url = "https://api.arcee.ai/api/v1".into();
    profile
}
