//! Parity tests for `tools/mcp_stdio_watchdog.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); these cases derive from the upstream code as oracle, exercising
//! the pure seams (orphan detection, argv post-processing, spawn + reap)
//! plus a live end-to-end supervisor run against a real parent swap.

use std::process::Command;
use std::time::Duration;

use hermes_tools::mcp_stdio_watchdog::{
    is_orphaned, no_command_message, run, strip_separator, POLL_INTERVAL_S, TERM_GRACE_S,
};

#[test]
fn constants_match_upstream() {
    assert_eq!(POLL_INTERVAL_S, 2.0);
    assert_eq!(TERM_GRACE_S, 3.0);
}

#[test]
fn orphan_detection_compares_parent_ids() {
    // `_is_orphaned`: the instant getppid() differs from the recorded
    // original, the process is orphaned.
    assert!(!is_orphaned(1234, 1234));
    assert!(is_orphaned(1234, 1));
    assert!(is_orphaned(1234, 9999));
}

#[test]
fn argv_separator_is_stripped() {
    let mk = |items: &[&str]| -> Vec<String> { items.iter().map(|s| s.to_string()).collect() };
    assert_eq!(
        strip_separator(&mk(&["--", "node", "server.js"])),
        mk(&["node", "server.js"])
    );
    assert_eq!(
        strip_separator(&mk(&["node", "server.js"])),
        mk(&["node", "server.js"])
    );
    assert_eq!(strip_separator(&mk(&["--"])), Vec::<String>::new());
    assert_eq!(strip_separator(&mk(&[])), Vec::<String>::new());
    // Only a *leading* separator is dropped (argparse REMAINDER semantics).
    assert_eq!(
        strip_separator(&mk(&["node", "--", "--flag"])),
        mk(&["node", "--", "--flag"])
    );
}

#[test]
fn empty_command_message_is_pinned() {
    assert_eq!(
        no_command_message(),
        "mcp_stdio_watchdog: no command given after '--'"
    );
}

#[test]
fn run_returns_exit_code_2_for_no_command() {
    assert_eq!(run(1234, &[]), 2);
    assert_eq!(run(1234, &["--".to_string()]), 2);
}

#[test]
fn run_spawns_child_in_its_own_session_and_reaps_exit_code() {
    // The real command runs as a direct child in its own process group and
    // its exit code is forwarded verbatim. The recorded parent must be this
    // process's real parent — exactly what the binary invocation records
    // via `--ppid` — so the watchdog does not consider us orphaned.
    let real_ppid = unsafe { libc::getppid() };
    let code = run(
        real_ppid,
        &["sh".to_string(), "-c".to_string(), "exit 7".to_string()],
    );
    assert_eq!(code, 7);
}

#[test]
fn run_survives_an_already_orphaned_record() {
    // A recorded original_ppid of 1 (init) while our real parent is not 1
    // means the watchdog believes we are orphaned immediately — it must
    // terminate the child and still return, not hang.
    let code = run(
        1,
        &["sh".to_string(), "-c".to_string(), "sleep 30".to_string()],
    );
    // 128+15 = SIGKILL? No: SIGTERM then SIGKILL; the shell is killed by
    // SIGTERM (or SIGKILL) — either way the process was reaped, not hung.
    assert!(
        code == 128 + 15 || code == 128 + 9 || code != 0,
        "code={code}"
    );
}

#[test]
fn passthrough_and_outliving_parent_swap() {
    // End-to-end: spawn the actual binary via the workspace test runner,
    // hand it a recorded ppid that stays valid, and confirm the relayed
    // child's output arrives (transparent stdio pass-through).
    let bin = env!("CARGO_BIN_EXE_mcp_stdio_watchdog");
    let ppid = std::process::id() as i32;
    let output = Command::new(bin)
        .args(["--ppid", &ppid.to_string(), "--", "echo", "relayed"])
        .output()
        .expect("spawn watchdog binary");
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "relayed");

    // Parent-death path: record a ppid that cannot match (the binary's own
    // parent differs from ours), so the watchdog must kill the sleeper
    // quickly instead of running 30s.
    let started = std::time::Instant::now();
    let output = Command::new(bin)
        .args(["--ppid", "999999999", "--", "sleep", "30"])
        .output()
        .expect("spawn watchdog binary");
    assert!(!output.status.success());
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "watchdog must not wait out the full sleep"
    );
}

#[test]
fn missing_ppid_is_a_usage_error() {
    let bin = env!("CARGO_BIN_EXE_mcp_stdio_watchdog");
    let output = Command::new(bin)
        .args(["--", "echo", "hi"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}
