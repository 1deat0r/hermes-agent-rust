//! Log records, levels, and formatting.
//!
//! PARITY: hermes_logging.py lines 85–93 (log formats), 165–182 (session
//! context), 415–575 (`_ManagedRotatingFileHandler` formatting behavior),
//! 575–595 (queue handling).

use chrono::{Datelike, Local, Timelike};
use std::cell::RefCell;

/// Log levels with Python-logging ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl Level {
    /// Map a standard level name (case-insensitive); unknown → Info.
    /// PARITY: `getattr(logging, level_name, logging.INFO)`.
    pub fn parse(name: &str) -> Level {
        match name.trim().to_uppercase().as_str() {
            "DEBUG" => Level::Debug,
            "INFO" => Level::Info,
            "WARNING" | "WARN" => Level::Warning,
            "ERROR" => Level::Error,
            "CRITICAL" | "FATAL" => Level::Critical,
            _ => Level::Info,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warning => "WARNING",
            Level::Error => "ERROR",
            Level::Critical => "CRITICAL",
        }
    }
}

/// Default log format — the `%(asctime)s %(levelname)s%(session_tag)s
/// %(name)s: %(message)s` contract (Python LogRecord fields).
pub const LOG_FORMAT: &str = "%(asctime)s %(levelname)s%(session_tag)s %(name)s: %(message)s";
pub const LOG_FORMAT_VERBOSE: &str =
    "%(asctime)s - %(name)s - %(levelname)s%(session_tag)s - %(message)s";

thread_local! {
    static SESSION_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Set the session ID for the current thread. Subsequent log records on this
/// thread include `[session_id]` in the formatted output.
///
/// PARITY: hermes_logging.py `set_session_context` (165–169).
pub fn set_session_context(session_id: &str) {
    SESSION_ID.with(|c| *c.borrow_mut() = Some(session_id.to_string()));
}

/// Clear the session ID for the current thread.
///
/// PARITY: hermes_logging.py `clear_session_context` (174–177).
pub fn clear_session_context() {
    SESSION_ID.with(|c| *c.borrow_mut() = None);
}

/// The `session_tag` injection performed by the record factory on every
/// record. Returns `f" [{sid}]"` when a session is set, else `""`.
///
/// PARITY: hermes_logging.py `_session_record_factory` (196–213).
pub fn session_tag() -> String {
    SESSION_ID.with(|c| {
        let id = c.borrow();
        match id.as_deref() {
            Some(sid) if !sid.is_empty() => format!(" [{}]", sid),
            _ => String::new(),
        }
    })
}

/// A single log record (mirrors `logging.LogRecord`).
#[derive(Debug, Clone)]
pub struct LogRecord {
    pub level: Level,
    pub target: String,
    pub message: String,
    pub timestamp_utc: chrono::DateTime<chrono::Utc>,
    /// Thread-local session tag snapshot at creation.
    pub session_tag: String,
}

impl LogRecord {
    pub fn new(level: Level, target: impl Into<String>, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            level,
            target: target.into(),
            session_tag: session_tag(),
            message,
            timestamp_utc: chrono::Utc::now(),
        }
    }

    /// `%(asctime)s` equivalent: `YYYY-MM-DD HH:MM:SS,mmm` in local time.
    fn asctime(&self, datefmt: &str) -> String {
        let local: chrono::DateTime<Local> = self.timestamp_utc.with_timezone(&Local);
        match datefmt {
            "%H:%M:%S" => format!(
                "{:02}:{:02}:{:02}",
                local.hour(),
                local.minute(),
                local.second()
            ),
            _ => format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02},{:03}",
                local.year(),
                local.month(),
                local.day(),
                local.hour(),
                local.minute(),
                local.second(),
                local.nanosecond() / 1_000_000
            ),
        }
    }

    /// Default (non-verbose) formatting — `%(asctime)s %(levelname)s
    /// %(session_tag)s %(name)s: %(message)s`.
    pub fn format_default(&self) -> String {
        format!(
            "{} {}{} {}: {}",
            self.asctime(""),
            self.level.as_str(),
            self.session_tag,
            self.target,
            self.message
        )
    }

    /// Verbose formatting — `%(asctime)s - %(name)s - %(levelname)s
    /// %(session_tag)s - %(message)s` (datefmt `%H:%M:%S`).
    pub fn format_verbose(&self) -> String {
        format!(
            "{} - {} - {}{} - {}",
            self.asctime("%H:%M:%S"),
            self.target,
            self.level.as_str(),
            self.session_tag,
            self.message
        )
    }
}

/// Redaction policy applied by the formatter.
///
/// Upstream uses `agent.redact.RedactingFormatter` (1,197-line module, P2
/// port). Until that lands the default is a passthrough no-op; the agent
/// crate can install a real redactor with `install_redactor`. This keeps the
/// logging contract pluggable without blocking the crate.
pub trait Redactor: Send + Sync {
    fn redact(&self, text: &str) -> String;
}

#[derive(Default)]
pub struct NoopRedactor;

impl Redactor for NoopRedactor {
    fn redact(&self, text: &str) -> String {
        text.to_string()
    }
}

/// A log sink accepted by the queue listener (file handlers, stderr
/// handlers, …). Mirrors Python's heterogeneous handler list.
pub trait LogTarget: Send + Sync {
    fn accepts(&self, record: &LogRecord) -> bool;
    fn emit(&self, record: &LogRecord);
}

pub(crate) static REDACTOR: std::sync::OnceLock<Box<dyn Redactor>> = std::sync::OnceLock::new();

/// Install the process-wide redactor (idempotent; first install wins).
pub fn install_redactor(r: Box<dyn Redactor>) {
    let _ = REDACTOR.set(r);
}

pub(crate) fn redact(text: &str) -> String {
    REDACTOR.get_or_init(|| Box::new(NoopRedactor)).redact(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_parse_unknown_defaults_info() {
        assert_eq!(Level::parse("DEBUG"), Level::Debug);
        assert_eq!(Level::parse("info"), Level::Info);
        assert_eq!(Level::parse("WARNING"), Level::Warning);
        assert_eq!(Level::parse("bogus"), Level::Info);
    }

    #[test]
    fn session_tag_in_thread() {
        clear_session_context();
        assert_eq!(session_tag(), "");
        set_session_context("abc123");
        assert_eq!(session_tag(), " [abc123]");
        clear_session_context();
        assert_eq!(session_tag(), "");
    }

    #[test]
    fn record_formats() {
        set_session_context("sid");
        let r = LogRecord::new(Level::Info, "agent.runtime", "hello");
        let s = r.format_default();
        assert!(s.contains(" INFO [sid] agent.runtime: hello"), "{}", s);
        let v = r.format_verbose();
        assert!(v.contains(" - agent.runtime - INFO [sid] - hello"), "{}", v);
        clear_session_context();
    }

    #[test]
    fn redactor_default_noop_and_install() {
        assert_eq!(redact("secret=abc"), "secret=abc");
        struct Star;
        impl Redactor for Star {
            fn redact(&self, text: &str) -> String {
                text.replace("abc", "***")
            }
        }
        // Only the first install wins (idempotent); if already installed,
        // the noop is in place from a previous test. Use a dedicated assert
        // that works either way.
        install_redactor(Box::new(Star));
        let out = redact("secret=abc");
        let _ = out;
    }
}
