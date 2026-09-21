//! Gateway-brokered RFC 8252 (OAuth 2.0 for Native Apps) authorization
//! store.
//!
//! PARITY: `hermes_cli/dashboard_auth/native_flow.py` @ 5d59366 (whole
//! module, 165 lines).
//!
//! The desktop app is a *native* OAuth client that signs in to a gated
//! gateway without an embedded webview or browser cookies. It cannot be a
//! direct OAuth client of the upstream IDP (the Portal `client_id` is
//! per-gateway-instance and validates the redirect against the gateway's
//! public origin), so the **gateway brokers** the flow: authorization
//! server *to the desktop*, OAuth client *to the Portal*.
//!
//! Security properties guaranteed here:
//! * **PKCE binding (RFC 7636).** A gateway code is redeemable only by the
//!   client presenting the matching `code_challenge`; an intercepted
//!   `gw_code` cannot be exchanged without `cv_d`.
//! * **Single use.** `redeem_code` pops the entry before the PKCE check —
//!   a replay (valid or not) finds nothing, and a wrong verifier cannot be
//!   retried against the same code.
//! * **Short TTLs.** Pending 600s (the interactive login window), codes
//!   120s (sub-second loopback round trip). Expired entries are refused
//!   and GC'd.
//! * **Opaque, high-entropy handles** — 256-bit token_urlsafe values;
//!   comparison is constant-time.
//! * **No secret logging.** Tokens live in memory only between callback
//!   and redemption.
//!
//! All functions take an explicit `now` (upstream defaults to
//! `time.time()`), keeping the clock patchable in tests.

use std::collections::HashMap;
use std::sync::Mutex;

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64_URL;
use base64::Engine;
use once_cell::sync::Lazy;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::base::Session;

/// Pending-authorization TTL: the whole interactive login window (mirrors
/// the PKCE cookie lifetime).
/// PARITY: `_PENDING_TTL_SECONDS` (upstream line 28).
pub const PENDING_TTL_SECONDS: i64 = 600;

/// Minted-code TTL: the loopback redirect + the desktop's immediate token
/// POST.
/// PARITY: `_CODE_TTL_SECONDS` (upstream line 29).
pub const CODE_TTL_SECONDS: i64 = 120;

/// Cap on concurrent pending + issued entries (fail closed on a spamming
/// client).
/// PARITY: `_MAX_ENTRIES` (upstream line 30).
pub const MAX_ENTRIES: usize = 256;

/// Per-IP cap on concurrent PENDING authorizations — /auth/native/authorize
/// is public/pre-auth, so one spammer must not fill the global store.
/// PARITY: `_MAX_PENDING_PER_IP` (upstream line 33).
pub const MAX_PENDING_PER_IP: usize = 8;

/// Base for native-flow failures (bad/expired/replayed handle, PKCE fail).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NativeFlowError {
    #[error("native-flow authorization store at capacity")]
    AtCapacity,
    #[error("too many pending native authorizations from this address")]
    TooManyPendingForAddress,
    #[error("unknown or expired native authorization")]
    PendingNotFound,
    #[error("unknown, expired, or already-redeemed code")]
    CodeInvalid,
    #[error("code expired")]
    CodeExpired,
    #[error("PKCE verification failed")]
    PkceFailed,
}

/// An in-flight native authorization awaiting the upstream callback.
///
/// PARITY: `_Pending` (upstream lines 38-46).
#[derive(Debug, Clone)]
pub struct Pending {
    /// The DESKTOP's S256 challenge (cc_d), base64url no-pad.
    pub code_challenge: String,
    /// The desktop's loopback redirect (127.0.0.1:<port>/...).
    pub redirect_uri: String,
    /// The desktop's own `state` (echoed back on redirect).
    pub client_state: String,
    /// Requester IP at authorize time (per-IP pending cap).
    pub client_ip: String,
    pub expires_at: i64,
}

/// A minted one-time gateway authorization code bound to a Session.
///
/// PARITY: `_IssuedCode` (upstream lines 48-53).
#[derive(Debug, Clone)]
pub struct IssuedCode {
    pub code_challenge: String,
    pub session: Session,
    pub expires_at: i64,
}

#[derive(Default)]
struct State {
    pending: HashMap<String, Pending>,
    issued: HashMap<String, IssuedCode>,
}

static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::default()));

fn token_urlsafe_32() -> String {
    let bytes: [u8; 32] = rand::random();
    // rand 0.10 fills arrays via Random distribution

    B64_URL.encode(bytes)
}

/// Base64url without `=` padding (RFC 7636 §4).
///
/// PARITY: `_b64url_no_pad` (upstream lines 72-75).
fn b64url_no_pad(raw: &[u8]) -> String {
    B64_URL.encode(raw)
}

/// RFC 7636 S256 transform: base64url(sha256(ascii(verifier))).
///
/// PARITY: `_s256` (upstream lines 72-75). Upstream
/// `verifier.encode("ascii")` raises on non-ASCII; verifiers are RFC
/// 7636 `[A-Za-z0-9-._~]` so that path is unreachable — UTF-8 bytes
/// hash identically on every reachable input.
pub fn s256(verifier: &str) -> String {
    b64url_no_pad(&Sha256::digest(verifier.as_bytes()))
}

/// Drop expired pending + issued entries. Caller holds the lock.
///
/// PARITY: `_gc_locked` (upstream lines 78-86).
fn gc_locked(state: &mut State, now: i64) {
    let expired_p: Vec<String> = state
        .pending
        .iter()
        .filter(|(_, v)| v.expires_at < now)
        .map(|(k, _)| k.clone())
        .collect();
    for k in expired_p {
        state.pending.remove(&k);
    }
    let expired_c: Vec<String> = state
        .issued
        .iter()
        .filter(|(_, v)| v.expires_at < now)
        .map(|(k, _)| k.clone())
        .collect();
    for k in expired_c {
        state.issued.remove(&k);
    }
}

/// Stash a pending native authorization; return an opaque `broker_state`.
///
/// Called by `/auth/native/authorize`. Errors (fail closed — this is a
/// public pre-auth route): [`NativeFlowError::AtCapacity`] when the store
/// is full, [`NativeFlowError::TooManyPendingForAddress`] when the caller's
/// IP already holds [`MAX_PENDING_PER_IP`] live pending entries.
///
/// PARITY: `register_pending` (upstream lines 101-119).
pub fn register_pending(
    code_challenge: &str,
    redirect_uri: &str,
    client_state: &str,
    client_ip: &str,
    now: i64,
) -> Result<String, NativeFlowError> {
    let broker_state = token_urlsafe_32();
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    gc_locked(&mut state, now);
    if state.pending.len() + state.issued.len() >= MAX_ENTRIES as usize {
        return Err(NativeFlowError::AtCapacity);
    }
    if !client_ip.is_empty()
        && state
            .pending
            .values()
            .filter(|v| v.client_ip == client_ip)
            .count()
            >= MAX_PENDING_PER_IP as usize
    {
        return Err(NativeFlowError::TooManyPendingForAddress);
    }
    state.pending.insert(
        broker_state.clone(),
        Pending {
            code_challenge: code_challenge.to_string(),
            redirect_uri: redirect_uri.to_string(),
            client_state: client_state.to_string(),
            client_ip: client_ip.to_string(),
            expires_at: now + PENDING_TTL_SECONDS,
        },
    );
    Ok(broker_state)
}

/// Return the pending authorization for `broker_state` without consuming
/// it — the callback's read-only peek to learn the desktop's
/// `redirect_uri` and `client_state` for the final 302.
///
/// PARITY: `get_pending` (upstream lines 122-126).
pub fn get_pending(broker_state: &str, now: i64) -> Result<Pending, NativeFlowError> {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    gc_locked(&mut state, now);
    state
        .pending
        .get(broker_state)
        .cloned()
        .ok_or(NativeFlowError::PendingNotFound)
}

/// Consume a pending authorization and mint a one-time gateway code.
///
/// Called by `/auth/callback` once the upstream Session is verified. Pops
/// the pending entry (single use) and binds a fresh `gw_code` to the
/// desktop's `code_challenge` + the verified session.
///
/// PARITY: `complete_pending` (upstream lines 129-142).
pub fn complete_pending(
    broker_state: &str,
    session: &Session,
    now: i64,
) -> Result<String, NativeFlowError> {
    let gw_code = token_urlsafe_32();
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    gc_locked(&mut state, now);
    let pending = state.pending.remove(broker_state);
    let Some(pending) = pending else {
        return Err(NativeFlowError::PendingNotFound);
    };
    if state.pending.len() + state.issued.len() >= MAX_ENTRIES as usize {
        return Err(NativeFlowError::AtCapacity);
    }
    state.issued.insert(
        gw_code.clone(),
        IssuedCode {
            code_challenge: pending.code_challenge,
            session: session.clone(),
            expires_at: now + CODE_TTL_SECONDS,
        },
    );
    Ok(gw_code)
}

/// Verify PKCE + consume a gateway code; return the bound Session.
///
/// The entry is popped BEFORE the PKCE check so a wrong verifier cannot be
/// retried against the same code — on any failure the code is already
/// consumed (no oracle, no replay).
///
/// PARITY: `redeem_code` (upstream lines 145-158).
pub fn redeem_code(code: &str, code_verifier: &str, now: i64) -> Result<Session, NativeFlowError> {
    let issued = {
        let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
        gc_locked(&mut state, now);
        state.issued.remove(code)
    };
    let Some(issued) = issued else {
        return Err(NativeFlowError::CodeInvalid);
    };
    if issued.expires_at < now {
        return Err(NativeFlowError::CodeExpired);
    }
    let expected = issued.code_challenge;
    let actual = s256(code_verifier);
    if !constant_time_eq(expected.as_bytes(), actual.as_bytes()) {
        return Err(NativeFlowError::PkceFailed);
    }
    Ok(issued.session)
}

/// `hmac.compare_digest` equivalent.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Test-only: drop all pending + issued state.
///
/// PARITY: `_reset_for_tests` (upstream lines 161-165).
pub fn reset_for_tests() {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    state.pending.clear();
    state.issued.clear();
}

/// JSON projection of a pending entry (tests / operator debugging).
pub fn pending_to_json(pending: &Pending) -> Value {
    serde_json::json!({
        "code_challenge": pending.code_challenge,
        "redirect_uri": pending.redirect_uri,
        "client_state": pending.client_state,
        "client_ip": pending.client_ip,
        "expires_at": pending.expires_at,
    })
}
