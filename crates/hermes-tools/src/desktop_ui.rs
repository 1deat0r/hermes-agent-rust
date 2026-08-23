//! Bridge desktop-only tools to Hermes-desktop renderer events.
//!
//! PARITY: tools/desktop_ui.py @ b9aa928 (40 LOC, ported 1:1). The preview
//! pane / pane-focus tools live in the desktop renderer, so desktop-gated
//! tools reach them through an emitter the desktop `tui_gateway` installs at
//! session start via [`set_emitter`]; everywhere else it stays `None` and the
//! tools report "desktop only".
//!
//! The upstream `get_session_env` (gateway.session_context) reads a
//! per-session env chain; the Rust seam is a per-thread sid provider the
//! gateway layer will install (mirrors the callback-injection pattern used by
//! clarify.rs). Until then it returns "" exactly like upstream's default.

use std::cell::Cell;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

/// Renderer-event sink signature: (session_sid, event, payload).
pub type Emitter = Box<dyn Fn(String, String, serde_json::Value) + Send + Sync>;

// The upstream `_emit` slot is process-global. Store an Arc clone so the
// mutex is not held while the gateway callback writes to stdout or re-enters
// this module.
static EMITTER: Lazy<Mutex<Option<Arc<Emitter>>>> = Lazy::new(|| Mutex::new(None));

// Session lookup remains thread-local because the tool executes on the
// session's worker thread; this is the Rust seam for gateway.session_context.
thread_local! {
    static SID_PROVIDER: Cell<Option<fn() -> String>> = const { Cell::new(None) };
}

/// Install (or clear) the renderer-event sink. Called by the desktop gateway.
pub fn set_emitter(fn_: Option<Emitter>) {
    *EMITTER.lock().unwrap() = fn_.map(Arc::from);
}

/// Gateway seam: installs the session-side `HERMES_UI_SESSION_ID` reader.
pub fn set_sid_provider(fn_: Option<fn() -> String>) {
    SID_PROVIDER.with(|slot| slot.set(fn_));
}

/// True when running under the desktop app (an emitter is wired).
pub fn available() -> bool {
    EMITTER.lock().unwrap().is_some()
}

/// Route `event` to the window that owns the current turn.
///
/// Returns `false` when no emitter is wired (i.e. not the desktop app).
pub fn emit(event: &str, payload: serde_json::Value) -> bool {
    let Some(emitter) = EMITTER.lock().unwrap().clone() else {
        return false;
    };
    let sid = SID_PROVIDER
        .with(|sp| sp.get().map(|g| g()).unwrap_or_default());
    emitter(sid, event.to_string(), payload);
    true
}
