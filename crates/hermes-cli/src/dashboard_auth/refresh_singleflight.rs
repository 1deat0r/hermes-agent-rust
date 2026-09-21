//! Single-flight + short replay cache for rotating refresh tokens.
//!
//! PARITY: `hermes_cli/dashboard_auth/refresh_singleflight.py` @ 5d59366
//! (whole module, 107 lines). Rotating refresh tokens with reuse
//! detection make replaying an already-rotated RT fatal, so parallel
//! bursts (desktop + browser on wake) must let exactly ONE request reach
//! the provider and hand the rotated session to the rest.
//!
//! Keying: the flight/cache key is `(provider identity, sha256(token))` —
//! a provider hint only orders discovery, it must neither split one
//! rotating credential's lock nor let an unrelated provider reuse its
//! result. Raw refresh tokens are never keys. Provider identity is the
//! `Arc` pointer (replacement, including same-name scoped
//! registrations, invalidates identity — upstream holds the provider
//! object so Python cannot recycle its `id` under a live entry; here
//! the `Arc` clone in the cache entry owns the same lifetime).
//!
//! Only successes (`_SUCCESS_TTL`) and dead-token failures
//! (`_FAILURE_TTL`) are cached; `ProviderError` is deliberately NOT
//! cached. A panicking provider is caught and treated as unreachable
//! (rather than crashing the scan, which is what an uncaught exception
//! does upstream) — consistent with the `catch_unwind` house rule in
//! `token_auth.rs`: a buggy provider must never 500 the gate.
//! Synchronous and network-bound: async callers run it in a threadpool
//! upstream; here callers pass a sync closure (the provider's blocking
//! refresh path).

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::base::{DashboardAuthProvider, ProviderError, Session};
use super::request_utils::{scan_session_providers, ScanOutcome};

/// Success TTL: covers the window between the winning response and the
/// siblings' arrival (a waking laptop can spread its burst over many
/// seconds).
///
/// PARITY: `_SUCCESS_TTL` (upstream line 28).
pub const SUCCESS_TTL: Duration = Duration::from_secs(30);
/// Failure TTL: only absorbs a retry storm against a token the provider
/// has already declared dead.
///
/// PARITY: `_FAILURE_TTL` (upstream line 29).
pub const FAILURE_TTL: Duration = Duration::from_secs(5);
/// Global cap on cache entries.
///
/// PARITY: `_MAX_ENTRIES` (upstream line 30).
pub const MAX_ENTRIES: usize = 256;

type ProviderId = usize;
type TokenDigest = [u8; 32];

#[derive(Clone)]
struct CacheEntry {
    expires: Instant,
    // The provider itself: replacement (including same-name scoped
    // registrations) invalidates identity. Holding one `Arc` clone pins
    // the allocation, so a fresh provider can never recycle the address
    // behind a live cache key (upstream's `id()` rationale).
    #[allow(dead_code)]
    provider: Arc<dyn DashboardAuthProvider>,
    /// `None` = the provider declared this token dead.
    session: Option<Session>,
}

#[derive(Default)]
struct Flight {
    /// Set while one caller owns the provider call; joiners wait on it.
    owned: bool,
}

struct State {
    cache: HashMap<(ProviderId, TokenDigest), CacheEntry>,
    flights: HashMap<(ProviderId, TokenDigest), Arc<(Mutex<Flight>, Condvar)>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            cache: HashMap::new(),
            flights: HashMap::new(),
        }
    }
}

static STATE: OnceLock<Mutex<State>> = OnceLock::new();

fn state() -> &'static Mutex<State> {
    STATE.get_or_init(|| Mutex::new(State::default()))
}

fn provider_id(provider: &Arc<dyn DashboardAuthProvider>) -> ProviderId {
    Arc::as_ptr(provider) as *const () as usize
}

fn token_digest(token: &str) -> TokenDigest {
    Sha256::digest(token.as_bytes()).into()
}

/// Drop expired entries, then evict earliest-expiry first past the cap.
///
/// PARITY: `_prune` (upstream lines 46-50).
fn prune_locked(state: &mut State, now: Instant) {
    state.cache.retain(|_, entry| entry.expires > now);
    while state.cache.len() > MAX_ENTRIES {
        let oldest = state
            .cache
            .iter()
            .min_by_key(|(_, entry)| entry.expires)
            .map(|(key, _)| key.clone());
        match oldest {
            Some(key) => {
                state.cache.remove(&key);
            }
            None => break,
        }
    }
}

/// Errors from the blocking provider refresh call.
pub enum RefreshCallError {
    /// The provider declared the token dead (cached briefly).
    Expired,
    /// Backing store unreachable (never cached).
    Unreachable(ProviderError),
    /// A panicking provider (never cached; the flight still completes).
    Panic,
}

/// Refresh one provider under the single-flight for `(provider, token)`.
///
/// Returns the session (`None` = dead token). Concurrent callers for the
/// same key block on the flight and share the winner's result.
///
/// PARITY: `_refresh_provider` (upstream lines 54-83).
pub fn refresh_provider_coalesced(
    provider: &Arc<dyn DashboardAuthProvider>,
    token: &str,
    call: impl FnOnce() -> Result<Session, RefreshCallError>,
) -> Result<Option<Session>, ProviderError> {
    let key = (provider_id(provider), token_digest(token));
    let flight = {
        let mut state = state().lock().unwrap_or_else(|e| e.into_inner());
        prune_locked(&mut state, Instant::now());
        state
            .flights
            .entry(key.clone())
            .or_insert_with(|| Arc::new((Mutex::new(Flight::default()), Condvar::new())))
            .clone()
    };
    let (flight_mutex, flight_cvar) = &*flight;
    // Fast path: a completed flight already cached the answer.
    {
        let state = state().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = state.cache.get(&key) {
            if entry.expires > Instant::now() {
                return Ok(entry.session.clone());
            }
        }
    }
    // Upstream `with flight.lock`: exactly one caller owns the provider
    // call; joiners block here, then read the winner's cache entry.
    let mut flight_guard = flight_mutex.lock().unwrap_or_else(|e| e.into_inner());
    if flight_guard.owned {
        while flight_guard.owned {
            flight_guard = flight_cvar
                .wait(flight_guard)
                .unwrap_or_else(|e| e.into_inner());
        }
        drop(flight_guard);
        let state = state().lock().unwrap_or_else(|e| e.into_inner());
        return match state.cache.get(&key) {
            Some(entry) if entry.expires > Instant::now() => Ok(entry.session.clone()),
            // Owner failed uncached (unreachable/panic): joiners see no
            // answer and fall through to their own scan step.
            _ => Ok(None),
        };
    }
    flight_guard.owned = true;
    drop(flight_guard);
    // Owner: run the provider call outside every lock.
    let outcome: Result<Option<Session>, ProviderError> = match catch_refresh(call) {
        Ok(session) => {
            store_locked(&key, provider, Some(session.clone()), SUCCESS_TTL);
            Ok(Some(session))
        }
        Err(RefreshCallError::Expired) => {
            store_locked(&key, provider, None, FAILURE_TTL);
            Ok(None)
        }
        Err(RefreshCallError::Unreachable(err)) => Err(err),
        Err(RefreshCallError::Panic) => Err(ProviderError(
            "refresh provider panicked; not cached".to_string(),
        )),
    };
    // Wake joiners, then retire the flight table entry.
    {
        let mut state = state().lock().unwrap_or_else(|e| e.into_inner());
        state.flights.remove(&key);
    }
    let mut flight_guard = flight_mutex.lock().unwrap_or_else(|e| e.into_inner());
    flight_guard.owned = false;
    flight_cvar.notify_all();
    outcome
}

fn catch_refresh(
    call: impl FnOnce() -> Result<Session, RefreshCallError>,
) -> Result<Session, RefreshCallError> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)) {
        Ok(result) => result,
        Err(_) => Err(RefreshCallError::Panic),
    }
}

fn store_locked(
    key: &(ProviderId, TokenDigest),
    provider: &Arc<dyn DashboardAuthProvider>,
    session: Option<Session>,
    ttl: Duration,
) {
    let mut state = state().lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    state.cache.insert(
        key.clone(),
        CacheEntry {
            expires: now + ttl,
            provider: Arc::clone(provider),
            session,
        },
    );
    prune_locked(&mut state, now);
}

/// Rotate `token` through the provider stack with per-provider
/// single-flight.
///
/// `(Session, provider_name)` or `None` when every provider rejects the
/// token; `Err(ProviderError)` propagates when nothing rotated and one
/// provider was unreachable (callers keep their 503-not-relogin
/// handling).
///
/// PARITY: `refresh_session_coalesced` (upstream lines 86-107).
/// `on_rejected` / `on_unreachable` fold into the return contract:
/// rejected providers yield `Next` (the caller observes them via
/// `rejected` output), unreachable ones are remembered for the 503.
pub fn refresh_session_coalesced(
    token: &str,
    provider_hint: Option<&str>,
    rejected: &mut Vec<String>,
    refresh: &impl Fn(&Arc<dyn DashboardAuthProvider>, &str) -> Result<Session, RefreshCallError>,
) -> Result<Option<(Session, String)>, ProviderError> {
    scan_session_providers(provider_hint, |provider| {
        match refresh_provider_coalesced(provider, token, || refresh(provider, token)) {
            Ok(Some(session)) => {
                let name = provider.name().to_string();
                ScanOutcome::Done((session, name))
            }
            Ok(None) => {
                rejected.push(provider.name().to_string());
                ScanOutcome::Next
            }
            Err(_) => ScanOutcome::Unreachable,
        }
    })
}

/// Test-only: drop all flights and cache entries.
pub fn reset_for_tests() {
    let mut state = state().lock().unwrap_or_else(|e| e.into_inner());
    state.cache.clear();
    state.flights.clear();
}
