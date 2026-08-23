//! Propagate agent-turn context into worker threads that dispatch Hermes tools.
//!
//! PARITY: tools/thread_context.py @ b9aa928 (120 LOC, ported 1:1).
//!
//! A bare `std::thread` / `ThreadPoolExecutor` worker starts with no tool
//! context: the approval session/platform ContextVars and the thread-local
//! CLI approval/sudo callbacks are lost, so gateway sessions could fall into
//! non-interactive auto-approve and dangerous commands could run without
//! prompting. This helper factors out that capture/install/clear lifecycle.
//!
//! Usage — call [`propagate_context_to_thread`] **on the parent thread** (it
//! snapshots the parent's callbacks at call time) and use the returned
//! callable as the worker's target:
//!
//! ```text
//! thread::spawn(propagate_context_to_thread(worker_fn));
//! // or
//! executor.submit(propagate_context_to_thread(worker_fn));
//! ```
//!
//! Approval/sudo callbacks are installed for the worker's lifetime and
//! always cleared on exit (via an RAII guard — the Rust equivalent of
//! upstream's `finally`), so a recycled thread never holds a stale reference
//! to a disposed CLI instance.
//!
//! SEAMS (documented):
//!  * Callback slots: upstream reads/writes `tools.terminal_tool`'s
//!    thread-local `_callback_tls`. That module isn't ported yet; the slots
//!    below live here and expose the same four accessor functions, which the
//!    terminal_tool port will re-export (or move) when it lands.
//!  * ContextVars: upstream copies `contextvars.Context`; the approval
//!    /gateway contextvars layer will install a snapshot factory via
//!    [`set_context_snapshot_factory`]. Until then the context is empty and
//!    `run` is identity — matching a process with no ContextVars registered.

use std::any::Any;
use std::cell::RefCell;
use std::sync::{Arc, Mutex, OnceLock};

/// Opaque terminal-tool callback payload (approval / sudo password prompt).
/// thread_context only carries these between threads; the terminal_tool port
/// defines the concrete callable types and downcasts on use.
pub type ToolCallback = dyn Any + Send + Sync;

thread_local! {
    static APPROVAL_CB: RefCell<Option<Arc<ToolCallback>>> = const { RefCell::new(None) };
    static SUDO_CB: RefCell<Option<Arc<ToolCallback>>> = const { RefCell::new(None) };
}

/// Current thread's approval-prompt callback (mirrors
/// `terminal_tool._get_approval_callback()`).
pub fn get_approval_callback() -> Option<Arc<ToolCallback>> {
    APPROVAL_CB.with(|slot| slot.borrow().clone())
}

/// Current thread's sudo-password-prompt callback (mirrors
/// `terminal_tool._get_sudo_password_callback()`).
pub fn get_sudo_password_callback() -> Option<Arc<ToolCallback>> {
    SUDO_CB.with(|slot| slot.borrow().clone())
}

/// Register (or clear) the current thread's approval callback (mirrors
/// `terminal_tool.set_approval_callback`).
pub fn set_approval_callback(cb: Option<Arc<ToolCallback>>) {
    APPROVAL_CB.with(|slot| *slot.borrow_mut() = cb);
}

/// Register (or clear) the current thread's sudo-password callback (mirrors
/// `terminal_tool.set_sudo_password_callback`).
pub fn set_sudo_password_callback(cb: Option<Arc<ToolCallback>>) {
    SUDO_CB.with(|slot| *slot.borrow_mut() = cb);
}

/// The four terminal_tool callback accessors, resolved as one unit (mirrors
/// upstream's private `_callback_api`, which imports the getters/setters
/// lazily from `tools.terminal_tool`).
#[derive(Clone, Copy)]
struct CallbackApi {
    get_approval: fn() -> Option<Arc<ToolCallback>>,
    get_sudo: fn() -> Option<Arc<ToolCallback>>,
    set_approval: fn(Option<Arc<ToolCallback>>),
    set_sudo: fn(Option<Arc<ToolCallback>>),
}

fn callback_api() -> CallbackApi {
    CallbackApi {
        get_approval: get_approval_callback,
        get_sudo: get_sudo_password_callback,
        set_approval: set_approval_callback,
        set_sudo: set_sudo_password_callback,
    }
}

/// Clears both propagated callbacks on drop — the Rust equivalent of
/// upstream's `finally:` so the worker thread is cleaned even when the
/// target panics.
struct CallbackClearGuard {
    set_approval: fn(Option<Arc<ToolCallback>>),
    set_sudo: fn(Option<Arc<ToolCallback>>),
}

impl Drop for CallbackClearGuard {
    fn drop(&mut self) {
        (self.set_approval)(None);
        (self.set_sudo)(None);
    }
}

/// A captured parent-thread context, mirror of `contextvars.copy_context()`.
/// Implementors must not leak the context past `run`'s return (like
/// `Context.run`'s enter/exit discipline).
pub trait ContextSnapshot: Send + Sync {
    /// Run `f` under this snapshot's context.
    fn run(&self, f: Box<dyn FnOnce() + Send>);
}

/// Factory seam: produces the calling thread's context snapshot (mirror of
/// `contextvars.copy_context()`).
pub type SnapshotFactory = dyn Fn() -> Arc<dyn ContextSnapshot> + Send + Sync;

static SNAPSHOT_FACTORY: OnceLock<Mutex<Option<Arc<SnapshotFactory>>>> = OnceLock::new();

/// Install (or clear) the context-snapshot factory. The approval/gateway
/// contextvars layer installs one; without it the context is empty.
pub fn set_context_snapshot_factory(factory: Option<Arc<SnapshotFactory>>) {
    let cell = SNAPSHOT_FACTORY.get_or_init(|| Mutex::new(None));
    *cell.lock().unwrap() = factory;
}

/// Empty context snapshot: run is identity (no ContextVars registered).
struct EmptySnapshot;

impl ContextSnapshot for EmptySnapshot {
    fn run(&self, f: Box<dyn FnOnce() + Send>) {
        f();
    }
}

fn capture_snapshot() -> Arc<dyn ContextSnapshot> {
    if let Some(cell) = SNAPSHOT_FACTORY.get() {
        if let Ok(guard) = cell.lock() {
            if let Some(factory) = &*guard {
                return factory();
            }
        }
    }
    Arc::new(EmptySnapshot)
}

/// Wrap *target* for execution on a worker thread with the *current* thread's
/// approval/sudo callbacks propagated.
///
/// Call this on the parent thread; pass the returned callable as the
/// thread/executor target. The target (and its captures) must be
/// `Send + Sync`: it is owned by the wrapper and is invoked from a worker
/// thread, where Python's GIL would otherwise mask shared mutable state. The returned callable forwards its argument to
/// *target* and returns its result. It is callable multiple times (each call
/// reinstalls the captured callbacks and clears them on exit).
///
/// Fail-closed semantics are preserved structurally: if callback capture were
/// unavailable the setters would be left unset (the terminal_tool import
/// failure branch upstream). With the built-in thread-local slots the
/// installation/clearing cannot raise, so the defensive try/except branches
/// are unreachable and are not repeated here.
pub fn propagate_context_to_thread<F, A, R>(target: F) -> impl Fn(A) -> R + Send + 'static
where
    F: Fn(A) -> R + Send + Sync + 'static,
    A: Send + 'static,
    R: Send + 'static,
{
    // Parent-thread capture (mirrors contextvars.copy_context() +
    // _callback_api()).
    let ctx = capture_snapshot();
    let api = callback_api();
    let parent_approval_cb = (api.get_approval)();
    let parent_sudo_cb = (api.get_sudo)();
    let target = Arc::new(target);

    move |args: A| {
        let result_slot = Arc::new(Mutex::new(None::<R>));
        let slot = result_slot.clone();
        let target = target.clone();
        let parent_approval_cb = parent_approval_cb.clone();
        let parent_sudo_cb = parent_sudo_cb.clone();

        let run_body: Box<dyn FnOnce() + Send> = Box::new(move || {
            // Install propagated callbacks for the worker's lifetime; only
            // non-None callbacks are installed (mirrors upstream).
            if let Some(cb) = &parent_approval_cb {
                (api.set_approval)(Some(cb.clone()));
            }
            if let Some(cb) = &parent_sudo_cb {
                (api.set_sudo)(Some(cb.clone()));
            }
            // Cleared on exit (even on panic) so a recycled thread never
            // holds a stale reference to a disposed CLI instance.
            let _clear_guard = CallbackClearGuard {
                set_approval: api.set_approval,
                set_sudo: api.set_sudo,
            };

            let result = (&*target)(args);
            *slot.lock().unwrap() = Some(result);
        });

        ctx.run(run_body);
        // Bind before the block ends so the MutexGuard (and its borrow of
        // result_slot) drops inside the same scope as the Arc.
        {
            let mut guard = result_slot.lock().unwrap();
            guard
                .take()
                .expect("propagate_context_to_thread: worker target must run exactly once")
        }
    }
}
