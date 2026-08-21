//! Rotating file handler with managed-mode chmod and external-rotation
//! inode detection.
//!
//! PARITY: hermes_logging.py `_ManagedRotatingFileHandler` (415–572).

use crate::record::{Level, LogRecord};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Optional component filter: only pass records whose target starts with one
/// of the prefixes.
///
/// PARITY: `_ComponentFilter` (219–233) + `COMPONENT_PREFIXES` (236–257).
#[derive(Debug, Clone, Default)]
pub struct ComponentFilter {
    pub prefixes: Vec<String>,
}

impl ComponentFilter {
    pub fn matches(&self, target: &str) -> bool {
        self.prefixes.iter().any(|p| target.starts_with(p.as_str()))
    }
}

/// A rotating file handler mirroring Python's `RotatingFileHandler` +
/// `_ManagedRotatingFileHandler` extensions (inode reopen + managed chmod).
///
/// All writes happen on the queue worker thread, so `emit` logic here runs
/// single-threaded; the `Mutex` guards the test-facing snapshot API.
pub struct RotatingHandler {
    pub path: PathBuf,
    pub level: Level,
    max_bytes: u64,
    backup_count: usize,
    pub component: Option<ComponentFilter>,
    state: Mutex<WriterState>,
}

struct WriterState {
    file: Option<File>,
    current_size: u64,
    dev_ino: Option<(u64, u64)>,
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
            state: Mutex::new(WriterState {
                file: None,
                current_size: 0,
                dev_ino: None,
            }),
        };
        // Open eagerly, mirroring Python's handler construction creating the
        // file immediately. Parse any existing size so rotation semantics
        // survive restarts.
        let mut state = handler.state.lock().unwrap();
        handler.open_stream(&mut state)?;
        handler.record_stream_stat(&mut state);
        drop(state);
        Ok(handler)
    }

    fn open_stream(&self, state: &mut WriterState) -> std::io::Result<()> {
        let file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        let size = file.metadata()?.len();
        state.file = Some(file);
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
    pub fn emit_record(&self, record: &LogRecord) {
        let mut state = self.state.lock().unwrap();
        self.reopen_if_externally_rotated(&mut state);

        let line = crate::record::redact(&record.format_default());
        let line_len = line.len() as u64 + 1; // + newline

        if state.current_size + line_len > self.max_bytes && self.max_bytes > 0 {
            self.do_rollover(&mut state);
        }
        {
            let file = state.file.as_mut();
            if let Some(f) = file {
                let _ = writeln!(f, "{}", line);
                let _ = f.flush();
            }
        }
        state.current_size += line_len;
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
        let f = ComponentFilter { prefixes: vec!["gateway".into(), "plugins.platforms".into()] };
        assert!(f.matches("gateway.run"));
        assert!(f.matches("plugins.platforms.telegram.adapter"));
        assert!(!f.matches("agent.runtime"));
    }

    #[test]
    fn level_and_component_filtering() {
        let td = tempfile::TempDir::new().unwrap();
        let h = RotatingHandler::new(td.path().join("agent.log"), Level::Info, 1024 * 1024, 3, None).unwrap();
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
        assert!(!td.path().join("b.log.3").exists(), "backup_count=2 keeps at most .2");
        assert!(td.path().join("b.log.1").exists());
    }
}
