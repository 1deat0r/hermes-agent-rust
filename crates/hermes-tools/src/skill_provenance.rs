//! Skill write-origin provenance — distinguish agent-sediment skill writes
//! from foreground user-directed writes.
//!
//! PARITY: `tools/skill_provenance.py` @ b9aa928 (whole module).
//!
//! The curator only consolidates/prunes skills it autonomously created via
//! the background self-improvement review fork. Skills a user asks a
//! foreground agent to write belong to the user and must never be
//! auto-curated.
//!
//! `run_agent` sets this before each tool loop so tool handlers (e.g.
//! skill_manage create) can check whether they are executing inside the
//! background-review fork. The signal piggybacks on
//! `AIAgent._memory_write_origin`, which is already
//! `"background_review"` for review-fork instances and defaults to
//! `"assistant_tool"` for normal (foreground) agents.
//!
//! CONTEXTVAR NOTE: upstream's `contextvars.ContextVar` becomes a
//! thread-local slot (the crate-wide convention, as in `thread_context`);
//! the reset token carries the previous value, so `reset_current_write_origin`
//! restores it exactly like `ContextVar.reset(token)`.
//!
//! Usage:
//!
//! ```ignore
//! let token = set_current_write_origin("background_review");
//! // ... tool runs here
//! reset_current_write_origin(token);
//! ```

use std::cell::RefCell;

/// The sentinel value the background review fork uses; mirrors run_agent's
/// `AIAgent._memory_write_origin` override in `_spawn_background_review()`.
///
/// PARITY: `BACKGROUND_REVIEW` (upstream line 33).
pub const BACKGROUND_REVIEW: &str = "background_review";

/// PARITY: the ContextVar's `default="foreground"` (upstream lines 27-30).
pub const FOREGROUND: &str = "foreground";

thread_local! {
    static WRITE_ORIGIN: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Opaque reset token: the prior origin at `set` time.
///
/// PARITY: `contextvars.Token[str]`.
#[derive(Debug, Clone)]
pub struct WriteOriginToken(String);

/// Bind the active write origin to the current context.
///
/// Returns a token the caller must pass to [`reset_current_write_origin`]
/// in a `finally` block. Empty origins coerce to `"foreground"` at the
/// `set()` boundary (Python `origin or "foreground"`).
///
/// PARITY: `set_current_write_origin` (upstream lines 38-45).
pub fn set_current_write_origin(origin: &str) -> WriteOriginToken {
    WRITE_ORIGIN.with(|slot| {
        let mut slot = slot.borrow_mut();
        let token = WriteOriginToken(slot.clone().unwrap_or_else(|| FOREGROUND.to_string()));
        let origin = if origin.is_empty() {
            FOREGROUND
        } else {
            origin
        };
        *slot = Some(origin.to_string());
        token
    })
}

/// Restore the prior write origin context.
///
/// PARITY: `reset_current_write_origin` (upstream lines 48-49). The token
/// carries the full prior value, so restore is exact (Python's stale-token
/// `ValueError` belongs to the cross-context case the Rust port cannot
/// reach — tokens only work within their thread, as the upstream
/// try/finally usage prescribes).
pub fn reset_current_write_origin(token: WriteOriginToken) {
    WRITE_ORIGIN.with(|slot| {
        *slot.borrow_mut() = Some(token.0);
    });
}

/// Return the active write origin.
///
/// Default: `"foreground"` — any tool call made by a regular (non-review)
/// agent, from the CLI, the gateway, cron, or a subagent.
///
/// `"background_review"` — the self-improvement review fork; only skills
/// created under this origin should be marked agent-created for curator
/// management.
///
/// PARITY: `get_current_write_origin` (upstream lines 62-71); the slot
/// materializes the ContextVar default on first read.
pub fn get_current_write_origin() -> String {
    WRITE_ORIGIN
        .with(|slot| slot.borrow().clone())
        .unwrap_or_else(|| FOREGROUND.to_string())
}

/// Convenience: true iff the current write origin is the background review
/// fork.
///
/// PARITY: `is_background_review` (upstream lines 74-75).
pub fn is_background_review() -> bool {
    get_current_write_origin() == BACKGROUND_REVIEW
}
