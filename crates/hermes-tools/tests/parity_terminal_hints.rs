//! Parity oracles for tools/terminal_hints.rs — output-pattern failure hints.
//!
//! Mirrors upstream tests/tools/test_terminal_hints.py @ b9aa928.
//!
//! DEFERRED (unported subsystem): `TestTerminalIntegration` exercises
//! `terminal_tool._interpret_exit_code` wiring (hint lands in the terminal
//! result dict; exit_note suppresses pattern hint). `terminal_tool` is not
//! ported yet — defer both wiring tests until it lands.
//!
//! DEFERRED (no monkey-patching in Rust): `test_hint_functions_cannot_crash_
//! annotate` patches `_OUTPUT_HINTS` with a raising lambda to pin the
//! try/except guard. Rust hint fns are pure string/regex ops that cannot
//! raise; the dispatcher already calls each hint independently and falls
//! through, so the guard is structurally unreachable.

use hermes_tools::terminal_hints::{annotate_failure, SCAN_CHARS};

#[test]
fn success_never_annotated() {
    assert_eq!(annotate_failure("python x.py", 0, "python: command not found"), None);
}

#[test]
fn empty_output_falls_to_exit_code_tier() {
    let h = annotate_failure("./run.sh", 126, "").unwrap();
    assert!(h.contains("126"));
    let h = annotate_failure("big_job", 137, "").unwrap();
    assert!(h.contains("SIGKILL"));
    let h = annotate_failure("sleep 999", 124, "").unwrap();
    assert!(h.contains("timeout"));
}

#[test]
fn unknown_failure_returns_none() {
    assert_eq!(annotate_failure("./x", 1, "some unrecognized error"), None);
}

#[test]
fn only_first_matching_hint() {
    // merge_conflict is ordered before command_not_found upstream.
    let out = "CONFLICT (content): Merge conflict in a.py\npython: command not found";
    let hint = annotate_failure("git merge x && python t.py", 1, out).unwrap();
    assert!(hint.to_lowercase().contains("conflict"));
    assert!(!hint.contains("python3"));
}

#[test]
fn gh_field_name_extracted() {
    let out = "Unknown JSON field: \"authorAssociation\"\nAvailable fields:\n  additions\n  author";
    let hint = annotate_failure("gh pr view 1 --json authorAssociation", 1, out).unwrap();
    assert!(hint.contains("authorAssociation"));
    assert!(hint.contains("valid field list"));
}

#[test]
fn bare_python_gets_python3_hint() {
    let out = "/usr/bin/bash: line 1: python: command not found";
    let hint = annotate_failure("python x.py", 127, out).unwrap();
    assert!(hint.contains("python3"));
}

#[test]
fn bare_pip_gets_pip3_hint() {
    let out = "bash: pip: command not found";
    let hint = annotate_failure("pip install x", 127, out).unwrap();
    assert!(hint.contains("pip3") || hint.contains("-m pip"));
}

#[test]
fn generic_command() {
    let out = "bash: line 3: shellcheck: command not found";
    let hint = annotate_failure("shellcheck s.sh", 127, out).unwrap();
    assert!(hint.contains("shellcheck"));
    assert!(hint.contains("which"));
}

#[test]
fn module_named() {
    let out = "Traceback (most recent call last):\n  File \"x.py\", line 1\nModuleNotFoundError: No module named 'requests'";
    let hint = annotate_failure("python3 x.py", 1, out).unwrap();
    assert!(hint.contains("requests"));
    assert!(hint.contains("venv"));
}

#[test]
fn dotted_module() {
    let out = "ImportError: No module named 'hermes_cli.main'";
    let hint = annotate_failure("python3 -m hermes_cli.main", 1, out).unwrap();
    assert!(hint.contains("hermes_cli"));
}

#[test]
fn merge_conflict() {
    let out = "Auto-merging a.py\nCONFLICT (content): Merge conflict in a.py\nAutomatic merge failed; fix conflicts and then commit the result.";
    let hint = annotate_failure("git merge feature", 1, out).unwrap();
    assert!(hint.contains("Do not retry"));
}

#[test]
fn branch_already_exists() {
    let out = "fatal: a branch named 'fix/x' already exists";
    let hint = annotate_failure("git checkout -b fix/x", 128, out).unwrap();
    assert!(hint.contains("fix/x"));
}

#[test]
fn rate_limit() {
    let out = "GraphQL: API rate limit already exceeded for user ID 1.";
    let hint = annotate_failure("gh pr list", 1, out).unwrap();
    assert!(hint.to_lowercase().contains("rate limit"));
}

#[test]
fn permission_denied() {
    let hint = annotate_failure(
        "touch /etc/x",
        1,
        "touch: cannot touch '/etc/x': Permission denied",
    )
    .unwrap();
    assert!(hint.contains("Permission denied"));
}

#[test]
fn pattern_beyond_scan_window_ignored() {
    // 5000 chars of noise then the recognizable failure — window is 4000.
    let out = "x".repeat(5000) + "\npython: command not found";
    assert_eq!(annotate_failure("noop", 1, &out), None);
}

#[test]
fn pattern_exactly_at_scan_window_boundary_ignored() {
    // The failure header starts exactly at SCAN_CHARS — outside the window
    // (upstream slices output[:4000], so index 4000 is excluded).
    let out = "x".repeat(SCAN_CHARS) + "python: command not found";
    assert_eq!(annotate_failure("noop", 1, &out), None);
}

#[test]
fn scan_window_is_char_based_not_byte_based() {
    // Multi-byte chars before the boundary — Python slices by code points.
    // Pad so the failure header exactly fills the char-based window: the
    // total length is SCAN_CHARS chars while the byte length is ~3x larger.
    let header = " python: command not found";
    let padding = SCAN_CHARS - header.chars().count();
    let out = "\u{4e2d}".repeat(padding) + header;
    let hint = annotate_failure("python x.py", 127, &out).unwrap();
    assert!(hint.contains("python3"));
}
