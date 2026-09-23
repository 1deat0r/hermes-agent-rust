//! Oracle mirrors for `tests/test_hermes_logging.py` recovery behaviors @
//! 5d59366 (TestExternalRotationRecovery ×3, EIO name-once ×2, unicode
//! emit).
//!
//! The EIO tests replace Python's `handler.stream` /
//! `handler._builtin_open` monkeypatches with the port's writer/open-hook
//! test seams — `std::fs::File` cannot be fault-injected directly. The
//! oracle's stderr assertions (`err.count(path) == 1`, no
//! `--- Logging error ---`) map to `unavailable_report_count()`: the
//! unavailable branch prints once and returns before the generic
//! `--- Logging error ---` path (structural; stderr is not capturable per-
//! test under libtest without racing the harness).
//!
//! Skipped upstream cases (documented in PLAN.md §7):
//! - windows_only CLH rows — no concurrent-log-handler on POSIX (linux_only
//!   inert row lives in the rotating unit tests).
//! - `TestSafeStderr.test_wraps_non_utf8_stderr` — `_safe_stderr` has no
//!   Rust analog (Rust stderr is Unicode-native); the unicode-emit row is
//!   mirrored below on the file side.

use hermes_logging::{
    log, setup::reset_logging_for_tests, setup_logging, Level, LogRecord, RotatingHandler,
    SetupOptions,
};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

static M: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    M.lock().unwrap_or_else(|p| p.into_inner())
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn msg_only() -> hermes_logging::rotating::Formatter {
    std::sync::Arc::new(|r: &LogRecord| r.message.clone())
}

fn rec(msg: &str) -> LogRecord {
    LogRecord::new(Level::Info, "gateway.run", msg)
}

fn make_handler(path: &Path) -> RotatingHandler {
    let mut h = RotatingHandler::new(path, Level::Info, 10 * 1024 * 1024, 3, None).unwrap();
    h.set_formatter(msg_only());
    h
}

/// Writer that always fails with EIO (errno 5) — PARITY: `_SickStream`
/// (690–696) raising `OSError(5, "Input/output error")`.
struct SickWriter;

impl Write for SickWriter {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(5))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::from_raw_os_error(5))
    }
}

#[test]
fn recovers_after_external_rename() {
    // PARITY: test_recovers_after_external_rename (592–618) — logrotate-
    // style rename; new write recreates the path, not the backup.
    let _g = lock();
    let td = tempfile::TempDir::new().unwrap();
    let path = td.path().join("gateway.log");
    let rotated = td.path().join("gateway.log.1");
    let h = make_handler(&path);

    h.emit_record(&rec("before rotation"));
    assert_eq!(read(&path), "before rotation\n");

    std::fs::rename(&path, &rotated).unwrap();
    assert!(!path.exists());

    h.emit_record(&rec("after rotation"));
    assert!(path.exists(), "handler did not recreate gateway.log");
    assert_eq!(read(&path), "after rotation\n");
    assert_eq!(read(&rotated), "before rotation\n");
}

#[test]
fn external_truncate_does_not_force_reopen() {
    // PARITY: test_external_truncate_does_not_force_reopen (621–643) —
    // same inode: content shrinks in place, writes continue on the fd.
    let _g = lock();
    let td = tempfile::TempDir::new().unwrap();
    let path = td.path().join("gateway.log");
    let h = make_handler(&path);

    h.emit_record(&rec(&"A".repeat(128)));
    assert!(std::fs::metadata(&path).unwrap().len() > 0);

    // Truncate in place (same inode) — `: > gateway.log`.
    let f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    drop(f);
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);

    h.emit_record(&rec("after truncate"));
    assert_eq!(read(&path), "after truncate\n");
}

#[test]
fn gateway_log_attached_after_external_rotation_then_re_setup() {
    // PARITY: test_gateway_log_attached_after_external_rotation_then_re_setup
    // (646–682) — Allen repro: external rename, setup re-entered (dedup
    // no-ops on the resolved-but-missing path), the live handler reopens and
    // the new record lands in the recreated file, not the backup.
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path().to_path_buf();

    setup_logging(SetupOptions {
        hermes_home: Some(home.clone()),
        mode: Some("gateway".to_string()),
        ..Default::default()
    });
    let gw_path = home.join("logs/gateway.log");
    let rotated = home.join("logs/gateway.log.1");

    log(Level::Info, "gateway.run", "line BEFORE rotation");
    flush();
    assert!(read(&gw_path).contains("BEFORE rotation"));

    std::fs::rename(&gw_path, &rotated).unwrap();
    assert!(!gw_path.exists());

    // Re-enter setup: per-path dedup (resolve_tolerant, no fs hit) no-ops;
    // the existing handler's inode detection recreates the file on emit.
    setup_logging(SetupOptions {
        hermes_home: Some(home.clone()),
        mode: Some("gateway".to_string()),
        ..Default::default()
    });

    log(Level::Info, "gateway.run", "line AFTER rotation");
    flush();

    assert!(gw_path.exists(), "gateway.log was never recreated");
    assert!(read(&gw_path).contains("AFTER rotation"));
    assert!(!read(&rotated).contains("AFTER rotation"));
    reset_logging_for_tests();
}

fn flush() {
    hermes_logging::flush_log_queue();
}

#[test]
fn eio_from_file_handler_names_the_path_once_then_recovers() {
    // PARITY: test_eio_from_file_handler_names_the_path_once_then_recovers
    // (685–716): five sick writes report once (stream dropped after the
    // first), then the next emit reopens the real file and logging resumes.
    let _g = lock();
    let td = tempfile::TempDir::new().unwrap();
    let path = td.path().join("agent.log");
    let h = make_handler(&path);
    h.inject_writer_for_tests(Some(Box::new(SickWriter)));

    for i in 0..5 {
        h.emit_record(&rec(&format!("sick {}", i)));
    }
    assert_eq!(
        h.unavailable_report_count(),
        1,
        "path named exactly once across 5 records (oracle err.count(path))"
    );
    // Record 1 drops the sick stream; records 2–5 lazy-reopen the real file
    // (PARITY: FileHandler.emit `stream is None → _open()`), so the handler
    // is writing normally again by the end of the loop — exactly upstream's
    // control flow. Recovery is then the final record:
    assert!(h.stream_open(), "real stream reopened after the notice");

    h.emit_record(&rec("recovered"));
    assert!(
        read(&path).contains("recovered"),
        "recovery write must land in the real file"
    );
    assert_eq!(
        h.unavailable_report_count(),
        1,
        "successful reopen must not re-arm the notice"
    );
    // The sick first record never reached disk; 2–5 + recovered did.
    let content = read(&path);
    assert!(
        !content.contains("sick 0"),
        "failed write not on disk: {}",
        content
    );
    assert!(
        content.contains("sick 1"),
        "later sick records recovered: {}",
        content
    );
}

#[test]
fn eio_after_successful_reopen_still_names_the_path_once() {
    // PARITY: test_eio_after_successful_reopen_still_names_the_path_once
    // (720–749): open() itself yields a sick stream (`_builtin_open`
    // monkeypatch) — 25 records, still exactly one notice.
    let _g = lock();
    let td = tempfile::TempDir::new().unwrap();
    let path = td.path().join("agent.log");
    let h = make_handler(&path);
    h.set_open_hook_for_tests(Some(Box::new(|_p: &Path| {
        Ok(Box::new(SickWriter) as Box<dyn Write + Send>)
    })));
    h.inject_writer_for_tests(Some(Box::new(SickWriter)));

    for i in 0..25 {
        h.emit_record(&rec(&format!("sick {}", i)));
    }
    assert_eq!(h.unavailable_report_count(), 1, "stuck device names once");
    assert!(!h.stream_open());
}

#[test]
fn unicode_emit_does_not_crash() {
    // PARITY: test_handler_emits_unicode_without_crash (779–811) — the
    // em-dash from the original bug report must survive the file path (the
    // stderr-wrapper half has no Rust analog; see file header).
    let _g = lock();
    let td = tempfile::TempDir::new().unwrap();
    let path = td.path().join("unicode.log");
    let h = make_handler(&path);
    h.emit_record(&rec("Session hygiene: 400 messages — auto-compressing"));
    assert!(read(&path).contains('—'));
}
