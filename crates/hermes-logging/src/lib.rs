//! `hermes-logging` — 1:1 Rust port of `hermes_logging.py` (Nous Research
//! Hermes Agent, pinned @ b9aa928).
//!
//! Centralized logging setup for Hermes Agent: rotating file handlers
//! (`agent.log`, `errors.log`, `gateway.log`, `gui.log`), component routing,
//! thread-local session tags, a background queue listener so log I/O never
//! blocks the emitting thread, external-rotation (inode) detection, and a
//! pluggable redactor (port of `agent/redact.py` lives in `redact`;
//! `setup_logging` installs it as the process-wide redactor).
//!
//! Port status: **core complete** (P1). Documented divergences (PLAN.md §5):
//! - redaction is wired at `setup_logging` time (first install wins);
//!   before setup the seam stays Noop — no log facility exists yet.
//! - managed-mode (`is_managed()`) 0o660 chmod and the Windows
//!   concurrent-log-handler lock are deferred (POSIX primary target);
//! - `setup_verbose_logging` routes through its own stderr LogTarget.
//! - `drain_log_queue` joins unbounded (std threads have no timed join).
//!
//! Targets (logger names) mirror the Python logger-name space so component
//! routing (`gateway.*`, `agent.*`, `tools.*`, `hermes_cli.*`, `cron.*`) is
//! preserved across the port.

pub mod queue;
pub mod redact;
pub mod record;
pub mod rotating;
pub mod setup;

pub use queue::{
    drain_log_queue, flush_log_queue, register_queued_handler, register_queued_target,
    reset_queued_handlers, rotating_file_handlers,
};
pub use redact::{redact_cdp_url, redact_sensitive_text, redact_terminal_output, is_env_dump_command, mask_secret, RedactingFormatter};
pub use record::{
    clear_session_context, install_redactor, set_session_context, Level, LogRecord, LogTarget,
    NoopRedactor, Redactor, LOG_FORMAT, LOG_FORMAT_VERBOSE,
};
pub use rotating::{ComponentFilter, RotatingHandler};
pub use setup::{
    add_rotating_handler, read_logging_config, setup_logging, setup_verbose_logging,
    SetupOptions, COMPONENT_PREFIXES, NOISY_LOGGERS,
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
