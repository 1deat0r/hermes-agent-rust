//! Provider profiles and registry surfaces from `providers/`.

pub mod base;
/// Pure and injected Z.AI endpoint discovery helpers.
pub mod zai;

pub use zai::{
    choose_zai_endpoint, detect_zai_endpoint, detect_zai_endpoint_with_probe, probe_zai_endpoint,
    probe_zai_endpoint_http, probe_zai_endpoint_http_at, resolve_zai_base_url, zai_endpoint_specs,
    ZaiEndpointResult, ZaiEndpointSpec, ZAI_ENDPOINTS,
};
pub(crate) mod profiles;
pub mod registry;

pub use base::{FixedTemperature, ModelsFetchMode, ProviderProfile, OMIT_TEMPERATURE};
pub use registry::{
    discover_with_loader, get_provider_profile, list_providers, plugin_module_name,
    register_provider, user_plugins_dir, ProviderSource,
};
