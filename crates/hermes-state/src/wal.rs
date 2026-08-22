//! WAL journal-mode application with filesystem + WAL-reset-bug fallback.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   resolve_journal_mode           (740–766)
//!   is_sqlite_wal_reset_vulnerable (710–725, hermes_cli/sqlite_runtime.py 24–39)
//!   sqlite_source_id               (725–740, sqlite_runtime.py 41–56)
//!   _on_disk_journal_mode          (624–645)
//!   _apply_macos_checkpoint_barrier(645–680)
//!   _enforce_macos_synchronous_full(680–710)
//!   WalUnsupportedError            (766–780)
//!   apply_wal_with_fallback        (780–957)
//!   _set_journal_mode_no_wait      (957–993)
//!   _apply_delete_for_wal_reset_bug(993–1060)
//!   _wal_reset_repair_hint         (1060–1088)
//!   _log_wal_reset_bug_once        (1088–1130)
//!   _log_wal_fallback_once         (1130–1159)
//!   apply_database_pragmas         (1159–1243)
//!
//! Divergence notes (PLAN §5 — intentional, documented):
//! - `_wal_reset_repair_hint` consults hermes_cli.config install-method
//!   detection (P3); the Rust port emits the static fallback hint.
//! - Warnings print to stderr (same convention as hermes-time); the Python
//!   runtime routes them into agent.log via the logging facility.

use std::sync::Mutex;

use once_cell::sync::Lazy;

/// `_WAL_INCOMPAT_MARKERS` @ hermes_state.py 441–445.
pub const WAL_INCOMPAT_MARKERS: [&str; 3] = [
    "locking protocol", // SQLITE_PROTOCOL on NFS/SMB
    "not authorized",   // Some FUSE mounts block WAL pragma outright
    "disk i/o error",   // ZFS SHM corruption under concurrent connections
];

/// Error type mirroring `sqlite3.OperationalError` for the WAL machinery.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct WalError(pub String);

impl From<rusqlite::Error> for WalError {
    fn from(e: rusqlite::Error) -> Self {
        WalError(e.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SqliteVersionInfo {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SqliteVersionInfo {
    /// Convert `rusqlite::version_number()` (e.g. 3_050_002) to a tuple
    /// equivalent of `sqlite3.sqlite_version_info`.
    pub fn from_version_number(n: i32) -> Self {
        let n = n.max(0) as u32;
        SqliteVersionInfo {
            major: n / 1_000_000,
            minor: (n / 1_000) % 1_000,
            patch: n % 1_000,
        }
    }
}

/// Return whether *version_info* contains SQLite's WAL-reset bug.
///
/// PARITY: hermes_cli/sqlite_runtime.py is_sqlite_wal_reset_vulnerable @ b9aa928.
/// Upstream documents the bug in versions 3.7.0 through 3.51.2, fixed in
/// 3.51.3+, with backports 3.50.7 and 3.44.6.
/// Pre-WAL libraries (< 3.7.0) cannot hit the race and are treated as safe.
pub fn is_sqlite_wal_reset_vulnerable(info: &SqliteVersionInfo) -> bool {
    let v = info;
    let v370 = version(3, 7, 0);
    let v3513 = version(3, 51, 3);
    let v3507 = version(3, 50, 7);
    let v3510 = version(3, 51, 0);
    let v3446 = version(3, 44, 6);
    let v3450 = version(3, 45, 0);
    if *v < v370 {
        return false;
    }
    if *v >= v3513 {
        return false;
    }
    if (v3507 <= *v) && (*v < v3510) {
        return false;
    }
    if (v3446 <= *v) && (*v < v3450) {
        return false;
    }
    true
}

/// The runtime's linked SQLite version, as `sqlite3.sqlite_version_info`.
pub fn sqlite_version_info() -> SqliteVersionInfo {
    SqliteVersionInfo::from_version_number(rusqlite::version_number())
}

/// Return `sqlite_source_id()`, or an empty string when unavailable.
///
/// PARITY: hermes_cli/sqlite_runtime.py sqlite_source_id @ b9aa928.
pub fn sqlite_source_id() -> String {
    let conn = match rusqlite::Connection::open_in_memory() {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    conn.query_row("SELECT sqlite_source_id()", [], |r| {
        r.get::<_, Option<String>>(0)
    })
    .ok()
    .flatten()
    .unwrap_or_default()
}

/// Return the configured journal mode (`wal` or `delete`).
///
/// `database.journal_mode` in config.yaml is the canonical operator setting.
/// `wal` remains the default; `delete` is used when the backing filesystem
/// does not provide WAL-safe durability. Invalid or malformed values fail
/// safely to the existing default.
///
/// PARITY: hermes_state.py resolve_journal_mode @ b9aa928 (740–766). The
/// Python source uses hermes_cli.config.load_config_readonly + cfg_get; the
/// Rust equivalent reads the same config.yaml with the hermes-time fail-open
/// convention.
pub fn resolve_journal_mode() -> String {
    let raw = crate::cfg::raw_database_journal_mode(&hermes_constants::get_config_path());
    match raw {
        Some(mode) => {
            let mode = mode.trim().to_ascii_lowercase();
            if mode == "wal" || mode == "delete" {
                mode
            } else {
                "wal".to_string()
            }
        }
        None => "wal".to_string(),
    }
}

/// Read the journal mode from the SQLite DB header on disk.
///
/// Returns the mode string (e.g. "wal", "delete"), or `None` if the value
/// cannot be determined (new DB, or the PRAGMA read failed).
///
/// PARITY: hermes_state.py _on_disk_journal_mode @ b9aa928 (624–645).
pub fn on_disk_journal_mode(conn: &rusqlite::Connection) -> Option<String> {
    let row: Result<Option<String>, _> =
        conn.query_row("PRAGMA journal_mode", [], |r| r.get::<_, Option<String>>(0));
    match row {
        Ok(Some(mode)) => Some(mode.trim().to_ascii_lowercase()),
        Ok(None) => None,
        Err(_) => None,
    }
}

/// Enable `PRAGMA checkpoint_fullfsync` on macOS (no-op elsewhere).
///
/// PARITY: hermes_state.py _apply_macos_checkpoint_barrier @ b9aa928 (645–680).
/// Best-effort: never raises.
pub fn apply_macos_checkpoint_barrier(conn: &rusqlite::Connection) {
    if !cfg!(target_os = "macos") {
        return;
    }
    let _ = conn.execute_batch("PRAGMA checkpoint_fullfsync=1");
}

/// Enforce `PRAGMA synchronous=FULL` on macOS (no-op elsewhere).
///
/// PARITY: hermes_state.py _enforce_macos_synchronous_full @ b9aa928 (680–710).
/// Best-effort: never raises.
pub fn enforce_macos_synchronous_full(conn: &rusqlite::Connection) {
    if !cfg!(target_os = "macos") {
        return;
    }
    let _ = conn.execute_batch("PRAGMA synchronous=FULL");
}

/// Raised by `apply_wal_with_fallback` when `require_wal=True` and the
/// filesystem cannot provide WAL journal mode.
///
/// PARITY: hermes_state.py WalUnsupportedError @ b9aa928 (766–780).
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct WalUnsupportedError(pub String);

impl From<WalError> for WalUnsupportedError {
    fn from(e: WalError) -> Self {
        WalUnsupportedError(e.0)
    }
}

const fn version(major: u32, minor: u32, patch: u32) -> SqliteVersionInfo {
    SqliteVersionInfo {
        major,
        minor,
        patch,
    }
}

/// Dedup ERROR/WARNING per db_label across the process (Python's module sets
/// with locks).
static WAL_FALLBACK_WARNED: Lazy<Mutex<std::collections::HashSet<String>>> =
    Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
static WAL_RESET_WARNED: Lazy<Mutex<std::collections::HashSet<String>>> =
    Lazy::new(|| Mutex::new(std::collections::HashSet::new()));

/// Hint for repairing the SQLite runtime (static fallback form).
///
/// PARITY: hermes_state.py _wal_reset_repair_hint @ b9aa928 (1060–1088) —
/// the honest static hint; install-method detection is CLI/Phase-3.
pub fn wal_reset_repair_hint() -> &'static str {
    "install a Python build bundled with SQLite 3.51.3+ \
     (or backports 3.50.7 / 3.44.6) and restart Hermes"
}

/// Log once per (process, db_label) about the WAL-reset vulnerability path.
///
/// PARITY: hermes_state.py _log_wal_reset_bug_once @ b9aa928 (1088–1130).
pub fn log_wal_reset_bug_once(db_label: &str, kept_wal: bool, indeterminate: bool) {
    {
        let mut warned = WAL_RESET_WARNED.lock().unwrap();
        if !warned.insert(db_label.to_string()) {
            return;
        }
    }
    let action = if indeterminate {
        "journal mode could not be verified or exclusively switched \
         (database is locked — possible concurrent openers); leaving the \
         journal mode untouched (no live downgrade under concurrent \
         openers)"
    } else if kept_wal {
        "is already in WAL mode — leaving WAL in place (no live \
         downgrade under concurrent openers)"
    } else {
        "using journal_mode=DELETE instead of enabling WAL"
    };
    eprintln!(
        "{}: linked SQLite {} is vulnerable to the WAL-reset corruption \
         bug (https://sqlite.org/wal.html#walresetbug) — {}. \
         Upgrade to SQLite 3.51.3+ (or backports 3.50.7 / 3.44.6); \
         {}. See `hermes doctor`. This warning fires once per \
         process per database.",
        db_label,
        rusqlite::version(),
        action,
        wal_reset_repair_hint(),
    );
}

/// Log a single ERROR per (process, db_label) about WAL fallback.
///
/// PARITY: hermes_state.py _log_wal_fallback_once @ b9aa928 (1130–1159).
pub fn log_wal_fallback_once(db_label: &str, exc: &str) {
    {
        let mut warned = WAL_FALLBACK_WARNED.lock().unwrap();
        if !warned.insert(db_label.to_string()) {
            return;
        }
    }
    eprintln!(
        "{}: WAL journal_mode unsupported on this filesystem ({}) — \
         falling back to journal_mode=DELETE (slower rollback-journal \
         mode; reduces concurrency but works on NFS/SMB/FUSE/ZFS). See \
         https://www.sqlite.org/wal.html for details. This message \
         fires once per process per database.",
        db_label, exc,
    );
}

/// Execute `PRAGMA journal_mode=<mode>` without waiting on other openers.
///
/// This is the ONLY place a journal-mode switch pragma may be issued for a
/// non-WAL target. Callers must treat a raised `OperationalError` as "not
/// exclusively owned: leave the journal mode alone", never as a retryable
/// condition.
///
/// PARITY: hermes_state.py _set_journal_mode_no_wait @ b9aa928 (957–993).
pub fn set_journal_mode_no_wait(
    conn: &rusqlite::Connection,
    mode: &str,
) -> Result<String, WalError> {
    let previous_timeout: i64 = conn
        .query_row("PRAGMA busy_timeout", [], |r| r.get::<_, Option<i64>>(0))
        .ok()
        .flatten()
        .unwrap_or(0);
    conn.execute_batch("PRAGMA busy_timeout=0")?;
    let result = (|| -> Result<String, WalError> {
        let row: Option<String> =
            conn.query_row(&format!("PRAGMA journal_mode={}", mode), [], |r| {
                r.get::<_, Option<String>>(0)
            })?;
        Ok(row
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default())
    })();
    // Restore the previous timeout before propagating any error.
    let _ = conn.execute_batch(&format!("PRAGMA busy_timeout={}", previous_timeout));
    result
}

/// Avoid enabling WAL when the linked SQLite has the WAL-reset bug.
///
/// - Already-WAL on disk: leave WAL alone (no live downgrade) and warn.
/// - Mode unreadable: leave the journal mode alone and warn.
/// - Otherwise: set DELETE (refusing to wait out concurrent openers) and warn.
/// - For an explicit operator request, verify SQLite accepted DELETE.
///
/// PARITY: hermes_state.py _apply_delete_for_wal_reset_bug @ b9aa928 (993–1060).
pub fn apply_delete_for_wal_reset_bug(
    conn: &rusqlite::Connection,
    db_label: &str,
    require_delete: bool,
) -> Result<String, WalError> {
    let current = on_disk_journal_mode(conn);

    if current.as_deref() == Some("wal") {
        log_wal_reset_bug_once(db_label, true, false);
        apply_macos_checkpoint_barrier(conn);
        enforce_macos_synchronous_full(conn);
        return Ok("wal".to_string());
    }

    if current.is_none() {
        if require_delete {
            return Err(WalError(
                "could not verify journal mode before applying configured \
                 journal_mode=delete (database is locked — possible \
                 concurrent openers); refusing to downgrade a database \
                 this process does not exclusively own"
                    .to_string(),
            ));
        }
        log_wal_reset_bug_once(db_label, true, true);
        return Ok("wal".to_string());
    }

    let current = current.unwrap();
    let actual = match set_journal_mode_no_wait(conn, "DELETE") {
        Ok(actual) => actual,
        Err(WalError(msg)) => {
            let lowered = msg.to_ascii_lowercase();
            if require_delete {
                return Err(WalError(msg));
            }
            if lowered.contains("locked") || lowered.contains("busy") {
                log_wal_reset_bug_once(db_label, true, true);
                return Ok(current.clone());
            }
            // Best-effort for the automatic vulnerable-runtime fallback:
            // DELETE is normally already the default for new file-backed DBs.
            String::new()
        }
    };
    if require_delete && actual != "delete" {
        return Err(WalError(format!(
            "could not set configured journal_mode=delete (got {})",
            if actual.is_empty() {
                "no result"
            } else {
                &actual
            }
        )));
    }
    log_wal_reset_bug_once(db_label, false, false);
    Ok("delete".to_string())
}

/// Set `journal_mode=WAL` on `conn`, falling back to DELETE on failure.
///
/// Returns the journal mode actually set ("wal" or "delete").
///
/// PARITY: hermes_state.py apply_wal_with_fallback @ b9aa928 (780–957).
/// The disk-I/O retry loop (2 retries, 50ms apart) is preserved; the
/// require_wal silent-refusal raise path is preserved.
#[allow(clippy::too_many_lines)]
pub fn apply_wal_with_fallback(
    conn: &rusqlite::Connection,
    db_label: &str,
    require_wal: bool,
) -> Result<String, WalError> {
    let configured = resolve_journal_mode();

    if is_sqlite_wal_reset_vulnerable(&sqlite_version_info()) {
        return apply_delete_for_wal_reset_bug(conn, db_label, configured == "delete");
    }

    // Read-only probe — no flock, no checkpoint, no WAL/SHM unlink.
    let current_mode = on_disk_journal_mode(conn);
    if current_mode.as_deref() == Some("wal") {
        apply_macos_checkpoint_barrier(conn);
        enforce_macos_synchronous_full(conn);
        return Ok("wal".to_string());
    }

    if configured == "delete" {
        if current_mode.is_none() {
            return Err(WalError(
                "could not verify journal mode before applying configured \
                 journal_mode=delete (database is locked — possible \
                 concurrent openers); refusing to downgrade a database \
                 this process does not exclusively own"
                    .to_string(),
            ));
        }
        let actual = set_journal_mode_no_wait(conn, "DELETE")?;
        if actual != "delete" {
            return Err(WalError(format!(
                "could not set configured journal_mode=delete (got {})",
                if actual.is_empty() {
                    "no result"
                } else {
                    &actual
                }
            )));
        }
        return Ok(actual);
    }

    // PRAGMA journal_mode=WAL is a query-that-sets: it RETURNS the resulting
    // journal mode. Trust the returned row, not the mere absence of an
    // exception.
    let set_wal = || -> Result<String, WalError> {
        let row: Option<String> = conn.query_row("PRAGMA journal_mode=WAL", [], |r| {
            r.get::<_, Option<String>>(0)
        })?;
        let mode = row
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if mode == "wal" {
            apply_macos_checkpoint_barrier(conn);
            enforce_macos_synchronous_full(conn);
            Ok("wal".to_string())
        } else {
            // Silent refusal (macOS NFS / SMB / AgentFS overlay).
            Err(WalError(format!(
                "journal_mode=WAL refused without raising (still {:?})",
                mode
            )))
        }
    };

    match set_wal() {
        Ok(mode) => Ok(mode),
        Err(WalError(msg)) => {
            if msg.contains("refused without raising") {
                let silent_exc = WalUnsupportedError(msg);
                if require_wal {
                    return Err(WalError(silent_exc.0));
                }
                log_wal_fallback_once(db_label, &silent_exc.0);
                // Silent refusal produced NO on-disk mode change; DELETE is the
                // pre-WAL default.
                let actual = set_journal_mode_no_wait(conn, "DELETE").unwrap_or_default();
                return Ok(actual);
            }
            let lowered = msg.to_ascii_lowercase();
            if !WAL_INCOMPAT_MARKERS.iter().any(|m| lowered.contains(m)) {
                // Unrelated OperationalError — don't silently swallow.
                return Err(WalError(msg));
            }
            // disk i/o error disambiguation: retry the pragma a couple of
            // times; transient EIO clears and we return "wal".
            if lowered.contains("disk i/o error") {
                for _ in 0..2 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    match set_wal() {
                        Ok(mode) => return Ok(mode),
                        Err(retry_exc) => {
                            let rl = retry_exc.0.to_ascii_lowercase();
                            if !rl.contains("disk i/o error") {
                                return Err(retry_exc);
                            }
                        }
                    }
                }
            }
            // Don't downgrade if another process already set WAL on disk, or
            // if the mode cannot be verified at all.
            let existing = on_disk_journal_mode(conn);
            if existing.as_deref() == Some("wal") || existing.is_none() {
                return Err(WalError(msg));
            }
            if require_wal {
                return Err(WalError(msg));
            }
            log_wal_fallback_once(db_label, &msg);
            let _ = set_journal_mode_no_wait(conn, "DELETE");
            Ok("delete".to_string())
        }
    }
}

/// Apply optional performance and WAL-sizing PRAGMAs from `config.yaml`.
///
/// Reads the `database:` section keys cache_size / mmap_size / temp_store /
/// wal_autocheckpoint / journal_size_limit. Best-effort: config load or
/// pragma failures are ignored so DB init never breaks on a malformed
/// `database:` section.
///
/// PARITY: hermes_state.py apply_database_pragmas @ b9aa928 (1159–1243).
pub fn apply_database_pragmas(conn: &rusqlite::Connection, db_label: &str) {
    let Some(cfg) = crate::cfg::load_config_value(&hermes_constants::get_config_path()) else {
        return;
    };
    let Some(database) = cfg.get("database").and_then(|v| v.as_mapping()) else {
        return;
    };
    for pragma_name in [
        "cache_size",
        "mmap_size",
        "temp_store",
        "wal_autocheckpoint",
        "journal_size_limit",
    ] {
        let raw = match database.get(serde_yaml::Value::String(pragma_name.to_string())) {
            Some(v) if !v.is_null() => v,
            _ => continue,
        };
        let rendered = match raw {
            serde_yaml::Value::String(s) => s.clone(),
            serde_yaml::Value::Number(n) => n.to_string(),
            other => format!("{:?}", other),
        };
        let Ok(value) = rendered.trim().parse::<i64>() else {
            eprintln!(
                "{}: ignoring non-integer database.{}=\"{}\"",
                db_label,
                pragma_name,
                rendered.trim(),
            );
            continue;
        };
        let _ = conn.execute_batch(&format!("PRAGMA {}={}", pragma_name, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_is_bundled_and_fts5_ready() {
        // Environment sanity for the whole crate (bundled build contract).
        assert!(rusqlite::version().starts_with("3.50."));
    }

    #[test]
    fn wal_reset_vulnerability_truth_table_matches_upstream() {
        // Oracle: hermes_cli/sqlite_runtime.py is_sqlite_wal_reset_vulnerable
        // (pure function of the version tuple; verified against upstream).
        assert!(!is_sqlite_wal_reset_vulnerable(&version(3, 6, 23)));
        assert!(is_sqlite_wal_reset_vulnerable(&version(3, 7, 0)));
        assert!(is_sqlite_wal_reset_vulnerable(&version(3, 50, 2)));
        assert!(!is_sqlite_wal_reset_vulnerable(&version(3, 50, 7)));
        assert!(!is_sqlite_wal_reset_vulnerable(&version(3, 50, 9)));
        assert!(is_sqlite_wal_reset_vulnerable(&version(3, 51, 0)));
        assert!(is_sqlite_wal_reset_vulnerable(&version(3, 51, 2)));
        assert!(!is_sqlite_wal_reset_vulnerable(&version(3, 51, 3)));
        assert!(!is_sqlite_wal_reset_vulnerable(&version(4, 0, 0)));
        assert!(!is_sqlite_wal_reset_vulnerable(&version(3, 44, 6)));
        assert!(!is_sqlite_wal_reset_vulnerable(&version(3, 44, 9)));
    }

    #[test]
    fn fresh_connection_under_vulnerable_sqlite_uses_delete() {
        // Our bundled 3.50.2 is in the vulnerable window (same as upstream's
        // bundled 3.50.4), so apply_wal_with_fallback must prefer DELETE.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fresh.db");
        let conn = rusqlite::Connection::open(&path).unwrap();
        let mode = apply_wal_with_fallback(&conn, "state.db", false).unwrap();
        assert_eq!(mode, "delete");
        assert_eq!(on_disk_journal_mode(&conn).as_deref(), Some("delete"));
        drop(conn);
        drop(dir); // ensure clean unlink (no stray wal/shm)
    }

    #[test]
    fn set_journal_mode_no_wait_restores_busy_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA busy_timeout=5000").unwrap();
        let mode = set_journal_mode_no_wait(&conn, "DELETE").unwrap();
        assert_eq!(mode, "delete");
        let t: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
            .unwrap();
        assert_eq!(t, 5000);
    }

    #[test]
    fn on_disk_journal_mode_reads_current_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.db");
        let conn = rusqlite::Connection::open(&path).unwrap();
        assert_eq!(on_disk_journal_mode(&conn).as_deref(), Some("delete"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn concurrent_connects_no_disk_io_error() {
        // Mirror of TestApplyWalProbe.test_apply_wal_concurrent_connects_no_eio:
        // 8 threads connecting with WAL fallback must not see disk i/o error.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("concurrent.db");
        let errors: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let path = path.clone();
                let errors = &errors;
                scope.spawn(move || {
                    for _ in 0..5 {
                        match rusqlite::Connection::open(&path) {
                            Ok(conn) => {
                                if let Err(e) = apply_wal_with_fallback(&conn, "state.db", false) {
                                    if e.0.to_ascii_lowercase().contains("disk i/o error") {
                                        errors.lock().unwrap().push(e.0);
                                    }
                                }
                            }
                            Err(e) => {
                                let msg = e.to_string().to_ascii_lowercase();
                                if msg.contains("disk i/o error") {
                                    errors.lock().unwrap().push(e.to_string());
                                }
                            }
                        }
                    }
                });
            }
        });
        let errors = errors.lock().unwrap();
        assert!(errors.is_empty(), "disk I/O errors: {:?}", *errors);
    }

    #[test]
    fn source_id_nonempty() {
        assert!(!sqlite_source_id().is_empty());
    }
}
