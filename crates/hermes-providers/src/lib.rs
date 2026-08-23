//! Provider profiles and registry surfaces from `providers/`.

pub mod base;
pub(crate) mod profiles;
pub mod registry;

pub use base::{FixedTemperature, ModelsFetchMode, ProviderProfile, OMIT_TEMPERATURE};
pub use registry::{
    discover_with_loader, get_provider_profile, list_providers, plugin_module_name,
    register_provider, user_plugins_dir, ProviderSource,
};
