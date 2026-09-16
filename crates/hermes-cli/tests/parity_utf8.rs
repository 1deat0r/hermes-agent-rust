//! Source-derived parity oracle for `hermes_cli/__init__.py::_ensure_utf8`
//! @ 5d59366 (oracle: `tests/hermes_cli/test_ensure_utf8_locale.py`).
//!
//! Only the portable contract ports: the UTF-8 codec gate
//! ([`hermes_cli::utf8::is_utf8_encoding`]) and the repaired-only child-process
//! env nudge ([`hermes_cli::utf8::note_stdio_repaired`]). The stream repair
//! itself (`TextIOWrapper.reconfigure()` in place, else reopening the fd with
//! `closefd=False`) is Python-runtime specific and has no Rust analog —
//! documented in the module, not guessed.
//! Tier: unit.

use std::sync::Mutex;

use hermes_cli::utf8::{is_utf8_encoding, note_stdio_repaired};

static UTF8_ENV_LOCK: Mutex<()> = Mutex::new(());

struct SavedEnv {
    python_utf8: Option<std::ffi::OsString>,
    python_io_encoding: Option<std::ffi::OsString>,
}

impl SavedEnv {
    fn capture() -> Self {
        Self {
            python_utf8: std::env::var_os("PYTHONUTF8"),
            python_io_encoding: std::env::var_os("PYTHONIOENCODING"),
        }
    }
    fn restore(self) {
        match self.python_utf8 {
            Some(value) => unsafe { std::env::set_var("PYTHONUTF8", value) },
            None => unsafe { std::env::remove_var("PYTHONUTF8") },
        }
        match self.python_io_encoding {
            Some(value) => unsafe { std::env::set_var("PYTHONIOENCODING", value) },
            None => unsafe { std::env::remove_var("PYTHONIOENCODING") },
        }
    }
}

#[test]
fn utf8_streams_are_left_untouched() {
    // PARITY: `test_utf8_stream_left_untouched` — already-UTF-8 streams are a
    // no-op: the gate recognizes every UTF-8 spelling the source accepts.
    for encoding in ["utf-8", "utf8", "UTF-8", "UTF8", "Utf-8"] {
        assert!(is_utf8_encoding(encoding), "{encoding} must count as UTF-8");
    }
}

#[test]
fn non_utf8_locales_are_detected() {
    // The Pi/latin-1/C/POSIX crash class: anything else trips the repair.
    // Quirks pinned: no trimming, only `-` stripped (so `"utf_8"` fails).
    for encoding in ["latin-1", "ascii", "cp1252", "", "ANSI_X3.4-1968", "utf_8"] {
        assert!(
            !is_utf8_encoding(encoding),
            "{encoding} must not count as UTF-8"
        );
    }
}

#[test]
fn repair_nudges_child_processes_toward_utf8() {
    // PARITY: the `if repaired:` tail — both vars default in when absent.
    let _guard = UTF8_ENV_LOCK.lock().unwrap();
    let saved = SavedEnv::capture();
    unsafe {
        std::env::remove_var("PYTHONUTF8");
        std::env::remove_var("PYTHONIOENCODING");
    }
    note_stdio_repaired();
    let result = (
        std::env::var("PYTHONUTF8"),
        std::env::var("PYTHONIOENCODING"),
    );
    saved.restore();
    assert_eq!(result.0.as_deref(), Ok("1"));
    assert_eq!(result.1.as_deref(), Ok("utf-8"));
}

#[test]
fn repair_never_overwrites_explicit_configuration() {
    // PARITY: `setdefault` semantics — an explicitly configured environment
    // is never overwritten.
    let _guard = UTF8_ENV_LOCK.lock().unwrap();
    let saved = SavedEnv::capture();
    unsafe {
        std::env::set_var("PYTHONUTF8", "0");
        std::env::set_var("PYTHONIOENCODING", "latin-1");
    }
    note_stdio_repaired();
    let result = (
        std::env::var("PYTHONUTF8"),
        std::env::var("PYTHONIOENCODING"),
    );
    saved.restore();
    assert_eq!(result.0.as_deref(), Ok("0"));
    assert_eq!(result.1.as_deref(), Ok("latin-1"));
}

#[test]
fn healthy_utf8_host_sees_no_environment_mutation() {
    // PARITY: `test_utf8_stream_left_untouched` env half — on a healthy UTF-8
    // host the gate matches and the nudge never runs, so the environment is
    // untouched. Pins the gate side-effect free: evaluating it over a battery
    // of encodings must not mutate either variable.
    let _guard = UTF8_ENV_LOCK.lock().unwrap();
    let saved = SavedEnv::capture();
    for encoding in ["utf-8", "UTF-8", "latin-1", "ascii", "cp1252", ""] {
        let _ = is_utf8_encoding(encoding);
    }
    let result = (
        std::env::var_os("PYTHONUTF8"),
        std::env::var_os("PYTHONIOENCODING"),
    );
    let unchanged = result.0 == saved.python_utf8 && result.1 == saved.python_io_encoding;
    saved.restore();
    assert!(unchanged);
}
