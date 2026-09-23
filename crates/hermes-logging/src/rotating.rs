//! Rotating file handler with managed-mode chmod, external-rotation
//! inode detection, and unavailable-stream (EIO) recovery.
//!
//! PARITY: hermes_logging.py `_ManagedRotatingFileHandler` (295–408) +
//! helpers `_is_windows_concurrent_log_lock_timeout` (70–81) and
//! `_is_unavailable_log_stream` (84–89).

use crate::record::{Level, LogRecord};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Optional component filter: only pass records whose target starts with one
/// of the prefixes.
///
/// PARITY: `_ComponentFilter` (147–155) + `COMPONENT_PREFIXES` (159–169).
#[derive(Debug, Clone, Default)]
pub struct ComponentFilter {
    pub prefixes: Vec<String>,
}

impl ComponentFilter {
    pub fn matches(&self, target: &str) -> bool {
        self.prefixes.iter().any(|p| target.starts_with(p.as_str()))
    }
}

/// True for concurrent-log-handler's Windows lock timeout message.
///
/// PARITY: `_is_windows_concurrent_log_lock_timeout` (70–81). The helper is
/// inert off Windows: CLH (and the RuntimeError it raises) is only installed
/// on win32, so the POSIX branch always returns false (oracle linux_only test).
/// The `isinstance(exc, RuntimeError)` check maps to "this is an error
/// message text", the only shape the port's `io::Error` path carries.
///
/// PARITY note: not wired into `emit` on POSIX (no CLH dependency); kept for
/// the oracle contract and the future Windows swap (#44873).
pub fn is_windows_concurrent_log_lock_timeout(message: &str) -> bool {
    cfg!(windows) && message.contains("Cannot acquire lock after 20 attempts")
}

/// True when a file handler lost its backing stream during teardown or I/O.
///
/// PARITY: `_is_unavailable_log_stream` (84–89): `OSError` with errno 5, or
/// `ValueError` containing "closed file". The ValueError branch has no Rust
/// analog — the port owns its writer handle and drops it instead of writing
/// to a closed file — so only EIO (raw error 5) classifies here.
/// PORT SEAMS: closed-file ValueError unrepresentable with `Box<dyn Write>`.
pub fn is_unavailable_log_stream(err: &std::io::Error) -> bool {
    err.raw_os_error() == Some(5)
}

/// Boxed writer behind the handler: `std::fs::File` in production, a
/// test-injected sick writer in the EIO oracle tests (Python monkeypatches
/// `handler.stream` / `handler._builtin_open` — no `std::fs::File` fault
/// injection exists, so the stream is a trait object).
type BoxedWriter = Box<dyn Write + Send>;

/// Open hook standing in for Python's `_builtin_open` monkeypatch
/// (`handler._builtin_open = lambda ...: _SickStream()`).
type OpenHook = Box<dyn Fn(&Path) -> std::io::Result<BoxedWriter> + Send + Sync>;

/// Formatter hook — PARITY: `handler.setFormatter(...)` (every Python
/// handler carries one; production uses `RedactingFormatter(_LOG_FORMAT)`,
/// tests often use `%(message)s`). `None` → `format_default()` (the
/// `_LOG_FORMAT` contract), which is what `setup_logging` installs.
pub type Formatter = std::sync::Arc<dyn Fn(&LogRecord) -> String + Send + Sync>;

/// A rotating file handler mirroring Python's `RotatingFileHandler` +
/// `_ManagedRotatingFileHandler` extensions (inode reopen + managed chmod +
/// unavailable-stream recovery).
///
/// All writes happen on the queue worker thread, so `emit` logic here runs
/// single-threaded; the `Mutex` guards the test-facing snapshot API.
pub struct RotatingHandler {
    pub path: PathBuf,
    pub level: Level,
    pub(crate) max_bytes: u64,
    pub(crate) backup_count: usize,
    pub component: Option<ComponentFilter>,
    /// Set once before the handler is shared (`Arc::new`), like Python's
    /// `setFormatter` during construction.
    formatter: Option<Formatter>,
    state: Mutex<WriterState>,
}

struct WriterState {
    file: Option<BoxedWriter>,
    current_size: u64,
    dev_ino: Option<(u64, u64)>,
    /// PARITY: `_unavailable_reported` (308) — one stderr notice per outage.
    unavailable_reported: bool,
    /// Total unavailable notices printed (oracle `err.count(path) == 1`).
    unavailable_report_count: u32,
    open_hook: Option<OpenHook>,
}

impl RotatingHandler {
    pub fn new(
        path: impl AsRef<Path>,
        level: Level,
        max_bytes: u64,
        backup_count: usize,
        component: Option<ComponentFilter>,
    ) -> std::io::Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let handler = RotatingHandler {
            path: path.as_ref().to_path_buf(),
            level,
            max_bytes,
            backup_count,
            component,
            formatter: None,
            state: Mutex::new(WriterState {
                file: None,
                current_size: 0,
                dev_ino: None,
                unavailable_reported: false,
                unavailable_report_count: 0,
                open_hook: None,
            }),
        };
        // Open eagerly, mirroring Python's handler construction creating the
        // file immediately. Parse any existing size so rotation semantics
        // survive restarts.
        let mut state = handler.state.lock().unwrap_or_else(|p| p.into_inner());
        handler.open_stream(&mut state)?;
        handler.record_stream_stat(&mut state);
        drop(state);
        Ok(handler)
    }

    /// Set the record formatter — PARITY: `handler.setFormatter(...)`.
    /// Call before sharing the handler (Python's `setFormatter` runs during
    /// test setup, before emit).
    pub fn set_formatter(&mut self, formatter: Formatter) {
        self.formatter = Some(formatter);
    }

    pub(crate) fn formatter(&self) -> Option<&Formatter> {
        self.formatter.as_ref()
    }

    /// Format one line through the configured formatter (default:
    /// `_LOG_FORMAT`), then redact — PARITY: `RedactingFormatter` wrapping.
    fn format_line(&self, record: &LogRecord) -> String {
        let raw = match &self.formatter {
            Some(f) => f(record),
            None => record.format_default(),
        };
        crate::record::redact(&raw)
    }

    fn open_stream(&self, state: &mut WriterState) -> std::io::Result<()> {
        let (writer, size) = match &state.open_hook {
            Some(hook) => {
                // Test seam: sick-writer factories (PARITY: `_builtin_open`
                // monkeypatch). Size still comes from the real path when it
                // exists — construction created it eagerly.
                let writer = hook(&self.path)?;
                let size = std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
                (writer, size)
            }
            None => {
                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.path)?;
                let size = file.metadata()?.len();
                (Box::new(file) as BoxedWriter, size)
            }
        };
        state.file = Some(writer);
        state.current_size = size;
        Ok(())
    }

    fn record_stream_stat(&self, state: &mut WriterState) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if let Ok(st) = std::fs::metadata(&self.path) {
                state.dev_ino = Some((st.dev(), st.ino()));
                return;
            }
        }
        state.dev_ino = None;
    }

    /// Reopen when `baseFilename` was renamed/unlinked underneath us
    /// (external rotation: logrotate, manual mv, another process).
    ///
    /// PARITY: `_reopen_if_externally_rotated` (342–359) — silent +
    /// best-effort; a stat failure keeps the existing stream.
    fn reopen_if_externally_rotated(&self, state: &mut WriterState) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let st = match std::fs::metadata(&self.path) {
                Ok(st) => st,
                Err(_) => {
                    // File missing: close and reopen (creates a fresh inode).
                    state.file = None;
                    let _ = self.open_stream(state);
                    self.record_stream_stat(state);
                    return;
                }
            };
            let cur = (st.dev(), st.ino());
            match state.dev_ino {
                Some(prev) if prev == cur => {}
                _ => {
                    // Different inode: close and reopen.
                    state.file = None;
                    let _ = self.open_stream(state);
                    state.dev_ino = Some(cur);
                }
            }
        }
        #[cfg(not(unix))]
        {
            let _ = state;
        }
    }

    /// Roll over like Python's RotatingFileHandler.doRollover():
    /// shift `.N` files up, then rename base to `.1`, reopen base.
    fn do_rollover(&self, state: &mut WriterState) {
        let _ = state.file.take(); // close current stream
        for i in (1..self.backup_count).rev() {
            let src = self.rotated_path(i);
            let dst = self.rotated_path(i + 1);
            if src.exists() {
                let _ = std::fs::rename(&src, &dst);
            }
        }
        let first = self.rotated_path(1);
        if self.path.exists() {
            let _ = std::fs::rename(&self.path, &first);
        }
        let _ = self.open_stream(state);
        // Managed-mode chmod (0o660): upstream only applies in managed/NixOS
        // deployments (config `is_managed()`). For a non-managed default this
        // is a no-op; the managed hook lands with the config crate (P1/P3).
        self.record_stream_stat(state);
    }

    fn rotated_path(&self, n: usize) -> PathBuf {
        // Python: baseFilename + "." + suffix
        let mut os: std::ffi::OsString = self.path.as_os_str().to_os_string();
        os.push(format!(".{}", n));
        PathBuf::from(os)
    }

    /// Dispatch a record through level + component filters — always returns
    /// whether the record passed (so the worker can count dropped records).
    pub fn accepts_record(&self, record: &LogRecord) -> bool {
        if record.level < self.level {
            return false;
        }
        if let Some(filter) = &self.component {
            if !filter.matches(&record.target) {
                return false;
            }
        }
        true
    }

    /// Write a formatted line (worker thread only).
    ///
    /// PARITY: `_ManagedRotatingFileHandler.emit` (361–370) +
    /// `FileHandler.emit`'s lazy open (CPython: stream None → `_open()`) +
    /// `RotatingFileHandler.shouldRollover` (live size via seek/tell — here a
    /// metadata refresh, because `Box<dyn Write>` is not seekable).
    pub fn emit_record(&self, record: &LogRecord) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());

        if state.file.is_none() {
            // Lazy open: recovery after an unavailable-stream drop (and the
            // post-rollover open-failure gap). PARITY: FileHandler.emit
            // `if self.stream is None: self.stream = self._open()`.
            if let Err(e) = self.open_stream(&mut state) {
                self.handle_emit_error(&mut state, &e);
                return;
            }
            self.record_stream_stat(&mut state);
        } else {
            self.reopen_if_externally_rotated(&mut state);
            // Live size (shouldRollover seek/tell analog): picks up external
            // truncation (`: > file`) and foreign appends on the same inode.
            if state.file.is_some() {
                if let Ok(md) = std::fs::metadata(&self.path) {
                    state.current_size = md.len();
                }
            }
        }

        let line = self.format_line(record);
        let line_len = line.len() as u64 + 1; // + newline (msg + terminator)

        // PARITY: `shouldRollover` — `pos + len(msg) >= maxBytes`, and never
        // roll an empty file (`if not pos: return False`, gh-116263); live
        // probe @ 5d59366: 9×5-byte lines fill a 50-byte file exactly, the
        // 10th triggers rollover before writing.
        if self.max_bytes > 0
            && state.current_size > 0
            && state.current_size + line_len >= self.max_bytes
        {
            self.do_rollover(&mut state);
        }

        let write_err: Option<std::io::Error> = match state.file.as_mut() {
            Some(f) => match writeln!(f, "{}", line).and_then(|()| f.flush()) {
                Ok(()) => {
                    state.current_size += line_len;
                    None
                }
                Err(e) => Some(e),
            },
            // Open failed inside do_rollover (its error is swallowed like
            // Python's best-effort `_open` in `_reopen_stream`); the write
            // itself then has no stream — handleError territory.
            None => Some(std::io::Error::other("log stream not open")),
        };
        if let Some(e) = write_err {
            self.handle_emit_error(&mut state, &e);
        }

        // PARITY: emit tail `if self.stream is not None:
        // self._unavailable_reported = False` (366–370) — reset only when a
        // record actually left a live stream, never inside `_open()` (an
        // open-then-EIO device would re-arm the notice per record).
        if state.file.is_some() {
            state.unavailable_reported = false;
        }
    }

    /// PARITY: `handleError` (372–395).
    fn handle_emit_error(&self, state: &mut WriterState, e: &std::io::Error) {
        if is_windows_concurrent_log_lock_timeout(&e.to_string()) {
            // Windows CLH lock timeout suppressed before stderr (inert on
            // POSIX — `cfg!(windows)` is false in this build's oracle path).
            return;
        }
        if is_unavailable_log_stream(e) {
            // Name the path once, drop the stale stream; the next emit
            // reopens it if the destination has recovered.
            if !state.unavailable_reported {
                state.unavailable_reported = true;
                state.unavailable_report_count += 1;
                eprintln!(
                    "hermes_logging: {} unavailable ({}); file logging paused until it recovers",
                    self.path.display(),
                    e
                );
            }
            state.file = None;
            return;
        }
        // PARITY: `super().handleError(record)` prints "--- Logging error ---"
        // + traceback to `_safe_stderr()`. Rust prints the same marker +
        // the io::Error Display (no Python traceback analog).
        // PORT SEAMS: `_safe_stderr` is unneeded — Rust's stderr is
        // Unicode-native on every platform (the cp949 wrap targets Python's
        // text layer only).
        eprintln!("--- Logging error ---\n{}: {}", self.path.display(), e);
    }

    /// PARITY test seam: replace the live writer (`handler.stream = ...`).
    #[doc(hidden)]
    pub fn inject_writer_for_tests(&self, writer: Option<BoxedWriter>) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state.file = writer;
    }

    /// PARITY test seam: replace `_builtin_open`.
    #[doc(hidden)]
    pub fn set_open_hook_for_tests(&self, hook: Option<OpenHook>) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state.open_hook = hook;
    }

    /// Whether a writer is currently held (`handler.stream is not None`).
    #[doc(hidden)]
    pub fn stream_open(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .file
            .is_some()
    }

    /// Total unavailable-stream notices printed (oracle `err.count(path)`).
    #[doc(hidden)]
    pub fn unavailable_report_count(&self) -> u32 {
        self.state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .unavailable_report_count
    }
}

impl crate::record::LogTarget for RotatingHandler {
    fn accepts(&self, record: &LogRecord) -> bool {
        self.accepts_record(record)
    }

    fn emit(&self, record: &LogRecord) {
        self.emit_record(record)
    }
}

/// Registry of live rotating file handlers (mirrors `_queued_file_handlers`).
pub type HandlerList = Vec<std::sync::Arc<RotatingHandler>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::LogRecord;

    fn rec(level: Level, target: &str, msg: &str) -> LogRecord {
        LogRecord::new(level, target, msg)
    }

    #[test]
    fn component_filter_prefix_matching() {
        let f = ComponentFilter {
            prefixes: vec!["gateway".into(), "plugins.platforms".into()],
        };
        assert!(f.matches("gateway.run"));
        assert!(f.matches("plugins.platforms.telegram.adapter"));
        assert!(!f.matches("agent.runtime"));
    }

    #[test]
    fn level_and_component_filtering() {
        let td = tempfile::TempDir::new().unwrap();
        let h = RotatingHandler::new(
            td.path().join("agent.log"),
            Level::Info,
            1024 * 1024,
            3,
            None,
        )
        .unwrap();
        assert!(!h.accepts_record(&rec(Level::Debug, "x", "m")));
        assert!(h.accepts_record(&rec(Level::Info, "x", "m")));
        assert!(h.accepts_record(&rec(Level::Warning, "x", "m")));
    }

    #[test]
    fn writes_and_rotates() {
        let td = tempfile::TempDir::new().unwrap();
        let path = td.path().join("test.log");
        let h = RotatingHandler::new(&path, Level::Debug, 200, 2, None).unwrap();
        for i in 0..30 {
            h.emit_record(&rec(Level::Info, "t", &format!("message {}", i)));
        }
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("message 29"), "last line written: {}", text);
        // Rotation happened: .1 backup exists.
        let backup = td.path().join("test.log.1");
        assert!(backup.exists(), "backup exists after rotation");
    }

    #[test]
    fn keeps_backup_count_bounded() {
        let td = tempfile::TempDir::new().unwrap();
        let path = td.path().join("b.log");
        let h = RotatingHandler::new(&path, Level::Debug, 100, 2, None).unwrap();
        for i in 0..100 {
            h.emit_record(&rec(Level::Info, "t", &format!("m {:04}", i)));
        }
        assert!(
            !td.path().join("b.log.3").exists(),
            "backup_count=2 keeps at most .2"
        );
        assert!(td.path().join("b.log.1").exists());
    }

    #[test]
    fn rollover_boundary_is_ge_and_skips_empty_file() {
        // Live-oracle probe @ 5d59366: 5-byte lines into a 50-byte cap —
        // exactly 45 bytes (9 lines) sit in .1, the 10th write triggers
        // rollover BEFORE writing (pos+len >= maxBytes). A single line
        // larger than max on an EMPTY file never rolls over (gh-116263).
        // Message-only formatter matches the oracle's `%(message)s` setup.
        let msg_only: Formatter = std::sync::Arc::new(|r: &LogRecord| r.message.clone());
        let td = tempfile::TempDir::new().unwrap();
        let path = td.path().join("edge.log");
        let mut h = RotatingHandler::new(&path, Level::Debug, 50, 1, None).unwrap();
        h.set_formatter(msg_only.clone());
        for _ in 0..12 {
            h.emit_record(&rec(Level::Info, "t", "xxxx"));
        }
        let main = std::fs::read_to_string(&path).unwrap();
        let backup = std::fs::read_to_string(td.path().join("edge.log.1")).unwrap();
        assert_eq!(backup.matches("xxxx\n").count(), 9, "backup: {:?}", backup);
        assert_eq!(main.matches("xxxx\n").count(), 3, "main: {:?}", main);

        // Empty-file guard: one oversized line writes through, no .1 yet.
        let td2 = tempfile::TempDir::new().unwrap();
        let big = td2.path().join("big.log");
        let mut h2 = RotatingHandler::new(&big, Level::Debug, 5, 1, None).unwrap();
        h2.set_formatter(msg_only);
        h2.emit_record(&rec(Level::Info, "t", "0123456789")); // 11 bytes > 5
        assert!(
            !td2.path().join("big.log.1").exists(),
            "no rollover from empty"
        );
        assert_eq!(std::fs::read_to_string(&big).unwrap(), "0123456789\n");
        // Next line: file non-empty and over cap → rolls over.
        h2.emit_record(&rec(Level::Info, "t", "next"));
        assert!(
            td2.path().join("big.log.1").exists(),
            "rolls once non-empty"
        );
        assert_eq!(std::fs::read_to_string(&big).unwrap(), "next\n");
    }

    #[test]
    fn unavailable_stream_error_classification() {
        // PARITY: linux_only `test_helper_never_matches_off_windows` — the
        // CLH helper must stay inert on POSIX.
        assert!(!is_windows_concurrent_log_lock_timeout(
            "Cannot acquire lock after 20 attempts"
        ));
        // PARITY: `_is_unavailable_log_stream` — EIO matches, ENOSPC doesn't.
        assert!(is_unavailable_log_stream(
            &std::io::Error::from_raw_os_error(5)
        ));
        assert!(!is_unavailable_log_stream(
            &std::io::Error::from_raw_os_error(28)
        ));
    }
}
