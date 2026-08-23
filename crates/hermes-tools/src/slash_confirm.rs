//! Generic slash-command confirmation primitive (gateway-side).
//!
//! PARITY: tools/slash_confirm.py @ b9aa928 (167 LOC, ported 1:1).
//!
//! Slash commands that have a non-destructive but expensive side effect
//! (currently only `/reload-mcp`) register a pending confirmation keyed by
//! gateway session key; adapters resolve it later with `once` / `always` /
//! `cancel`.  State is module-level (like upstream `tools.approval`) so
//! platform adapters can resolve callbacks without a backreference to the
//! gateway instance.
//!
//! PORT SEAMS (documented divergences):
//! - Upstream handlers are `async def` callables and `resolve` is an async
//!   function awaited from an asyncio loop.  This crate has no async
//!   runtime, so handlers are boxed `Fn(String) -> Pin<Box<dyn Future>>`
//!   and `resolve` is a *blocking* function that drives the handler future
//!   to completion with a minimal no-op-waker executor.  The observable
//!   contract is identical: the entry is popped before the handler runs,
//!   concurrent resolves run the handler at most once, stale/mismatched
//!   confirms resolve to `None`.
//! - `resolve_sync_compat`'s `loop` parameter (an asyncio event loop) has
//!   no Rust equivalent; its scheduling helper
//!   (`agent.async_utils.safe_schedule_threadsafe`) lives in the unported
//!   agent/ surface, so the function is a thin wrapper over [`resolve`]
//!   with the same fallback-to-None contract.
//! - The upstream `_pending` dict values are plain dicts; here they are a
//!   typed [`PendingConfirm`].  [`get_pending`] returns a defensive copy
//!   (cloned fields / cloned handler Arc) exactly like upstream's
//!   `dict(entry)` shallow copy.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;

/// Default timeout — a pending confirm older than this is discarded when
/// the next message arrives for the same session.
pub const DEFAULT_TIMEOUT_SECONDS: f64 = 300.0;

/// A pending confirmation's async handler: takes the user's choice
/// (`"once"`, `"always"`, `"cancel"`) and returns an optional follow-up
/// message string.
///
/// PARITY: slash_confirm.py `handler: Callable[[str], Awaitable[Optional[str]]]`.
pub type SlashConfirmHandler =
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Option<String>> + Send>> + Send + Sync;

/// A registered pending confirmation (the upstream `_pending[session_key]`
/// dict, typed).
#[derive(Clone)]
pub struct PendingConfirm {
    pub confirm_id: String,
    /// e.g. "reload-mcp".
    pub command: String,
    pub handler: Arc<SlashConfirmHandler>,
    pub created_at: f64,
}

/// Pending confirmations keyed by gateway `session_key`.
static PENDING: Lazy<Mutex<HashMap<String, Arc<PendingConfirm>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Register a pending slash-command confirmation.
///
/// Overwrites any prior pending confirm for the same `session_key` — the
/// user invoking a new confirmable command supersedes the stale one.
pub fn register(
    session_key: &str,
    confirm_id: &str,
    command: &str,
    handler: Arc<SlashConfirmHandler>,
) {
    PENDING.lock().expect("slash_confirm lock").insert(
        session_key.to_string(),
        Arc::new(PendingConfirm {
            confirm_id: confirm_id.to_string(),
            command: command.to_string(),
            handler,
            created_at: now(),
        }),
    );
}

/// Return the pending confirm for a session, or `None`.
///
/// Returns a defensive copy (upstream `dict(entry)`), so mutating the
/// returned struct does not affect the stored entry.
pub fn get_pending(session_key: &str) -> Option<PendingConfirm> {
    PENDING
        .lock()
        .expect("slash_confirm lock")
        .get(session_key)
        .map(|entry| PendingConfirm {
            confirm_id: entry.confirm_id.clone(),
            command: entry.command.clone(),
            handler: entry.handler.clone(),
            created_at: entry.created_at,
        })
}

/// Drop the pending confirm for `session_key` without running it.
pub fn clear(session_key: &str) {
    PENDING
        .lock()
        .expect("slash_confirm lock")
        .remove(session_key);
}

/// Drop the pending confirm if older than `timeout` seconds.
///
/// Returns `true` if an entry was dropped.
pub fn clear_if_stale(session_key: &str, timeout: f64) -> bool {
    let mut pending = PENDING.lock().expect("slash_confirm lock");
    let stale = match pending.get(session_key) {
        None => false,
        Some(entry) => now() - (entry.created_at.max(0.0)) > timeout,
    };
    if stale {
        pending.remove(session_key);
    }
    stale
}

/// Minimal no-op-waker executor that drives a handler future to completion.
///
/// Upstream's event loop awaits the handler; this crate has no async
/// runtime, and the handler futures installed by adapters resolve on their
/// first poll (no real IO awaits), so a spin-with-yield loop is safe here.
fn block_on<F: Future>(fut: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| RawWaker::new(std::ptr::null(), &VTABLE),
        |_| {},
        |_| {},
        |_| {},
    );
    // SAFETY: the waker's data pointer is null and the vtable methods are
    // no-ops that never dereference it.
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = Box::pin(fut);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(out) => return out,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// Best-effort panic message extraction (Python `str(exc)` equivalent for
/// handler panics).
fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown error".to_string()
    }
}

/// Resolve a pending confirm.
///
/// `choice` must be one of `"once"`, `"always"`, or `"cancel"`.  Returns
/// the handler's output string (to be sent as a follow-up message), or
/// `None` if the confirm was stale, already resolved, or the `confirm_id`
/// doesn't match.
///
/// Safe to call from a callback (button click) or from the gateway's
/// message-intercept path.  The entry is popped before the handler runs so
/// a duplicate callback (e.g. button double-click) cannot run it twice.
///
/// PARITY: slash_confirm.py `resolve` @ b9aa928 (async → blocking port).
pub fn resolve(session_key: &str, confirm_id: &str, choice: &str, timeout: f64) -> Option<String> {
    let (handler, command) = {
        let mut pending = PENDING.lock().expect("slash_confirm lock");
        let entry = pending.get(session_key)?;
        if entry.confirm_id != confirm_id {
            // Stale confirm_id — superseded by a newer prompt on the same
            // session.
            return None;
        }
        // Pop before running the handler to prevent duplicate callbacks
        // from running it twice.
        let entry = pending.remove(session_key)?;
        if now() - entry.created_at.max(0.0) > timeout {
            return None;
        }
        (entry.handler.clone(), entry.command.clone())
    };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        block_on(handler(choice.to_string()))
    }));
    match result {
        Ok(output) => output,
        Err(panic) => {
            let message = panic_message(panic);
            log::error!("Slash-confirm handler for /{} raised: {}", command, message);
            Some(format!("❌ Error handling confirmation: {message}"))
        }
    }
}

/// Synchronous helper: resolve a pending confirm and wait for the result.
///
/// Used by platform callback paths that run on a different thread than the
/// event loop (e.g. Discord's button click handler in some
/// configurations).  Prefer [`resolve`] from an async-capable context.
///
/// PARITY: slash_confirm.py `resolve_sync_compat` @ b9aa928.  The upstream
/// `loop` parameter and `safe_schedule_threadsafe` scheduling seam belong
/// to the unported agent/ async layer; in this crate the helper is a thin
/// wrapper over [`resolve`] with the same "any scheduling failure → None"
/// fallback contract.
pub fn resolve_sync_compat(session_key: &str, confirm_id: &str, choice: &str) -> Option<String> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        resolve(session_key, confirm_id, choice, DEFAULT_TIMEOUT_SECONDS)
    }));
    match result {
        Ok(Some(out)) => Some(out),
        Ok(None) => None,
        Err(_) => {
            log::error!("resolve_sync_compat failed");
            None
        }
    }
}

/// Test seam: rewrite a pending entry's `created_at` (upstream tests reach
/// into `slash_confirm._pending["sess"]["created_at"]` directly).
#[doc(hidden)]
pub fn set_pending_created_at_for_test(session_key: &str, created_at: f64) {
    if let Some(entry) = PENDING
        .lock()
        .expect("slash_confirm lock")
        .get_mut(session_key)
    {
        Arc::make_mut(entry).created_at = created_at;
    }
}
