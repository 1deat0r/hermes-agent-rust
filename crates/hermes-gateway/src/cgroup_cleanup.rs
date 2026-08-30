//! SIGKILL any process left in this systemd unit's cgroup.
//!
//! PARITY: `gateway/cgroup_cleanup.py` @ b9aa928 (whole module).
//!
//! Runs as `ExecStopPost=` so it only fires after the gateway's main process
//! has exited. The gateway already reaps its own tool subprocesses on a clean
//! shutdown; this is the safety net for long-lived helpers it doesn't track
//! (`adb`, platform bridges, etc.) that would otherwise be orphaned in the
//! cgroup and block `Restart=always` — issue #37454.
//!
//! We deliberately iterate `cgroup.procs` and send per-PID SIGKILLs instead
//! of writing `1` to `cgroup.kill`: the original failure mode in #37454 was
//! the kernel returning `EINVAL` on the cgroup-wide kill, while per-PID
//! signal delivery uses a separate code path that still works.
//!
//! The `_at` forms take explicit `/proc`/`/sys` paths — the equivalent of the
//! upstream tests monkeypatching the `Path` constructor.

use std::fs;
use std::path::Path;

/// PARITY: `_own_cgroup_path` body (upstream lines 27-35) — the
/// `^0::(.+)$` MULTILINE search over the v2 unified-hierarchy line.
pub fn parse_own_cgroup_path(text: &str) -> Option<String> {
    // `^0::(.+)$` — `.+` needs at least one character, so a bare `0::` line
    // is not a match (but `0:: ` is; the whitespace survives to the strip).
    text.lines()
        .filter_map(|line| line.strip_prefix("0::"))
        .find(|rest| !rest.is_empty())
        .map(|rest| rest.trim().to_string())
}

/// Return the cgroup v2 path for the calling process, or None.
///
/// PARITY: `_own_cgroup_path` (upstream lines 27-35).
pub fn own_cgroup_path() -> Option<String> {
    let text = fs::read_to_string("/proc/self/cgroup").ok()?;
    parse_own_cgroup_path(&text)
}

/// PARITY: `_read_cgroup_pids` body (upstream lines 38-51).
pub fn read_cgroup_pids_at(procs_file: &Path) -> Vec<i64> {
    let raw = match fs::read_to_string(procs_file) {
        Ok(raw) => raw,
        Err(_) => return Vec::new(),
    };
    let mut pids = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(pid) = line.parse::<i64>() {
            pids.push(pid);
        }
    }
    pids
}

/// PARITY: `_read_cgroup_pids` (upstream lines 38-51).
pub fn read_cgroup_pids(cgroup_path: &str) -> Vec<i64> {
    read_cgroup_pids_at(
        &Path::new("/sys/fs/cgroup")
            .join(cgroup_path)
            .join("cgroup.procs"),
    )
}

/// SIGKILL every PID in the cgroup other than the caller. Returns the count
/// killed.
///
/// PARITY: `reap_cgroup` (upstream lines 54-71). `ProcessLookupError` /
/// `PermissionError` are skipped; any other kill error propagates upstream —
/// here `libc::kill` only surfaces ESRCH/EPERM for these targets, so both
/// arms map to "continue".
pub fn reap_cgroup(cgroup_path: Option<&str>) -> usize {
    let cgroup_path = match cgroup_path.map(str::to_string).or_else(own_cgroup_path) {
        Some(path) if !path.is_empty() => path,
        _ => return 0,
    };
    reap_cgroup_at(
        &Path::new("/sys/fs/cgroup")
            .join(cgroup_path)
            .join("cgroup.procs"),
    )
}

/// Explicit-procs-file form of [`reap_cgroup`].
pub fn reap_cgroup_at(procs_file: &Path) -> usize {
    let own = std::process::id() as i64;
    let mut killed = 0;
    for pid in read_cgroup_pids_at(procs_file) {
        if pid == own {
            continue;
        }
        // windows-footgun: ok — Linux-only (reads /proc, /sys/fs/cgroup; runs
        // from a systemd unit). SIGKILL delivery; ESRCH = ProcessLookupError,
        // EPERM = PermissionError, both continue per upstream.
        let Ok(pid) = i32::try_from(pid) else {
            continue;
        };
        let rc = unsafe { libc::kill(pid, libc::SIGKILL) };
        // ESRCH = ProcessLookupError, EPERM = PermissionError — both are
        // `continue` upstream and neither counts toward `killed`.
        if rc == 0 {
            killed += 1;
        }
    }
    killed
}

/// `main()` entry for the systemd `ExecStopPost=` invocation.
///
/// PARITY: `main` (upstream lines 74-76) — reap, then exit 0.
pub fn run() -> i32 {
    reap_cgroup(None);
    0
}
