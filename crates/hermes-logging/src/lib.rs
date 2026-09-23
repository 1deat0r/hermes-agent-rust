//! `hermes-logging` — 1:1 Rust port of `hermes_logging.py` (Nous Research
//! Hermes Agent, pinned @ 5d59366).
//!
//! Centralized logging setup for Hermes Agent: rotating file handlers
//! (`agent.log`, `errors.log`, `gateway.log`, `gui.log`), component routing,
//! thread-local session tags, per-record Hermes-home stamps for profile
//! routing (#97489), a background queue listener so log I/O never blocks the
//! emitting thread, external-rotation (inode) detection, unavailable-stream
//! (EIO) recovery, profile/second-home routing, and a pluggable redactor
//! (port of `agent/redact.py` lives in `redact`; `setup_logging` installs it
//! as the process-wide redactor).
//!
//! Port status: **re-certified @ 5d59366**. Documented divergences (PLAN.md §5):
//! - redaction is wired at `setup_logging` time (first install wins);
//!   before setup the seam stays Noop — no log facility exists yet.
//! - managed-mode (`is_managed()`) 0o660 chmod and the Windows
//!   concurrent-log-handler lock are deferred (POSIX primary target);
//!   `is_windows_concurrent_log_lock_timeout` exists for the oracle
//!   contract but is inert on POSIX (CLH never installed).
//! - `setup_verbose_logging` routes through its own stderr LogTarget; no
//!   Python root logger exists to lower or to pin `_NOISY_LOGGERS` against.
//! - `drain_log_queue` joins unbounded (std threads have no timed join).
//! - `_safe_stderr` has no Rust analog — `std::io::stderr` is
//!   Unicode-native on every platform (the cp949 wrap targets Python's
//!   text layer only).
//! - `read_logging_config` runs upstream's direct-parse FALLBACK (the
//!   `config_effective` cache lives in `hermes_cli`, an upward import);
//!   same values when no managed overlay is configured.
//! - `mkdir_under_hermes_home`'s named-profile liveness guard is not yet
//!   ported (`create_dir_all` fails open) — lands with hermes-constants
//!   profiles.
//! - non-EIO write errors print `--- Logging error ---` + the `io::Error`
//!   Display instead of a Python traceback.
//! - `rotating_file_handlers()` returns only bare handlers (upstream's one
//!   list also contained routers after routing; the oracle's post-routing
//!   assertions are isinstance-negative and match an empty typed list).
//! - atexit flush is registered via `libc::atexit` at first queue
//!   registration (statics never run `Drop`).
//! - upstream's `(name, home)` tuple entries in `enable_profile_log_routing`
//!   are accepted as plain paths only (no in-tree tuple caller).
//!
//! Targets (logger names) mirror the Python logger-name space so component
//! routing (`gateway.*`, `agent.*`, `tools.*`, `hermes_cli.*`, `cron.*`) is
//! preserved across the port.

pub mod profile;
pub mod queue;
pub mod record;
pub mod redact;
pub mod rotating;
pub mod setup;

pub use profile::{enable_profile_log_routing, ProfileRouter};
pub use queue::{
    drain_log_queue, flush_log_queue, register_queued_handler, register_queued_router,
    register_queued_target, reset_queued_handlers, rotating_file_handlers,
};
pub use record::{
    clear_session_context, install_redactor, set_session_context, Level, LogRecord, LogTarget,
    NoopRedactor, Redactor, LOG_FORMAT, LOG_FORMAT_VERBOSE,
};
pub use redact::{
    is_env_dump_command, mask_secret, redact_cdp_url, redact_for_egress, redact_sensitive_text,
    redact_terminal_output, RedactingFormatter, REDACTION_UNAVAILABLE,
};
pub use rotating::{
    is_unavailable_log_stream, is_windows_concurrent_log_lock_timeout, ComponentFilter,
    RotatingHandler,
};
pub use setup::{
    add_rotating_handler, read_logging_config, setup_logging, setup_verbose_logging, SetupOptions,
    COMPONENT_PREFIXES, NOISY_LOGGERS,
};

/// Log a record at a specific level/target (the worker applies filtering).
pub fn log(level: Level, target: &str, message: impl Into<String>) {
    crate::queue::enqueue(LogRecord::new(level, target, message));
}

/// DEBUG-level convenience.
#[macro_export]
macro_rules! debug_log {
    ($target:expr, $($arg:tt)*) => {
        $crate::log($crate::Level::Debug, $target, format!($($arg)*))
    };
}

/// INFO-level convenience.
#[macro_export]
macro_rules! info_log {
    ($target:expr, $($arg:tt)*) => {
        $crate::log($crate::Level::Info, $target, format!($($arg)*))
    };
}

/// WARNING-level convenience.
#[macro_export]
macro_rules! warn_log {
    ($target:expr, $($arg:tt)*) => {
        $crate::log($crate::Level::Warning, $target, format!($($arg)*))
    };
}

/// ERROR-level convenience.
#[macro_export]
macro_rules! error_log {
    ($target:expr, $($arg:tt)*) => {
        $crate::log($crate::Level::Error, $target, format!($($arg)*))
    };
}
