//! UTF-8 stdio guard for the Hermes CLI.
//!
//! PARITY: `hermes_cli/__init__.py::_ensure_utf8` @ 5d59366.
//!
//! The CLI prints box-drawing characters and the ☤ glyph in the setup wizard,
//! doctor, and status banners; under a non-UTF-8 codec that raises before the
//! command can even start (e.g. `hermes setup` on a fresh Pi with a latin-1 /
//! C / POSIX locale).
//!
//! What ports and what does not:
//! - [`is_utf8_encoding`] mirrors the source's per-stream UTF-8 check
//!   (`(encoding or "").lower().replace("-", "") == "utf8"`) exactly,
//!   including its quirks: no whitespace trimming and only `-` (not `_`) is
//!   stripped, so `"utf_8"` does *not* count.
//! - [`note_stdio_repaired`] mirrors the child-process nudge
//!   (`os.environ.setdefault("PYTHONUTF8", "1")` +
//!   `setdefault("PYTHONIOENCODING", "utf-8")`), which the source runs only
//!   when a non-UTF-8 locale was actually repaired, leaving a healthy UTF-8
//!   host's environment untouched.
//! - The stream repair itself (`TextIOWrapper.reconfigure()` in place, else
//!   reopening the fd as UTF-8 with `closefd=False`) is Python-runtime
//!   specific and has no Rust analog: Rust strings are UTF-8 end to end and
//!   the standard streams have no locale codec to misfire. The entry point
//!   that embeds this crate calls [`note_stdio_repaired`] when it repairs
//!   non-UTF-8 child-process state it detects itself.

/// PARITY: the source's `(getattr(stream, "encoding", "") or
/// "").lower().replace("-", "") == "utf8"` gate.
pub fn is_utf8_encoding(encoding: &str) -> bool {
    encoding.to_lowercase().replace('-', "") == "utf8"
}

/// PARITY: the `if repaired:` tail of `_ensure_utf8` — nudge child processes
/// toward UTF-8, but only via set-default so an explicitly configured
/// environment is never overwritten. Call this only when a non-UTF-8 locale
/// was actually detected and repaired; on a healthy UTF-8 host it must not
/// run at all (minimal footprint).
pub fn note_stdio_repaired() {
    // Plain `set_var`: this toolchain's `std::env::set_var` is safe in
    // non-test code (test targets wrap it in `unsafe` + a mutex by repo
    // convention); the production entry point calls this once at startup.
    if std::env::var_os("PYTHONUTF8").is_none() {
        std::env::set_var("PYTHONUTF8", "1");
    }
    if std::env::var_os("PYTHONIOENCODING").is_none() {
        std::env::set_var("PYTHONIOENCODING", "utf-8");
    }
}
