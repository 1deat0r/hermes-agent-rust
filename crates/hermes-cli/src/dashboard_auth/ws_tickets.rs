//! WS-upgrade auth credentials for gated mode.
//!
//! PARITY: `hermes_cli/dashboard_auth/ws_tickets.py` @ 5d59366 (whole
//! module, 97 lines). `mint_ticket` / `consume_ticket` (lines 36-59),
//! `_gc_expired_locked` (lines 62-66), `internal_ws_credential` /
//! `consume_internal_credential` (lines 69-89), `TicketInvalid` (lines
//! 32-33), `TTL_SECONDS` / `INTERNAL_*` (lines 21/27-29),
//! `_reset_for_tests` (lines 92-97).
//!
//! Intentional divergence: the unknown-ticket truncation counts chars
//! where upstream slices bytes (`ticket[:8]`). `token_urlsafe` output
//! is always ASCII so the shapes coincide on every reachable input,
//! and the char form can never panic on a non-boundary.
//!
//! Browsers cannot set `Authorization` on a WebSocket upgrade. In loopback
//! mode the legacy `?token=` query param works because the token is
//! injected into the SPA bundle. In gated mode there is no injected token —
//! so this module provides two credential shapes:
//!
//! 1. **Single-use browser tickets** ([`mint_ticket`] /
//!    [`consume_ticket`]). The SPA gets a fresh ticket via the
//!    authenticated REST endpoint `POST /api/auth/ws-ticket` and passes it
//!    as `?ticket=` on the WS upgrade. Single-use, TTL = 30 seconds — a
//!    leaked ticket is uninteresting.
//!
//! 2. **A process-lifetime internal credential**
//!    ([`internal_ws_credential`] / [`consume_internal_credential`]). This
//!    authenticates *server-spawned* WS clients — specifically the
//!    embedded-TUI PTY child, which attaches to `/api/ws` and `/api/pub`
//!    over loopback. The child reads its attach URL once at startup and
//!    reuses it on every reconnect, and on a slow cold boot may not dial
//!    within 30s — so a single-use ticket is the wrong shape. The internal
//!    credential is minted once per process, never expires, is multi-use,
//!    and — critically — is **never injected into any HTML/SPA**: it only
//!    ever leaves the process via the spawned child's environment, so
//!    browser-side XSS cannot read it. A leaked internal credential grants
//!    no more than a single-use ticket already does (the same two internal
//!    WS endpoints), and the same Origin / host guards still apply
//!    downstream.
//!
//! In-memory; the dashboard is a single process so no distributed
//! coordination is needed.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64_URL;
use base64::Engine;
use once_cell::sync::Lazy;
use serde_json::Value;

/// Time-to-live for newly-minted tickets in seconds. 30 s is long enough
/// that the SPA can call `getWsTicket()` and immediately open the WS,
/// short enough that a leaked ticket is uninteresting.
///
/// PARITY: `TTL_SECONDS` (upstream line 21).
pub const TTL_SECONDS: i64 = 30;

/// Identity recorded for connections that authenticate via the internal
/// credential, so audit logs distinguish them from browser-initiated
/// tickets.
///
/// PARITY: `INTERNAL_USER_ID` / `INTERNAL_PROVIDER` (upstream lines 27-29).
pub const INTERNAL_USER_ID: &str = "server-internal";
pub const INTERNAL_PROVIDER: &str = "server-internal";

/// Identity `info` returned on consume (`user_id` / `provider` / minted_at
/// for tickets), mirroring the upstream dict shape.
pub type TicketInfo = HashMap<String, Value>;

/// PARITY: `TicketInvalid` — ticket missing, expired, or already consumed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TicketInvalid {
    #[error("unknown ticket: {0}")]
    UnknownTicket(String),
    #[error("expired")]
    Expired,
    #[error("no internal credential")]
    NoInternalCredential,
    #[error("internal credential mismatch")]
    InternalCredentialMismatch,
}

#[derive(Default)]
struct State {
    /// ticket → (expires_at, info).
    tickets: HashMap<String, (i64, TicketInfo)>,
    internal_credential: Option<String>,
}

static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::default()));

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `secrets.token_urlsafe(32)` — base64url of 32 random bytes (43 chars).
fn token_urlsafe_32() -> String {
    use base64::Engine;
    let bytes: [u8; 32] = rand::random();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Generate a one-shot ticket bound to this user identity.
///
/// The info dict is stored for the consumer so the WS handler can carry
/// the identity forward into its session log.
///
/// PARITY: `mint_ticket` (upstream lines 36-44).
pub fn mint_ticket(user_id: &str, provider: &str) -> String {
    mint_ticket_at(user_id, provider, now_unix())
}

/// Explicit-clock form of [`mint_ticket`] (the upstream tests patch
/// `time.time`).
pub fn mint_ticket_at(user_id: &str, provider: &str, now: i64) -> String {
    let ticket = token_urlsafe_32();
    let mut info = TicketInfo::new();
    info.insert("user_id".to_string(), Value::String(user_id.to_string()));
    info.insert("provider".to_string(), Value::String(provider.to_string()));
    info.insert("minted_at".to_string(), Value::from(now));
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    state
        .tickets
        .insert(ticket.clone(), (now + TTL_SECONDS, info));
    gc_expired_locked(&mut state, now);
    ticket
}

/// Validate and consume. Errors on missing/expired/used.
///
/// Single-use semantics: a successful consume immediately removes the
/// ticket from the store, so a second call with the same value errors as
/// `UnknownTicket` with the value truncated (misuse never logs the secret
/// in full).
///
/// PARITY: `consume_ticket` (upstream lines 47-59).
pub fn consume_ticket(ticket: &str) -> Result<TicketInfo, TicketInvalid> {
    consume_ticket_at(ticket, now_unix())
}

/// Explicit-clock form of [`consume_ticket`] (the upstream tests patch
/// `time.time`).
pub fn consume_ticket_at(ticket: &str, now: i64) -> Result<TicketInfo, TicketInvalid> {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    match state.tickets.remove(ticket) {
        None => {
            let truncated = if ticket.is_empty() {
                "<empty>".to_string()
            } else {
                format!("{}\u{2026}", &ticket[..ticket.chars().count().min(8)])
            };
            Err(TicketInvalid::UnknownTicket(truncated))
        }
        Some((expires_at, info)) => {
            if expires_at < now {
                Err(TicketInvalid::Expired)
            } else {
                Ok(info)
            }
        }
    }
}

/// Drop expired tickets. Caller holds the lock.
///
/// PARITY: `_gc_expired_locked` (upstream lines 62-66).
fn gc_expired_locked(state: &mut State, now: i64) {
    let expired: Vec<String> = state
        .tickets
        .iter()
        .filter(|(_, (exp, _))| *exp < now)
        .map(|(t, _)| t.clone())
        .collect();
    for t in expired {
        state.tickets.remove(&t);
    }
}

/// Return the process-lifetime internal WS credential, minting it once.
///
/// Stable for the life of the process, multi-use, never expires — so a
/// server-spawned child can reconnect its `/api/ws` / `/api/pub` sockets
/// indefinitely. Never injected into the SPA HTML or returned over any
/// REST endpoint; only ever passed to a child process via its environment.
///
/// PARITY: `internal_ws_credential` (upstream lines 69-76).
pub fn internal_ws_credential() -> String {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    state
        .internal_credential
        .get_or_insert_with(token_urlsafe_32)
        .clone()
}

/// Validate an internal credential. Errors on mismatch.
///
/// Unlike [`consume_ticket`] this is **not** single-use — the value is not
/// removed on success. A constant-time compare avoids leaking
/// length/prefix information on mismatch. If no internal credential has
/// been minted yet, any value is rejected.
///
/// PARITY: `consume_internal_credential` (upstream lines 79-89).
pub fn consume_internal_credential(value: &str) -> Result<TicketInfo, TicketInvalid> {
    let expected = STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .internal_credential
        .clone();
    if value.is_empty() || expected.is_none() {
        return Err(TicketInvalid::NoInternalCredential);
    }
    let expected = expected.unwrap();
    if !constant_time_eq(value.as_bytes(), expected.as_bytes()) {
        return Err(TicketInvalid::InternalCredentialMismatch);
    }
    let mut info = TicketInfo::new();
    info.insert(
        "user_id".to_string(),
        Value::String(INTERNAL_USER_ID.to_string()),
    );
    info.insert(
        "provider".to_string(),
        Value::String(INTERNAL_PROVIDER.to_string()),
    );
    Ok(info)
}

/// `secrets.compare_digest` equivalent.
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

/// Test-only: drop all tickets and the internal credential.
///
/// PARITY: `_reset_for_tests` (upstream lines 92-97).
pub fn reset_for_tests() {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    state.tickets.clear();
    state.internal_credential = None;
}
