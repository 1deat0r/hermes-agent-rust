//! Module-level registry for DashboardAuthProvider instances.
//!
//! PARITY: `hermes_cli/dashboard_auth/registry.py` @ b9aa928 (whole
//! module).
//!
//! Plugins call [`register_provider`] via the plugin context hook at
//! startup. The auth gate middleware iterates [`list_providers`] and uses
//! [`get_provider`] to dispatch on the session's `provider` field.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use super::base::assert_protocol_compliance;

pub use super::base::{
    assert_protocol_compliance as assert_protocol_compliance_exported, DashboardAuthProvider,
    InvalidCodeError, InvalidCredentialsError, LoginStart, ProviderError, RefreshExpiredError,
    Session, TokenPrincipal,
};

static REGISTRY: OnceLock<Mutex<RegistryState>> = OnceLock::new();

#[derive(Default)]
struct RegistryState {
    /// Insertion-ordered by first registration; `list_providers` returns
    /// registration order.
    order: Vec<String>,
    providers: HashMap<String, Arc<dyn DashboardAuthProvider>>,
}

fn registry() -> &'static Mutex<RegistryState> {
    REGISTRY.get_or_init(|| Mutex::new(RegistryState::default()))
}

/// Register a provider.
///
/// Errors: protocol violation ([`assert_protocol_compliance`]) or a
/// duplicate provider name (upstream raises `ValueError`).
///
/// PARITY: `register_provider` (upstream lines 24-41).
pub fn register_provider(provider: Arc<dyn DashboardAuthProvider>) -> Result<(), String> {
    assert_protocol_compliance(provider.as_ref())?;
    let mut state = registry().lock().unwrap_or_else(|e| e.into_inner());
    if state.providers.contains_key(provider.name()) {
        return Err(format!(
            "dashboard-auth provider already registered: {:?}",
            provider.name()
        ));
    }
    state.order.push(provider.name().to_string());
    state
        .providers
        .insert(provider.name().to_string(), provider);
    Ok(())
}

/// Return the registered provider for `name`, or None if unknown.
///
/// PARITY: `get_provider` (upstream lines 44-47).
pub fn get_provider(name: &str) -> Option<Arc<dyn DashboardAuthProvider>> {
    registry()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .providers
        .get(name)
        .cloned()
}

/// All registered providers, in registration order.
///
/// PARITY: `list_providers` (upstream lines 50-53).
pub fn list_providers() -> Vec<Arc<dyn DashboardAuthProvider>> {
    let state = registry().lock().unwrap_or_else(|e| e.into_inner());
    state
        .order
        .iter()
        .filter_map(|name| state.providers.get(name).cloned())
        .collect()
}

/// Registered providers that support non-interactive token auth.
///
/// The `token_auth` middleware seam consults these (and only these) when a
/// token-authable route is hit, so OAuth/password-only providers are never
/// asked to `verify_token`. Returns an empty list when no token provider
/// is registered — a token-authable route then fails closed (401), never
/// open.
///
/// PARITY: `list_token_providers` (upstream lines 56-68).
pub fn list_token_providers() -> Vec<Arc<dyn DashboardAuthProvider>> {
    let state = registry().lock().unwrap_or_else(|e| e.into_inner());
    state
        .order
        .iter()
        .filter_map(|name| state.providers.get(name).cloned())
        .filter(|p| p.supports_token())
        .collect()
}

/// Registered providers with `supports_session` true (interactive cookie
/// sessions) — the login page, /auth/login, and the gate's verify/refresh
/// loops consult only these. Mirror of [`list_token_providers`].
///
/// PARITY: `list_session_providers` (upstream lines 71-79).
pub fn list_session_providers() -> Vec<Arc<dyn DashboardAuthProvider>> {
    let state = registry().lock().unwrap_or_else(|e| e.into_inner());
    state
        .order
        .iter()
        .filter_map(|name| state.providers.get(name).cloned())
        .filter(|p| p.supports_session())
        .collect()
}

/// Test-only: drop all registrations.
///
/// PARITY: `clear_providers` (upstream lines 82-85).
pub fn clear_providers() {
    let mut state = registry().lock().unwrap_or_else(|e| e.into_inner());
    state.order.clear();
    state.providers.clear();
}
