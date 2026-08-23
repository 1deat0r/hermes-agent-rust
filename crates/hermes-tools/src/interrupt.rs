//! Per-thread interrupt signaling for all tools.
//!
//! PARITY: tools/interrupt.py @ b9aa928 (113 LOC, ported 1:1).
//!
//! Provides thread-scoped interrupt tracking so that interrupting one agent
//! session does not kill tools running in other sessions. This is critical
//! in the gateway where multiple agents run concurrently in the same process.
//!
//! The agent stores its execution thread ID at the start of
//! run_conversation() and passes it to set_interrupt()/clear_interrupt().
//! Tools call is_interrupted() which checks the CURRENT thread.
//!
//! The thread id type is `std::thread::ThreadId` instead of Python's int
//! ident. The observable contract is identical: ids are stable per thread,
//! distinct across live threads, and unused by other code except as opaque
//! set keys.
//!
//! Env-read (`HERMES_DEBUG_INTERRUPT`) is cached at first access for the
//! process lifetime, exactly like the upstream module-load global. The
//! upstream `logger.setLevel(logging.INFO)` in debug mode is a no-op here —
//! `log::info!` is already at info level; the debug trace lines remain.

use std::collections::HashSet;
use std::sync::Mutex;
use std::thread::ThreadId;

use once_cell::sync::Lazy;

/// Opt-in debug tracing — pairs with `HERMES_DEBUG_INTERRUPT` in
/// tools/environments/base.py. Enables per-call logging of set/check so the
/// caller thread, target thread, and current state are visible when
/// diagnosing "interrupt signaled but tool never saw it" reports.
static DEBUG_INTERRUPT: Lazy<bool> = Lazy::new(|| {
    std::env::var("HERMES_DEBUG_INTERRUPT")
        .map(|value| !value.is_empty())
        .unwrap_or(false)
});

/// Set of thread ids that have been interrupted. Exposed for parity with the
/// upstream module globals `_interrupted_threads` / `_lock` (tests and
/// process_registry inspect it directly).
pub static INTERRUPTED_THREADS: Lazy<Mutex<HashSet<ThreadId>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

/// Set or clear interrupt for a specific thread.
///
/// `active`: True to signal interrupt, False to clear it.
/// `thread_id`: Target thread id. When None, targets the current thread
/// (backward compat for CLI/tests).
pub fn set_interrupt(active: bool, thread_id: Option<ThreadId>) {
    let tid = thread_id.unwrap_or_else(|| std::thread::current().id());
    let snapshot = {
        let mut interrupted = INTERRUPTED_THREADS.lock().unwrap();
        if active {
            interrupted.insert(tid);
        } else {
            interrupted.remove(&tid);
        }
        if *DEBUG_INTERRUPT {
            Some(interrupted.iter().copied().collect::<Vec<_>>())
        } else {
            None
        }
    };
    if let Some(current_set) = snapshot {
        log::info!(
            "[interrupt-debug] set_interrupt(active={}, target_tid={:?}) called_from_tid={:?} current_set={:?}",
            active,
            tid,
            std::thread::current().id(),
            current_set
        );
    }
}

/// Check if an interrupt has been requested for the current thread.
///
/// Safe to call from any thread — each thread only sees its own interrupt
/// state.
pub fn is_interrupted() -> bool {
    let tid = std::thread::current().id();
    INTERRUPTED_THREADS.lock().unwrap().contains(&tid)
}

/// Clear any interrupt bit on the CURRENT thread.
///
/// Gives a user-approved command a clean interrupt slate immediately before
/// it spawns its child process, so a stale bit that landed on this thread
/// during the blocking approval-wait cannot SIGINT the just-approved run.
/// Call this directly, never via the `_interrupt_event` proxy (its .clear()
/// binds to whatever thread runs it).
pub fn clear_current_thread_interrupt() {
    // thread_id=None -> current thread (see set_interrupt)
    set_interrupt(false, None);
}

/// Drop-in proxy that maps `threading.Event` methods to per-thread state
/// (legacy `_interrupt_event` shim).
#[derive(Debug, Clone, Copy)]
pub struct ThreadAwareEventProxy;

/// Backward-compatible `_interrupt_event` proxy.
pub static INTERRUPT_EVENT: ThreadAwareEventProxy = ThreadAwareEventProxy;

impl ThreadAwareEventProxy {
    pub fn is_set(&self) -> bool {
        is_interrupted()
    }

    pub fn set(&self) {
        set_interrupt(true, None);
    }

    pub fn clear(&self) {
        set_interrupt(false, None);
    }

    /// Not truly supported — returns the current state immediately.
    pub fn wait(&self, _timeout: Option<std::time::Duration>) -> bool {
        self.is_set()
    }
}
