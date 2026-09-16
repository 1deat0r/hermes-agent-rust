//! Best-effort native-parser warmup at agent-package import (P1).
//!
//! PARITY: `agent/jiter_preload.py` @ 5d59366 (whole module). Upstream
//! eagerly imports the OpenAI SDK's native streaming parser (`jiter`) once
//! at package import: on some Windows installs the native extension loads
//! fine from the venv but fails on first import inside the threaded
//! streaming path, so one early load avoids that while keeping the SDK's
//! normal error path for genuinely broken installs.
//!
//! The `jiter` native extension has no Rust counterpart, so the loader
//! crosses the seam as an injected closure ([`preload_with`]); the default
//! [`preload`] performs the Rust-side equivalent (a `serde_json` round-trip
//! that touches the JSON engine before the streaming path needs it).
//! Contract, mirroring upstream exactly: idempotent (short-circuits after
//! the first success), fail-open (loader errors become `false`, never
//! raised), last error retained for diagnostics and cleared on success.
//!
//! The module-level "run at import" half (`preload_jiter_native_extension()`
//! at `agent/jiter_preload.py` bottom + `agent/__init__.py` re-export) is
//! owned by the crate root: Rust has no import-time side effects, so the
//! binary entry point calls [`preload`] explicitly at startup.

use std::sync::Mutex;

/// Loader failure. Wraps the loader's message; the loader boundary stays
/// `String`-typed so callers aren't forced onto this crate's error enum for
/// an injected test seam (documented divergence from §3-thiserror at the
/// seam only — see `preload_with`).
#[derive(Debug, thiserror::Error)]
#[error("native parser preload failed: {0}")]
pub struct PreloadError(pub String);

static PRELOADED: Mutex<bool> = Mutex::new(false);
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

// PARITY: `preload_jiter_native_extension` (upstream lines 15-26) — the
// name mirrors upstream exactly (review finding); the loader is injected
// because the `jiter` native extension has no Rust counterpart.
/// Run the default Rust-side warmup. Never panics; reports success.
pub fn preload_jiter_native_extension() -> bool {
    preload_with(rust_json_warmup)
}

// PARITY: seam form of `preload_jiter_native_extension` — the `importlib`
// import crosses here as an injected closure (repo seam convention).
/// Best-effort load via an injected loader. Idempotent after first success;
/// fail-open on loader error. Never panics.
pub fn preload_with(loader: impl FnOnce() -> Result<(), String>) -> bool {
    if *PRELOADED.lock().unwrap_or_else(|e| e.into_inner()) {
        return true;
    }
    match loader() {
        Ok(()) => {
            if let Ok(mut flag) = PRELOADED.lock() {
                *flag = true;
            }
            if let Ok(mut err) = LAST_ERROR.lock() {
                *err = None;
            }
            true
        }
        Err(e) => {
            if let Ok(mut err) = LAST_ERROR.lock() {
                *err = Some(e);
            }
            false
        }
    }
}

// PARITY: `_JITER_PRELOAD_ERROR` read arm (upstream lines 12, 23-25) —
/// last loader error message, if the most recent attempt failed.
pub fn last_error() -> Option<String> {
    LAST_ERROR.lock().map(|e| e.clone()).unwrap_or(None)
}

// PARITY: the `importlib.import_module("jiter.jiter")` + `from_json` touch
// (upstream lines 20-21), adapted: no native parser exists in Rust.
/// Touch the JSON engine so first real use never pays init cost.
fn rust_json_warmup() -> Result<(), String> {
    let v: serde_json::Value =
        serde_json::from_str("{\"warmup\":true}").map_err(|e| e.to_string())?;
    let _ = serde_json::to_string(&v).map_err(|e| e.to_string())?;
    Ok(())
}

/// Test seam: clear the preloaded flag and recorded error.
pub fn reset_for_tests() {
    if let Ok(mut flag) = PRELOADED.lock() {
        *flag = false;
    }
    if let Ok(mut err) = LAST_ERROR.lock() {
        *err = None;
    }
}
