//! Kilo Code provider profile.
//!
//! PARITY: `plugins/model-providers/kilocode/__init__.py` @ b9aa928.

use crate::base::ProviderProfile;

pub(crate) fn profile() -> ProviderProfile {
    let mut profile = ProviderProfile::new("kilocode");
    profile.aliases = vec!["kilo-code".into(), "kilo".into(), "kilo-gateway".into()];
    profile.env_vars = vec!["KILOCODE_API_KEY".into()];
    profile.base_url = "https://api.kilo.ai/api/gateway".into();
    profile.default_aux_model = "google/gemini-3.6-flash".into();
    profile
}
