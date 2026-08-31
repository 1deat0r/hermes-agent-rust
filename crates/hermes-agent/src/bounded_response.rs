//! Bounded reads of HTTP error response bodies.
//!
//! PARITY: `agent/bounded_response.py` @ b9aa928 (whole module). Ported
//! and adapted from openclaw/openclaw#95108 ("bound Anthropic error
//! streams"), generalized to cover the streaming error-body sites.
//!
//! A bare `response.read()` on a streaming error body is unbounded in two
//! dangerous ways: a server can declare (or stream) an arbitrarily large
//! body, and a server can open the body and then stall forever. Both are
//! realistic against a misbehaving proxy or a hijacked endpoint. The
//! diagnostic body is only ever shown truncated to a few hundred
//! characters, so reading megabytes — or blocking forever — buys nothing.
//!
//! A wall-clock deadline placed only *between* yielded chunks cannot
//! interrupt a server that stalls mid-chunk (control never returns until
//! the HTTP client's own read timeout fires), so — exactly like upstream —
//! the drain runs on a worker thread and the caller waits with a hard
//! deadline; on timeout the partial bytes collected so far are returned.
//!
//! TRANSLATION NOTE: the httpx response is abstracted as a blocking chunk
//! iterator (`Iterator<Item = Vec<u8>>`); the caller's HTTP layer supplies
//! the iterator and owns closing the response. Never raises: any transport
//! error, stall, or oversize condition is swallowed and the best-effort
//! partial text (or an empty string) is returned — this runs on the error
//! path and must not mask the original HTTP failure.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Comfortably holds any real provider error envelope (Google RPC error
/// JSON, Anthropic error JSON) while rejecting pathological bodies.
///
/// PARITY: `DEFAULT_ERROR_BODY_MAX_BYTES` (upstream line 24).
pub const DEFAULT_ERROR_BODY_MAX_BYTES: usize = 64 * 1024;

/// Hard wall-clock deadline for the whole bounded read. A streaming error
/// body that does not finish within this window is abandoned.
///
/// PARITY: `DEFAULT_ERROR_BODY_TIMEOUT_S` (upstream line 26).
pub const DEFAULT_ERROR_BODY_TIMEOUT_S: f64 = 10.0;

#[derive(Default)]
struct DrainState {
    chunks: VecDeque<Vec<u8>>,
    done: bool,
}

fn drain_chunks(
    chunks_iter: Box<dyn Iterator<Item = Vec<u8>> + Send>,
    max_bytes: usize,
    state: &Arc<DrainShared>,
) {
    let mut total = 0usize;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for chunk in chunks_iter {
            if chunk.is_empty() {
                continue;
            }
            let remaining = max_bytes.saturating_sub(total);
            if remaining == 0 {
                state.truncated.store(true, Ordering::SeqCst);
                break;
            }
            if chunk.len() > remaining {
                let mut taken = chunk;
                taken.truncate(remaining);
                state.lock().chunks.push_back(taken);
                total += remaining;
                state.truncated.store(true, Ordering::SeqCst);
                break;
            }
            total += chunk.len();
            state.lock().chunks.push_back(chunk);
        }
    }));
    if result.is_err() {
        // `except Exception` — the error path must not raise.
        log::debug!("bounded error-body read failed");
    }
}

#[derive(Default)]
struct DrainShared {
    inner: Mutex<DrainState>,
    signal: Condvar,
    truncated: AtomicBool,
}

impl DrainShared {
    fn lock(&self) -> std::sync::MutexGuard<'_, DrainState> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Read a non-OK streaming response body with a byte cap and a hard
/// deadline.
///
/// Returns the decoded body text (UTF-8, errors replaced), truncated to
/// `max_bytes`. Never raises/panics into the caller.
///
/// PARITY: `read_streaming_error_body` (upstream lines 40-99) — the worker
/// thread + hard deadline guarantees a bounded stop even when the
/// underlying read stalls mid-chunk.
pub fn read_streaming_error_body(
    chunks_iter: Box<dyn Iterator<Item = Vec<u8>> + Send>,
    max_bytes: usize,
    timeout_s: f64,
) -> String {
    let shared = Arc::new(DrainShared {
        truncated: AtomicBool::new(false),
        ..Default::default()
    });

    // Worker "thread": bounded by cap; panic-isolated. Draining inline
    // would risk an unbounded block, so — matching upstream — the drain
    // runs on a separate thread and the caller waits with the hard
    // deadline.
    let worker_shared = Arc::clone(&shared);
    let worker = std::thread::spawn(move || {
        drain_chunks(chunks_iter, max_bytes, &worker_shared);
        let mut state = worker_shared.lock();
        state.done = true;
        worker_shared.signal.notify_all();
    });

    let deadline = Instant::now() + Duration::from_secs_f64(timeout_s);
    let mut state = shared.lock();
    let mut timed_out = false;
    while !state.done {
        let now = Instant::now();
        if now >= deadline {
            timed_out = true;
            break;
        }
        let (guard, _) = shared
            .signal
            .wait_timeout(state, deadline - now)
            .unwrap_or_else(|e| e.into_inner());
        state = guard;
    }
    let collected: Vec<Vec<u8>> = state.chunks.drain(..).collect();
    drop(state);
    if timed_out {
        // Detach the still-blocked worker — upstream's daemon-thread
        // semantics. Rust threads are not daemonic, but the test harness /
        // process exit path does not join detached-and-abandoned threads
        // holding only their own blocked read.
        std::mem::forget(worker);
        log::debug!(
            "bounded error-body read: hard timeout after {:.1}s",
            timeout_s
        );
    } else {
        let _ = worker.join();
    }
    if shared.truncated.load(Ordering::SeqCst) {
        log::debug!("bounded error-body read: capped at {max_bytes} bytes");
    }

    let mut bytes = Vec::with_capacity(collected.iter().map(|c| c.len()).sum());
    for chunk in collected {
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Like [`read_streaming_error_body`] but returns `None` on empty body —
/// convenience for callers that distinguish "no body" from "empty
/// string".
///
/// PARITY: `read_error_body_or_default` (upstream lines 102-115).
pub fn read_error_body_or_default(
    chunks_iter: Box<dyn Iterator<Item = Vec<u8>> + Send>,
    max_bytes: usize,
    timeout_s: f64,
) -> Option<String> {
    let text = read_streaming_error_body(chunks_iter, max_bytes, timeout_s);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}
