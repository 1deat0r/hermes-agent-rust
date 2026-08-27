// Tier: unit — mirrors tests/hermes_cli/test_build_info.py for
// `hermes_cli/build_info.py`.

use hermes_cli::build_info::{get_build_sha_at, get_build_sha_with};
use std::fs;
use tempfile::tempdir;

const FULL_SHA: &str = "abcdef1234567890abcdef1234567890abcdef12";

#[test]
fn returns_none_when_the_file_is_absent() {
    // Source installs: no file present → None, callers fall back to git.
    let td = tempdir().expect("tempdir");
    let missing = td.path().join(".hermes_build_sha");
    assert_eq!(get_build_sha_at(&missing, 8), None);
}

#[test]
fn respects_the_short_argument() {
    let td = tempdir().expect("tempdir");
    let sha_file = td.path().join(".hermes_build_sha");
    fs::write(&sha_file, format!("{FULL_SHA}\n")).expect("write sha");

    assert_eq!(
        get_build_sha_with(&sha_file, 12).as_deref(),
        Some("abcdef123456")
    );
    assert_eq!(get_build_sha_with(&sha_file, 0).as_deref(), Some(FULL_SHA));
    assert_eq!(get_build_sha_with(&sha_file, -1).as_deref(), Some(FULL_SHA));
    assert_eq!(get_build_sha_at(&sha_file, 8).as_deref(), Some("abcdef12"));
}

// Source-derived branches from the module docstring: the read is fail-open on
// any IO/decoding problem, and a blank file is "no build sha".
#[test]
fn blank_and_unreadable_files_fail_open_to_none() {
    let td = tempdir().expect("tempdir");
    let sha_file = td.path().join(".hermes_build_sha");
    fs::write(&sha_file, "   \n").expect("write blank");
    assert_eq!(get_build_sha_at(&sha_file, 8), None);

    // A directory where the file is expected raises IsADirectoryError upstream.
    let directory = td.path().join("as-a-directory");
    fs::create_dir(&directory).expect("mkdir");
    assert_eq!(get_build_sha_at(&directory, 8), None);
}

#[test]
fn truncation_is_character_based_and_whitespace_trimmed() {
    let td = tempdir().expect("tempdir");
    let sha_file = td.path().join(".hermes_build_sha");
    fs::write(&sha_file, "  \u{1f7e9}abcdef0123456789  ").expect("write");
    // Python slices by code point, not byte: a 10-char request yields 10 chars.
    assert_eq!(
        get_build_sha_at(&sha_file, 10).as_deref(),
        Some("\u{1f7e9}abcdef012")
    );
}
