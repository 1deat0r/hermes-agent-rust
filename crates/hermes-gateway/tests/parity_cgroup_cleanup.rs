//! Parity tests for `gateway/cgroup_cleanup.py` @ b9aa928, mirroring
//! upstream `tests/gateway/test_cgroup_cleanup.py` plus direct-body cases
//! (upstream monkeypatches the `Path` constructor; here the `_at` forms take
//! explicit paths).

use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

use hermes_gateway::cgroup_cleanup;

#[test]
fn parses_v2_cgroup_path() {
    assert_eq!(
        cgroup_cleanup::parse_own_cgroup_path(
            "12:pids:/user.slice\n0::/user.slice/user-1000.slice/hermes-gateway.service\n"
        )
        .as_deref(),
        Some("/user.slice/user-1000.slice/hermes-gateway.service")
    );
}

#[test]
fn missing_v2_line_is_none() {
    // Only controller-hierarchy lines (cgroup v1) -> no unified path, the
    // None arm that makes reap_cgroup a no-op.
    assert_eq!(
        cgroup_cleanup::parse_own_cgroup_path("12:pids:/user.slice\n"),
        None
    );
    assert_eq!(cgroup_cleanup::parse_own_cgroup_path(""), None);
}

#[test]
fn own_cgroup_path_reads_proc_self_cgroup() {
    // Read-only check against the live /proc — on cgroup v2 hosts the
    // unified line exists; the function must fail open to None otherwise.
    let path = cgroup_cleanup::own_cgroup_path();
    if let Some(path) = path {
        assert!(path.starts_with('/'));
    }
}

#[test]
fn reads_cgroup_pids_leniently() {
    let td = tempfile::TempDir::new().unwrap();
    let procs = td.path().join("cgroup.procs");
    fs::write(&procs, "123\n  456 \n\nnot-a-pid\n789\n").unwrap();
    assert_eq!(
        cgroup_cleanup::read_cgroup_pids_at(&procs),
        vec![123, 456, 789]
    );
}

#[test]
fn missing_procs_file_yields_empty() {
    let td = tempfile::TempDir::new().unwrap();
    assert!(cgroup_cleanup::read_cgroup_pids_at(&td.path().join("does-not-exist")).is_empty());
}

#[test]
fn reap_is_noop_when_procs_file_missing() {
    // Upstream: kill must not be called at all when cgroup.procs is
    // unreadable; the count stays 0.
    let td = tempfile::TempDir::new().unwrap();
    assert_eq!(
        cgroup_cleanup::reap_cgroup_at(&td.path().join("does-not-exist")),
        0
    );
}

#[test]
fn reap_skips_own_pid_and_counts_only_successful_kills() {
    // Spawn a real helper process, list its pid alongside our own, and reap:
    // our own pid is skipped, the child is SIGKILLed and counted.
    let mut child = Command::new("sleep")
        .arg("30")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sleep helper");
    let child_pid = child.id() as i64;
    let td = tempfile::TempDir::new().unwrap();
    let procs = td.path().join("cgroup.procs");
    fs::write(&procs, format!("{}\n{}\n", std::process::id(), child_pid)).unwrap();

    assert_eq!(cgroup_cleanup::reap_cgroup_at(&procs), 1);

    let status = child.wait().unwrap();
    assert!(
        status.signal() == Some(9),
        "child should have been SIGKILLed, got {status:?}"
    );
}

#[test]
fn reap_with_only_own_pid_kills_nothing() {
    let td = tempfile::TempDir::new().unwrap();
    let procs = td.path().join("cgroup.procs");
    fs::write(&procs, format!("{}\n", std::process::id())).unwrap();
    assert_eq!(cgroup_cleanup::reap_cgroup_at(&procs), 0);
}
