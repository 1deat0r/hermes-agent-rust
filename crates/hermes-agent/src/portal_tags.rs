//! Centralized Nous Portal request tags.
//!
//! PARITY: `agent/portal_tags.py` @ b9aa928 (whole module, lines 1-144).
//!
//! Every Hermes request that hits the Nous Portal — main agent loop,
//! auxiliary client (compression / titles / vision / web_extract /
//! session_search / etc.), and any future code path — must carry the same
//! product-attribution tags so Nous can attribute usage to Hermes Agent and
//! bucket it by client release.
//!
//! Tag shape (sent in OpenAI-compatible `extra_body["tags"]`):
//! `["product=hermes-agent", "client=hermes-client-v<version>"]`.
//!
//! DOCUMENTED DIVERGENCE (approved pending hermes-cli crate): upstream reads
//! the version live from `hermes_cli.__version__` on every call
//! (`_hermes_version`, upstream lines 85-95) so a hot-reloaded or bumped
//! release is picked up without a restart. There is no `hermes-cli` crate in
//! this workspace yet, so [`HERMES_VERSION`] is pinned to the value at the
//! `b9aa928` oracle (`"0.20.0"`, `hermes_cli/__init__.py` line 17) and
//! `hermes_client_tag` keeps upstream's compute-at-call-time shape so the
//! seam only has to replace the constant's owner. Upstream's `"unknown"`
//! import-failure fallback (line 95) has no Rust analogue: the version is a
//! compile-time constant here, so the branch is unreachable.
//!
//! Ambient propagation: upstream stores the id in a `ContextVar`, so
//! `tools.thread_context.propagate_context_to_thread` carries it onto worker
//! threads through `contextvars.copy_context()` (upstream test
//! `test_ambient_context_propagates_via_thread_context_helper`). The Rust port
//! is thread-local — which already matches a bare Python worker, where the id
//! is absent — and [`ConversationContext`] plus
//! [`propagate_conversation_context_to_thread`] reproduce the captured-Context
//! half of that contract for this one variable.
//!
//! PENDING SEAM: the fan-out call sites (tool executor, MoA fan-out, background
//! review) are not ported yet, and `hermes_tools::thread_context` still owns a
//! single process-wide snapshot factory slot reserved for the
//! approval/gateway contextvars layer. When that composite layer lands, this
//! module's capture should register with it instead of each consumer wrapping
//! the call itself.

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

/// PARITY: `hermes_cli.__version__` at upstream `b9aa928`
/// (`hermes_cli/__init__.py` line 17).
pub const HERMES_VERSION: &str = "0.20.0";

thread_local! {
    /// PARITY: `_conversation_id: ContextVar[Optional[str]]` (upstream
    /// `agent/portal_tags.py` lines 54-56). A `ContextVar` is per-context and
    /// a Rust `thread_local!` is per-thread; each worker starts with a fresh
    /// context upstream, so the ambient id is invisible to other threads in
    /// both models.
    static CONVERSATION_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Publish the active conversation id for ambient Portal tagging.
///
/// PARITY: `set_conversation_context` (upstream lines 59-67). Called by the
/// agent loop at turn entry with the conversation's stable id — the
/// session-lineage ROOT id, so the tag survives context-compression session
/// rotation. Pass `None` (or an empty string) to clear: upstream coerces any
/// falsy value with `conversation_id or None`, which for `str` inputs means
/// exactly `""`. Returns the previous value so callers can restore it on turn
/// exit, mirroring the upstream ContextVar token.
pub fn set_conversation_context(conversation_id: Option<&str>) -> Option<String> {
    let next = conversation_id
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    CONVERSATION_ID.with(|slot| {
        let mut current = slot.borrow_mut();
        std::mem::replace(&mut *current, next)
    })
}

/// Restore a previous conversation context (pair with
/// [`set_conversation_context`]).
///
/// PARITY: `reset_conversation_context` (upstream lines 70-77). A token owned
/// by another Context fails to reset and upstream falls back to clearing
/// rather than raising in cleanup paths; Rust tokens are same-thread by
/// construction, so the restore always applies and the fallback branch is
/// unreachable.
pub fn reset_conversation_context(token: Option<String>) {
    CONVERSATION_ID.with(|slot| *slot.borrow_mut() = token);
}

/// Return the ambient conversation id, or `None` when unset.
///
/// PARITY: `get_conversation_context` (upstream lines 80-82).
pub fn get_conversation_context() -> Option<String> {
    CONVERSATION_ID.with(|slot| slot.borrow().clone())
}

/// Return the `client=...` tag for Nous Portal requests.
///
/// PARITY: `hermes_client_tag` (upstream lines 98-103). Format:
/// `client=hermes-client-v<MAJOR>.<MINOR>.<PATCH>`.
pub fn hermes_client_tag() -> String {
    format!("client=hermes-client-v{HERMES_VERSION}")
}

/// Return the `conversation=...` tag for a Hermes session/conversation.
///
/// PARITY: `conversation_tag` (upstream lines 106-117). `session_id` is the
/// canonical Hermes conversation identifier — the same value used for
/// `~/.hermes/sessions/` storage, session logs, and lineage. Unlike the
/// product/client tags this is high-cardinality, so it is only appended when
/// a session id is actually available — never as part of the always-on base
/// tag set.
pub fn conversation_tag(session_id: &str) -> String {
    format!("conversation={session_id}")
}

/// Return the canonical list of Nous Portal product tags.
///
/// PARITY: `nous_portal_tags` (upstream lines 120-144). Always returns a
/// fresh list so callers can mutate it freely. The ambient conversation
/// context wins over the explicit `session_id` fallback: the agent loop
/// publishes the lineage ROOT id (stable across context-compression rotation
/// and delegate subagent trees), which is the better conversation key than a
/// per-segment `session_id` passed explicitly. Upstream's `or` chain also
/// treats a falsy ambient id or argument as absent, so an empty `session_id`
/// appends no tag.
pub fn nous_portal_tags(session_id: Option<&str>) -> Vec<String> {
    let mut tags = vec!["product=hermes-agent".to_string(), hermes_client_tag()];
    let effective = get_conversation_context()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            session_id
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        });
    if let Some(effective) = effective {
        tags.push(conversation_tag(&effective));
    }
    tags
}

/// A captured ambient conversation context.
///
/// PARITY: the `contextvars.Context` object that
/// `tools.thread_context.propagate_context_to_thread` builds with
/// `copy_context()` (upstream `tools/thread_context.py` line 84) and enters on
/// the worker with `ctx.run(_inner)` (line 116), specialized to this module's
/// single `ContextVar`. Like a reused Python `Context`, the captured value is
/// stable against later publishes on the capturing thread, values written
/// inside a run persist in this context for its next run, and the calling
/// thread's own ambient id is restored when the run returns — including on
/// unwind, which is the Rust analogue of CPython restoring the context after
/// the callable exits.
#[derive(Default)]
pub struct ConversationContext {
    state: Mutex<Option<String>>,
}

struct ContextRestore<'a> {
    slot: &'a Mutex<Option<String>>,
    previous: Option<String>,
}

impl Drop for ContextRestore<'_> {
    fn drop(&mut self) {
        // Persist what the frame wrote into the captured context, then put the
        // worker's own ambient id back. Poisoning only happens if a nested run
        // panicked while holding the slot, so recovering the guard keeps the
        // restore path non-raising like upstream's context exit.
        let mut guard = self
            .slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = get_conversation_context();
        drop(guard);
        reset_conversation_context(self.previous.take());
    }
}

impl ConversationContext {
    /// Snapshot the calling thread's ambient conversation id.
    ///
    /// PARITY: `contextvars.copy_context()` called on the parent thread.
    pub fn capture() -> Self {
        Self {
            state: Mutex::new(get_conversation_context()),
        }
    }

    /// Run `f` under this captured context and return its result.
    ///
    /// PARITY: `Context.run`. Two CPython guarantees are deliberately not
    /// reproduced: entering the same `Context` recursively, or entering one
    /// that another thread is already inside, raises `RuntimeError` there.
    /// Here both cases are serialized through the captured slot — the nested
    /// frame sees (and can write back to) the same captured value, and two
    /// concurrent workers each get their own thread-local while sharing the
    /// captured value. No upstream behavior depends on that distinction
    /// because each fan-out site wraps a fresh context.
    pub fn run<T>(&self, f: impl FnOnce() -> T) -> T {
        let captured = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let previous = set_conversation_context(captured.as_deref());
        let _restore = ContextRestore {
            slot: &self.state,
            previous,
        };
        f()
    }
}

/// Wrap `target` so a worker thread runs it under the *current* thread's
/// ambient conversation id.
///
/// PARITY: `tools.thread_context.propagate_context_to_thread` (upstream
/// `tools/thread_context.py` lines 66-118) restricted to this module's
/// `ContextVar`: capture happens on the parent thread at call time, the
/// returned callable may be invoked repeatedly (each call re-enters the
/// captured context), and the worker's own ambient id is restored on exit.
/// Positional/keyword forwarding is left to the caller's closure, since Rust
/// has no `*args`; the argument-carrying general case stays with
/// `hermes_tools::thread_context::propagate_context_to_thread`.
/// The approval/sudo callback propagation half of the upstream helper stays
/// with `hermes_tools::thread_context`, whose snapshot-factory slot is where a
/// composite multi-variable context will register.
pub fn propagate_conversation_context_to_thread<R, F>(
    target: F,
) -> impl Fn() -> R + Send + Sync + 'static
where
    R: Send + 'static,
    F: Fn() -> R + Send + Sync + 'static,
{
    let context = Arc::new(ConversationContext::capture());
    let target = Arc::new(target);
    move || {
        // The run only needs `&self`, so the shared captured context can serve
        // every invocation of this wrapper.
        let target = Arc::clone(&target);
        context.run(move || (target)())
    }
}
