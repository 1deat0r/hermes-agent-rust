//! Cross-process full-FTS-rebuild admission (single authority).
//!
//! PARITY: hermes_state_common.py (949–1218): `_acquire_db_flock`,
//! `fts_rebuild_admission`, holder-record helpers, contention table.
//!
//! Several independent Hermes processes share one state.db; a full
//! structural FTS rebuild must run in ONE of them at a time (PR #93200
//! class). Semantics mirror upstream: portable advisory lock, bounded
//! wait, FAIL CLOSED — a caller that cannot acquire must NOT rebuild.
//! The lock file is `<db>.fts_rebuild.lock`.
//!
//! `flock` rides the open file DESCRIPTION, which `fork()` duplicates: a
//! holder that forks then dies leaves the lock held by a child that will
//! never release it (#100108). When the recorded holder is provably dead
//! the file is unlinked and retaken on a fresh inode; indeterminate
//! liveness defers (fail closed).

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::common::{
    _FTS_REBUILD_LOCK_POLL_SECONDS, _FTS_REBUILD_LOCK_TIMEOUT_SECONDS,
    _LOCK_BREAK_REACQUIRE_SECONDS, _LOCK_CONTENTION_ERRNOS,
};

fn unix_now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// True when *exc* means another process holds the lock; on any other
/// `OSError` fail closed at once.
///
/// PARITY: `is_advisory_lock_contention` (993–995): `BlockingIOError`
/// (WouldBlock) or errno in {EAGAIN, EACCES, EWOULDBLOCK, EDEADLK}.
pub fn is_advisory_lock_contention(err: &std::io::Error) -> bool {
    match err.kind() {
        std::io::ErrorKind::WouldBlock => return true,
        std::io::ErrorKind::Interrupted => return false,
        _ => {}
    }
    match err.raw_os_error() {
        Some(errno) => _LOCK_CONTENTION_ERRNOS.contains(&errno),
        None => false,
    }
}

/// Kernel start time of *pid* (field 22 of `/proc/<pid>/stat`; with the PID
/// it identifies a process uniquely). `None` off Linux or on any failure —
/// callers must treat None as unknowable and FAIL CLOSED.
// PARITY: `_proc_start_ticks` (998–1006)
pub fn proc_start_ticks(pid: u32) -> Option<i64> {
    #[cfg(target_os = "linux")]
    {
        let raw = std::fs::read(format!("/proc/{}/stat", pid)).ok()?;
        let after_comm = std::str::from_utf8(raw.rsplit(|b: &u8| *b == b')').next()?).ok()?;
        // After the last ')': fields 3.. of stat; start-time is field 22 →
        // index 19 of the post-comm split (matches Python [19]). split_whitespace
        // skips empty segments (leading space after ')'), matching Python .split().
        after_comm.split_whitespace().nth(19)?.parse().ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

/// Best-effort parse of the holder metadata JSON in a lock file.
// PARITY: `_read_lock_holder_record` (1009–1017)
fn read_lock_holder_record(handle: &mut File) -> Option<serde_json::Value> {
    let mut buf = [0u8; 4096];
    handle.seek(SeekFrom::Start(0)).ok()?;
    let n = handle.read(&mut buf).ok()?;
    if n == 0 {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(&buf[..n]).ok()?;
    value.is_object().then_some(value)
}

/// Best-effort truncate-and-write of *payload* at offset 0.
// PARITY: `_rewrite_lock_file` (1020–1028)
fn rewrite_lock_file(handle: &mut File, payload: &[u8]) {
    if handle.seek(SeekFrom::Start(0)).is_err() {
        return;
    }
    let _ = handle.set_len(0);
    if !payload.is_empty() {
        let _ = handle.write_all(payload);
    }
    let _ = handle.flush();
}

/// Record this process as holder (best effort) so timed-out contenders can
/// tell an orphaned-fd holder from a live wedged one.
// PARITY: `_write_lock_holder_record` (1030–1038)
fn write_lock_holder_record(handle: &mut File) {
    let pid = std::process::id();
    let record = serde_json::json!({
        "pid": pid,
        "start_ticks": proc_start_ticks(pid),
        "acquired_at": unix_now(),
    });
    let payload = serde_json::to_vec(&record).unwrap_or_default();
    rewrite_lock_file(handle, &payload);
}

/// Erase holder metadata before a normal release: a surviving record means
/// ABNORMAL exit (break allowed).
// PARITY: `_clear_lock_holder_record` (1041–1043)
fn clear_lock_holder_record(handle: &mut File) {
    rewrite_lock_file(handle, b"");
}

/// True ONLY when the recorded holder is provably dead or PID-recycled.
/// Anything indeterminate is False: FAIL CLOSED and defer.
// PARITY: `_lock_holder_provably_dead` (1046–1065)
fn lock_holder_provably_dead(record: Option<&serde_json::Value>) -> bool {
    let Some(record) = record else { return false };
    let Some(pid) = record.get("pid").and_then(|p| p.as_i64()) else {
        return false;
    };
    if pid <= 0 {
        return false;
    }
    // kill(pid, 0): 0 → alive; ESRCH(3) → dead; anything else (EPERM…) →
    // PID exists or unknowable → closed.
    let rc = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if rc != 0 {
        let errno = std::io::Error::last_os_error().raw_os_error();
        return errno == Some(libc::ESRCH);
    }
    let Some(recorded_ticks) = record.get("start_ticks").and_then(|t| t.as_i64()) else {
        return false;
    };
    match proc_start_ticks(pid as u32) {
        Some(current) => current != recorded_ticks, // different start time: recycled
        None => false,                              // unknowable → closed
    }
}

/// Human-readable holder identity for deferral warnings.
// PARITY: `_describe_lock_holder` (1132–1140)
pub fn describe_lock_holder(record: Option<&serde_json::Value>) -> String {
    let Some(record) = record.filter(|r| r.is_object()) else {
        return "unknown (no holder record; pre-fix writer or non-Hermes)".to_string();
    };
    if record.get("pid").is_none() {
        return "unknown (no holder record; pre-fix writer or non-Hermes)".to_string();
    }
    let mut age = String::new();
    if let Some(acquired) = record.get("acquired_at").and_then(|a| a.as_f64()) {
        let delta = unix_now() - acquired;
        if delta.is_finite() && delta >= 0.0 {
            age = format!(", acquired {:.0}s ago", delta);
        }
    }
    format!("pid {}{}", record.get("pid").unwrap(), age)
}

/// Advisory lock outcomes (upstream True / False / None tuple).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acquired {
    Yes,
    No,
    /// Non-contention OSError — already logged; treat as not acquired.
    Error,
}

#[cfg(unix)]
fn try_flock(handle: &File) -> std::io::Result<()> {
    use std::os::unix::io::AsRawFd;
    let rc = unsafe { libc::flock(handle.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn try_funlock(handle: &File) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        libc::flock(handle.as_raw_fd(), libc::LOCK_UN);
    }
}

#[cfg(unix)]
fn same_file(handle: &File, lock_path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let Ok(fd_meta) = handle.metadata() else {
        return false;
    };
    let Ok(path_meta) = std::fs::metadata(lock_path) else {
        return false;
    };
    fd_meta.dev() == path_meta.dev() && fd_meta.ino() == path_meta.ino()
}

#[cfg(not(unix))]
fn same_file(_handle: &File, _lock_path: &Path) -> bool {
    false
}

fn open_lock_file(lock_path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(lock_path)
}

/// Bounded POSIX flock acquire with orphaned-holder break.
///
/// Returns `(Acquired, handle)` — *handle* may have been re-opened (the
/// breaker replaces the inode); the caller closes whichever comes back.
// PARITY: `_acquire_db_flock` (1068–1129)
pub fn acquire_db_flock(
    lock_path: &Path,
    handle: &mut File,
    timeout: Duration,
    poll: Duration,
    description: &str,
) -> (Acquired, Option<File>) {
    let mut deadline = Instant::now() + timeout;
    let mut broke_lock = false;
    loop {
        match try_flock(handle) {
            Ok(()) => {}
            Err(exc) => {
                if !is_advisory_lock_contention(&exc) {
                    log::warn!(
                        "Could not acquire {} {} ({}) — deferring rather than \
                         waiting out the {:.0}s holder timeout on a non-contention error.",
                        description,
                        lock_path.display(),
                        exc,
                        timeout.as_secs_f64()
                    );
                    return (Acquired::Error, None);
                }
                if Instant::now() < deadline {
                    std::thread::sleep(poll);
                    continue;
                }
                if broke_lock {
                    return (Acquired::No, None);
                }
                let record = read_lock_holder_record(handle);
                if !lock_holder_provably_dead(record.as_ref()) {
                    return (Acquired::No, None);
                }
                let pid = record
                    .as_ref()
                    .and_then(|r| r.get("pid"))
                    .map(|p| p.to_string())
                    .unwrap_or_default();
                log::warn!(
                    "{} {} is held by an orphaned file descriptor (recorded holder pid {} is dead — a \
                     forked child inherited the lock fd); breaking the stale lock and retaking it on a \
                     fresh file.",
                    description,
                    lock_path.display(),
                    pid
                );
                match std::fs::remove_file(lock_path) {
                    Ok(()) => {}
                    Err(e) => {
                        log::warn!(
                            "Could not break stale {} {} ({}) — deferring.",
                            description,
                            lock_path.display(),
                            e
                        );
                        return (Acquired::No, None);
                    }
                }
                match open_lock_file(lock_path) {
                    Ok(f) => {
                        *handle = f;
                    }
                    Err(e) => {
                        log::warn!(
                            "Could not break stale {} {} ({}) — deferring.",
                            description,
                            lock_path.display(),
                            e
                        );
                        return (Acquired::No, None);
                    }
                }
                broke_lock = true;
                // Post-break re-acquire budget: the fresh inode is contended
                // only by live processes — never the full timeout
                // (PARITY: deadline shrink at 1112; continue the main loop
                // exactly like upstream rather than early-returning).
                deadline = Instant::now() + Duration::from_secs_f64(_LOCK_BREAK_REACQUIRE_SECONDS);
                continue;
            }
        }
        // A breaker may have replaced the file while we waited; a lock on a
        // dead inode excludes nobody.
        if same_file(handle, lock_path) {
            write_lock_holder_record(handle);
            return (Acquired::Yes, None);
        }
        match open_lock_file(lock_path) {
            Ok(f) => {
                *handle = f;
            }
            Err(_) => return (Acquired::No, None),
        }
        if Instant::now() >= deadline {
            return (Acquired::No, None);
        }
        std::thread::sleep(poll);
    }
}

/// Outcome of an admission hold (the `yield acquired` in upstream's
/// contextmanager).
#[derive(Debug)]
pub struct FtsRebuildAdmission {
    handle: Option<File>,
    acquired: bool,
    windows_style: bool,
}

impl FtsRebuildAdmission {
    /// Whether this process holds the authority. False → do NOT rebuild.
    pub fn acquired(&self) -> bool {
        self.acquired
    }
}

impl Drop for FtsRebuildAdmission {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.as_mut() {
            if self.acquired {
                if self.windows_style {
                    // Windows msvcrt path deferred with the Windows wave —
                    // see PORT SEAMS in lib.rs; nothing to clear on POSIX.
                } else {
                    clear_lock_holder_record(handle);
                    try_funlock(handle);
                }
            }
        }
    }
}

/// Serialize full structural FTS rebuilds on *db_path* across processes.
///
/// Yields an admission whose `acquired()` is true when this process holds
/// the authority, false when the bounded acquire timed out or the lock file
/// could not be opened (FAIL CLOSED — #100368: an unopenable lock file means
/// the FS is out of space/inodes while a sibling may still be rebuilding).
/// `db_path` `None` (in-memory) always admits. In-process retries pass
/// `timeout_seconds = Some(0.0)` so a live holder never stalls a long-lived
/// writer; the orphan break still applies.
///
/// PARITY: `fts_rebuild_admission` (1162–1218).
pub fn fts_rebuild_admission(db_path: Option<&Path>) -> FtsRebuildAdmission {
    fts_rebuild_admission_with_timeout(db_path, None)
}

pub fn fts_rebuild_admission_with_timeout(
    db_path: Option<&Path>,
    timeout_seconds: Option<f64>,
) -> FtsRebuildAdmission {
    let Some(db_path) = db_path else {
        return FtsRebuildAdmission {
            handle: None,
            acquired: true,
            windows_style: false,
        };
    };
    let timeout = timeout_seconds
        .map(|t| t.max(0.0))
        .unwrap_or(_FTS_REBUILD_LOCK_TIMEOUT_SECONDS);
    let lock_path: PathBuf = PathBuf::from(format!("{}.fts_rebuild.lock", db_path.display()));

    let mut handle = match open_lock_file(&lock_path) {
        Ok(h) => h,
        Err(exc) => {
            // Fail closed like a timed-out acquire (#100368).
            log::warn!(
                "Could not open FTS rebuild lock {} ({}) — deferring this rebuild \
                 rather than running it without cross-process authority.",
                lock_path.display(),
                exc
            );
            return FtsRebuildAdmission {
                handle: None,
                acquired: false,
                windows_style: false,
            };
        }
    };

    let poll = Duration::from_secs_f64(_FTS_REBUILD_LOCK_POLL_SECONDS);
    let (outcome, reopened) = acquire_db_flock(
        &lock_path,
        &mut handle,
        Duration::from_secs_f64(timeout),
        poll,
        "FTS rebuild lock",
    );
    let handle = reopened.unwrap_or(handle);
    let mut acquired = matches!(outcome, Acquired::Yes);
    if outcome == Acquired::Error {
        // Already logged with the real errno; "held" would be a lie.
        acquired = false;
    }
    if !acquired {
        let record = match handle.try_clone() {
            Ok(mut h) => read_lock_holder_record(&mut h),
            Err(_) => None,
        };
        if timeout <= 0.0 {
            log::info!(
                "FTS rebuild lock {} is busy — deferring this retry \
                 (the stale-FTS breadcrumb keeps it retryable). Recorded holder: {}.",
                lock_path.display(),
                describe_lock_holder(record.as_ref())
            );
        } else {
            log::warn!(
                "FTS rebuild lock {} held by another process for more than {:.0}s — deferring \
                 this rebuild to avoid racing the holder (the stale-FTS breadcrumb keeps it \
                 retryable). Recorded holder: {}.",
                lock_path.display(),
                timeout,
                describe_lock_holder(record.as_ref())
            );
        }
    }
    FtsRebuildAdmission {
        handle: Some(handle),
        acquired,
        windows_style: false,
    }
}

// NOTE: upstream's Windows branch (`_acquire_msvcrt_lock`) is cfg'd out —
// POSIX-only target; see lib.rs PORT SEAMS (Windows wave).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::_FTS_REBUILD_LOCK_TIMEOUT_SECONDS;

    #[test]
    fn contention_table_matches_oracle() {
        // PARITY: test_is_advisory_lock_contention_table (480–494).
        // BlockingIOError(EAGAIN) → WouldBlock; raw errno checks cover the
        // rest; non-os errors (io::Error::other) → false (ValueError analog).
        let eagain = std::io::Error::from_raw_os_error(libc::EAGAIN);
        assert!(is_advisory_lock_contention(&eagain));
        let ewould = std::io::Error::from_raw_os_error(libc::EWOULDBLOCK);
        assert!(is_advisory_lock_contention(&ewould));
        let eacces = std::io::Error::from_raw_os_error(libc::EACCES);
        assert!(is_advisory_lock_contention(&eacces));
        let edeadlk = std::io::Error::from_raw_os_error(libc::EDEADLK);
        assert!(is_advisory_lock_contention(&edeadlk));
        // WouldBlock kind without raw errno still counts (BlockingIOError shape).
        assert!(is_advisory_lock_contention(&std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            "x"
        )));
        for errno in [libc::ESTALE, libc::ENOTSUP, libc::ENOLCK, libc::EIO] {
            assert!(
                !is_advisory_lock_contention(&std::io::Error::from_raw_os_error(errno)),
                "errno {} must fail closed",
                errno
            );
        }
        assert!(!is_advisory_lock_contention(&std::io::Error::other(
            "not an oserror"
        )));
    }

    #[test]
    fn admission_fails_closed_when_lock_file_is_unopenable() {
        // PARITY: test_fts_admission_fails_closed_when_lock_file_is_unopenable
        // (60–66) — a directory where the code expects a file.
        let td = tempfile::TempDir::new().unwrap();
        let db_path = td.path().join("state.db");
        let lock_path = td.path().join("state.db.fts_rebuild.lock");
        std::fs::create_dir(&lock_path).unwrap();
        let admission = fts_rebuild_admission(Some(&db_path));
        assert!(
            !admission.acquired(),
            "unopenable lock must refuse admission"
        );
    }

    #[test]
    fn admission_still_admits_a_pathless_db() {
        // PARITY: test_fts_admission_still_admits_a_pathless_db (69–75).
        assert!(fts_rebuild_admission(None).acquired());
    }

    #[test]
    fn admission_holds_and_clears_holder_record() {
        // PARITY: test_holder_record_cleared_on_normal_release (322–327) —
        // record present while held, empty after drop.
        let td = tempfile::TempDir::new().unwrap();
        let db = td.path().join("x.db");
        let lock_path = td.path().join("x.db.fts_rebuild.lock");
        {
            let admission = fts_rebuild_admission(Some(&db));
            assert!(admission.acquired());
            let mut f = OpenOptions::new()
                .read(true)
                .open(&lock_path)
                .expect("lock file exists while held");
            let record = read_lock_holder_record(&mut f);
            assert!(record.is_some(), "holder record written under the lock");
            assert_eq!(
                record
                    .as_ref()
                    .and_then(|r| r.get("pid"))
                    .and_then(|p| p.as_u64()),
                Some(std::process::id() as u64)
            );
        }
        let mut f = OpenOptions::new()
            .read(true)
            .open(&lock_path)
            .expect("lock file survives release");
        let record = read_lock_holder_record(&mut f);
        assert!(
            record.is_none(),
            "normal release clears the holder record: {:?}",
            record
        );
    }

    #[test]
    fn admission_defers_while_foreign_holder_present() {
        // PARITY: test_rebuild_defers_while_another_process_holds_authority —
        // a same-process second fd contends (flock is per open-file-description,
        // so a second OPEN contends on Linux).
        let td = tempfile::TempDir::new().unwrap();
        let db = td.path().join("state.db");
        let lock_path = td.path().join("state.db.fts_rebuild.lock");
        let mut holder = open_lock_file(&lock_path).unwrap();
        // First acquire on the fresh lock succeeds.
        let (out, _) = acquire_db_flock(
            &lock_path,
            &mut holder,
            Duration::from_millis(150),
            Duration::from_millis(20),
            "FTS rebuild lock",
        );
        assert_eq!(out, Acquired::Yes, "uncontended first acquire must succeed");

        // Second open contends: bounded acquire times out fail-closed.
        let mut contender = open_lock_file(&lock_path).unwrap();
        let (out, _) = acquire_db_flock(
            &lock_path,
            &mut contender,
            Duration::from_millis(150),
            Duration::from_millis(20),
            "FTS rebuild lock",
        );
        assert_eq!(out, Acquired::No, "contended acquire times out fail-closed");

        // Zero-timeout probe (in-process retry shape) stays quiet + closed.
        let admission = fts_rebuild_admission_with_timeout(Some(&db), Some(0.0));
        assert!(!admission.acquired());
        drop(admission);
        drop(contender);
        drop(holder);

        // Holder gone → admits.
        let admission = fts_rebuild_admission(Some(&db));
        assert!(admission.acquired());
    }

    #[test]
    fn proc_start_ticks_reads_own_pid_on_linux() {
        #[cfg(target_os = "linux")]
        {
            let ticks = proc_start_ticks(std::process::id()).expect("own /proc stat");
            assert!(ticks > 0);
            // Invalid pid → None (fail-closed unknowable).
            assert_eq!(proc_start_ticks(u32::MAX - 1), None);
        }
        #[cfg(not(target_os = "linux"))]
        {
            assert_eq!(proc_start_ticks(1), None);
        }
    }

    #[test]
    fn default_timeout_constant_matches_upstream() {
        assert_eq!(_FTS_REBUILD_LOCK_TIMEOUT_SECONDS, 120.0);
    }
}
