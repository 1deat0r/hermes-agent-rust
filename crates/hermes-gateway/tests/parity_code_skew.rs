//! Parity tests for `gateway/code_skew.py` @ b9aa928, mirroring upstream
//! `tests/test_code_skew.py` (the `TestDetectCodeSkew` / `TestShort`
//! classes; the `TestModelSwitchSkewGuard` class belongs to
//! `gateway.slash_commands`, which is not yet ported).
//!
//! The Rust seams `set_fingerprint_override_for_tests` / `reset_for_tests`
//! stand in for the upstream tests' `monkeypatch` of `_fingerprint` and the
//! autouse `_reset_boot_fingerprint` fixture.

use hermes_gateway::code_skew;

fn set_fp(value: &str) {
    code_skew::set_fingerprint_override_for_tests(Some(Some(value.to_string())));
}

#[test]
fn no_boot_fingerprint_means_no_skew() {
    code_skew::reset_for_tests();
    // Nothing recorded (e.g. non-git install) -> never a false positive.
    set_fp("git:refs/heads/main:def456");
    assert!(code_skew::detect_code_skew().is_none());
}

#[test]
fn drift_is_detected_with_short_revs() {
    code_skew::reset_for_tests();
    set_fp("git:refs/heads/main:abc1234567890");
    code_skew::record_boot_fingerprint();
    set_fp("git:refs/heads/main:def4567890123");
    assert_eq!(
        code_skew::detect_code_skew(),
        Some(("abc1234567".to_string(), "def4567890".to_string()))
    );
}

#[test]
fn unchanged_fingerprint_is_not_skew() {
    code_skew::reset_for_tests();
    set_fp("git:refs/heads/main:abc1234567890");
    code_skew::record_boot_fingerprint();
    assert!(code_skew::detect_code_skew().is_none());
}

#[test]
fn unreadable_disk_fingerprint_after_boot_is_not_skew() {
    // `current is None` arm — a removed checkout mid-process never yields a
    // partial comparison.
    code_skew::reset_for_tests();
    set_fp("git:refs/heads/main:abc1234567890");
    code_skew::record_boot_fingerprint();
    code_skew::set_fingerprint_override_for_tests(Some(None));
    assert!(code_skew::detect_code_skew().is_none());
}

#[test]
fn record_boot_fingerprint_is_idempotent() {
    code_skew::reset_for_tests();
    set_fp("git:refs/heads/main:abc1234567890");
    code_skew::record_boot_fingerprint();
    set_fp("git:refs/heads/main:def4567890123");
    code_skew::record_boot_fingerprint();
    assert_eq!(
        code_skew::detect_code_skew(),
        Some(("abc1234567".to_string(), "def4567890".to_string()))
    );
}

#[test]
fn real_disk_fingerprint_round_trip() {
    // Wire-level check against the hermes-cli reader on a real (pinned)
    // checkout directory: fingerprint_at must produce the `git:<ref>:<sha>`
    // shape.
    let dir = tempfile::TempDir::new().unwrap();
    let git = dir.path().join(".git");
    std::fs::create_dir_all(git.join("refs/heads")).unwrap();
    std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(
        git.join("refs/heads/main"),
        "1111111111111111111111111111111111111111\n",
    )
    .unwrap();
    assert_eq!(
        code_skew::fingerprint_at(dir.path()).unwrap(),
        "git:refs/heads/main:1111111111111111111111111111111111111111"
    );
}

#[test]
fn short_shortens_long_sha() {
    assert_eq!(
        code_skew::short("git:refs/heads/main:abcdef0123456789"),
        "abcdef0123"
    );
}

#[test]
fn short_keeps_unresolved_marker() {
    assert_eq!(
        code_skew::short("git:refs/heads/main:unresolved"),
        "unresolved"
    );
}

#[test]
fn short_passes_short_sha_through_untruncated() {
    assert_eq!(code_skew::short("git:HEAD:abc1234"), "abc1234");
}

#[test]
fn short_empty_sha_falls_back_to_whole_fingerprint() {
    // Python `return sha or fingerprint`.
    assert_eq!(code_skew::short("git:"), "git:");
    assert_eq!(code_skew::short(""), "");
}
