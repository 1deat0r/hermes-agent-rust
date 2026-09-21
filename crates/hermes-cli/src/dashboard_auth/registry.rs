//! Module-level registry for DashboardAuthProvider instances.
//!
//! PARITY: `hermes_cli/dashboard_auth/registry.py` @ 5d59366 (whole
//! module, 125 lines). Registration order is preserved (upstream dict
//! insertion order); the global map plus per-scope overlays merge on
//! read; `register_provider` raises protocol-violation / duplicate;
//! `register_global_provider` upserts in place; `snapshot` / `restore`
//! give the plugin manager identity-conditional teardown.
//!
//! Scope note: upstream defaults `scope=None` to the current
//! `hermes_home_key()` overlay. That helper is not yet ported, so this
//! seam takes `scope: Option<&str>` with `None` = process-global map
//! only; callers pass `Some(home_key)` for the per-home overlay once
//! the helper lands. All merge/restore semantics are otherwise exact.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use super::base::assert_protocol_compliance;

pub use super::base::{
    assert_protocol_compliance as assert_protocol_compliance_exported, DashboardAuthProvider,
    InvalidCodeError, InvalidCredentialsError, LoginStart, ProviderError, RefreshExpiredError,
    Session, TokenPrincipal,
};

#[derive(Default)]
struct RegistryState {
    /// Process-global providers, insertion-ordered.
    order: Vec<String>,
    providers: HashMap<String, Arc<dyn DashboardAuthProvider>>,
    /// Per-scope overlays: scope → (insertion order, providers).
    scoped_order: HashMap<String, Vec<String>>,
    scoped_providers: HashMap<String, HashMap<String, Arc<dyn DashboardAuthProvider>>>,
}

static REGISTRY: OnceLock<Mutex<RegistryState>> = OnceLock::new();

fn registry() -> &'static Mutex<RegistryState> {
    REGISTRY.get_or_init(|| Mutex::new(RegistryState::default()))
}

/// Merged view: global entries, overlaid by `scope`'s entries.
///
/// PARITY: `_merged` (upstream lines 19-22). Overlay entries shadow
/// same-name global entries; order is global-first, then overlay-only
/// names in overlay order.
fn merged_locked(
    state: &RegistryState,
    scope: Option<&str>,
) -> Vec<(String, Arc<dyn DashboardAuthProvider>)> {
    let mut out: Vec<(String, Arc<dyn DashboardAuthProvider>)> = state
        .order
        .iter()
        .filter_map(|name| {
            state
                .providers
                .get(name)
                .map(|p| (name.clone(), Arc::clone(p)))
        })
        .collect();
    if let Some(scope) = scope {
        if let Some(order) = state.scoped_order.get(scope) {
            let overlay = &state.scoped_providers[scope];
            for name in order {
                if let Some(p) = overlay.get(name) {
                    if let Some(slot) = out.iter_mut().find(|(n, _)| n == name) {
                        slot.1 = Arc::clone(p);
                    } else {
                        out.push((name.clone(), Arc::clone(p)));
                    }
                }
            }
        }
    }
    out
}

/// Register a provider, globally or in one scope's overlay.
///
/// Errors: protocol violation ([`assert_protocol_compliance`]) or a
/// duplicate name in the effective view (global map for `None`, merged
/// view for `Some`).
///
/// PARITY: `register_provider` (upstream lines 36-45).
pub fn register_provider(
    provider: Arc<dyn DashboardAuthProvider>,
    scope: Option<&str>,
) -> Result<(), String> {
    assert_protocol_compliance(provider.as_ref())?;
    let name = provider.name().to_string();
    let display_name = provider.display_name().to_string();
    let mut state = registry().lock().unwrap_or_else(|e| e.into_inner());
    let duplicate = match scope {
        None => state.providers.contains_key(&name),
        Some(scope) => merged_locked(&state, Some(scope))
            .iter()
            .any(|(n, _)| *n == name),
    };
    if duplicate {
        return Err(format!(
            "dashboard-auth provider already registered: {name:?}"
        ));
    }
    match scope {
        None => {
            state.order.push(name.clone());
            state.providers.insert(name.clone(), provider);
        }
        Some(scope) => {
            state
                .scoped_order
                .entry(scope.to_string())
                .or_default()
                .push(name.clone());
            state
                .scoped_providers
                .entry(scope.to_string())
                .or_default()
                .insert(name.clone(), provider);
        }
    }
    log::info!(
        "dashboard-auth: registered provider {:?} ({})",
        name,
        display_name
    );
    Ok(())
}

/// Return the registered provider for `name`, or None if unknown.
///
/// PARITY: `get_provider` (upstream lines 48-51).
pub fn get_provider(name: &str, scope: Option<&str>) -> Option<Arc<dyn DashboardAuthProvider>> {
    let state = registry().lock().unwrap_or_else(|e| e.into_inner());
    merged_locked(&state, scope)
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, p)| p)
}

/// Read one scope's own slot without merging (the plugin manager's
/// teardown seam).
///
/// PARITY: `snapshot_registration` (upstream lines 54-57).
pub fn snapshot_registration(
    name: &str,
    scope: Option<&str>,
) -> Option<Arc<dyn DashboardAuthProvider>> {
    let state = registry().lock().unwrap_or_else(|e| e.into_inner());
    match scope {
        None => state.providers.get(name).cloned(),
        Some(scope) => state
            .scoped_providers
            .get(scope)
            .and_then(|overlay| overlay.get(name).cloned()),
    }
}

/// Restore a host-owned registration if it is still current.
///
/// Identity-conditional: returns false (no-op) when the slot no longer
/// holds `current`. Prunes the overlay when it becomes empty.
///
/// PARITY: `restore_registration` (upstream lines 60-74).
pub fn restore_registration(
    name: &str,
    current: &Arc<dyn DashboardAuthProvider>,
    previous: Option<Arc<dyn DashboardAuthProvider>>,
    scope: Option<&str>,
) -> bool {
    let mut state = registry().lock().unwrap_or_else(|e| e.into_inner());
    let is_current = match scope {
        None => state
            .providers
            .get(name)
            .is_some_and(|p| Arc::ptr_eq(p, current)),
        Some(scope) => state
            .scoped_providers
            .get(scope)
            .and_then(|overlay| overlay.get(name))
            .is_some_and(|p| Arc::ptr_eq(p, current)),
    };
    if !is_current {
        return false;
    }
    match scope {
        None => match previous {
            None => {
                state.providers.remove(name);
                state.order.retain(|n| n != name);
            }
            Some(previous) => {
                state.providers.insert(name.to_string(), previous);
            }
        },
        Some(scope) => {
            let remove_overlay = match previous {
                None => {
                    if let Some(overlay) = state.scoped_providers.get_mut(scope) {
                        overlay.remove(name);
                    }
                    if let Some(order) = state.scoped_order.get_mut(scope) {
                        order.retain(|n| n != name);
                    }
                    state
                        .scoped_providers
                        .get(scope)
                        .is_some_and(|overlay| overlay.is_empty())
                }
                Some(previous) => {
                    if let Some(overlay) = state.scoped_providers.get_mut(scope) {
                        overlay.insert(name.to_string(), previous);
                    }
                    false
                }
            };
            if remove_overlay {
                state.scoped_providers.remove(scope);
                state.scoped_order.remove(scope);
            }
        }
    }
    true
}

/// All registered providers in the effective view, registration order.
///
/// PARITY: `list_providers` (upstream lines 77-80).
pub fn list_providers(scope: Option<&str>) -> Vec<Arc<dyn DashboardAuthProvider>> {
    let state = registry().lock().unwrap_or_else(|e| e.into_inner());
    merged_locked(&state, scope)
        .into_iter()
        .map(|(_, p)| p)
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
/// PARITY: `list_token_providers` (upstream lines 83-87).
pub fn list_token_providers() -> Vec<Arc<dyn DashboardAuthProvider>> {
    list_providers(None)
        .into_iter()
        .filter(|p| p.supports_token())
        .collect()
}

/// Registered providers with `supports_session` true (interactive cookie
/// sessions) — the login page, /auth/login, and the gate's verify/refresh
/// loops consult only these. Mirror of [`list_token_providers`].
///
/// PARITY: `list_session_providers` (upstream lines 90-93).
pub fn list_session_providers() -> Vec<Arc<dyn DashboardAuthProvider>> {
    list_providers(None)
        .into_iter()
        .filter(|p| p.supports_session())
        .collect()
}

/// Register a host-owned provider in the process-global slot (upsert).
///
/// Always targets the global map (never an overlay) and *replaces* a
/// same-name entry instead of raising, so a forced plugin re-discovery
/// rotates the provider in place.
///
/// PARITY: `register_global_provider` (upstream lines 96-108).
pub fn register_global_provider(provider: Arc<dyn DashboardAuthProvider>) -> Result<(), String> {
    assert_protocol_compliance(provider.as_ref())?;
    let name = provider.name().to_string();
    let display_name = provider.display_name().to_string();
    let mut state = registry().lock().unwrap_or_else(|e| e.into_inner());
    if !state.providers.contains_key(&name) {
        state.order.push(name.clone());
    }
    state.providers.insert(name.clone(), provider);
    log::info!(
        "dashboard-auth: registered global provider {:?} ({})",
        name,
        display_name
    );
    Ok(())
}

/// Remove a global registration if `provider` is still current.
///
/// A stale handle whose provider was already replaced never clears the
/// live one.
///
/// PARITY: `unregister_global_provider` (upstream lines 111-118).
pub fn unregister_global_provider(name: &str, provider: &Arc<dyn DashboardAuthProvider>) -> bool {
    let mut state = registry().lock().unwrap_or_else(|e| e.into_inner());
    let is_current = state
        .providers
        .get(name)
        .is_some_and(|p| Arc::ptr_eq(p, provider));
    if !is_current {
        return false;
    }
    state.providers.remove(name);
    state.order.retain(|n| n != name);
    true
}

/// Test-only: drop all registrations.
///
/// PARITY: `clear_providers` (upstream lines 121-125).
pub fn clear_providers() {
    let mut state = registry().lock().unwrap_or_else(|e| e.into_inner());
    state.order.clear();
    state.providers.clear();
    state.scoped_order.clear();
    state.scoped_providers.clear();
}
