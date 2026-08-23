//! Parity oracles for working-diff collection, mirroring upstream
//! tests/tools/test_working_diff.py @ b9aa928.
//!
//! Tier: mock/unit (live git binary in temporary repos; no mocks).
//! Upstream runs the whole suite against real temporary git repositories
//! (skipped when git is not installed); this port does the same.
//!
//! The upstream file contains exactly three tests (clean repo reports
//! empty, unstaged change in default mode, unknown mode rejected). This
//! port mirrors those and adds code-oracle tests for the untracked-file
//! fold-in, `staged` / `all` modes, pathspec restriction, and the
//! not-a-repo failure — behaviors the upstream module implements but its
//! test file does not pin (upstream test gap, noted).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use hermes_tools::working_diff::collect_working_diff;

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("HOME", repo)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed");
}

/// Mirror of the upstream `repo` fixture: a fresh repo with one committed
/// file (`tracked.py`).
fn repo_fixture(tmp: &tempfile::TempDir) -> PathBuf {
    let d = tmp.path().join("repo");
    fs::create_dir_all(&d).unwrap();
    git(&d, &["init", "-q"]);
    fs::write(d.join("tracked.py"), "print('hello')\n").unwrap();
    git(&d, &["add", "-A"]);
    git(&d, &["commit", "-q", "-m", "init"]);
    d
}

// --- Upstream-mirrored tests ------------------------------------------------

#[test]
fn clean_repo_reports_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_fixture(&tmp);
    let result = collect_working_diff(&repo, "working", None);
    assert!(result.success);
    assert_eq!(result.empty, Some(true));
    assert_eq!(result.diff.as_deref(), Some(""));
    assert_eq!(result.untracked.as_deref(), Some([].as_slice()));
}

#[test]
fn unstaged_change_appears_in_default_mode() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_fixture(&tmp);
    fs::write(repo.join("tracked.py"), "print('changed')\n").unwrap();
    let result = collect_working_diff(&repo, "working", None);
    assert!(result.success);
    let diff = result.diff.unwrap_or_default();
    assert!(diff.contains("-print('hello')"), "diff: {diff}");
    assert!(diff.contains("+print('changed')"), "diff: {diff}");
    assert!(result.stat.unwrap_or_default().contains("tracked.py"));
    assert_eq!(result.empty, None);
}

#[test]
fn unknown_mode_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_fixture(&tmp);
    let result = collect_working_diff(&repo, "bogus", None);
    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("bogus"));
}

// --- Code-oracle extra tests (upstream module behavior without a test) ----

#[test]
fn staged_mode_shows_only_staged_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_fixture(&tmp);
    // One staged change and one unstaged change.
    fs::write(repo.join("tracked.py"), "print('staged')\n").unwrap();
    git(&repo, &["add", "tracked.py"]);
    fs::write(repo.join("tracked.py"), "print('unstaged')\n").unwrap();

    let result = collect_working_diff(&repo, "staged", None);
    assert!(result.success);
    let diff = result.diff.unwrap_or_default();
    assert!(diff.contains("+print('staged')"), "staged diff: {diff}");
    assert!(
        !diff.contains("+print('unstaged')"),
        "staged leaked: {diff}"
    );
    assert!(result.stat.unwrap_or_default().contains("tracked.py"));
}

#[test]
fn all_mode_combines_staged_and_unstaged() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_fixture(&tmp);
    fs::write(repo.join("tracked.py"), "print('one')\n").unwrap();
    git(&repo, &["add", "tracked.py"]);
    fs::write(repo.join("tracked.py"), "print('two')\n").unwrap();

    let result = collect_working_diff(&repo, "all", None);
    assert!(result.success);
    let diff = result.diff.unwrap_or_default();
    assert!(diff.contains("-print('hello')"), "diff: {diff}");
    assert!(diff.contains("+print('two')"), "diff: {diff}");
}

#[test]
fn untracked_file_folded_into_working_diff() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_fixture(&tmp);
    fs::write(repo.join("new.py"), "print('untracked')\n").unwrap();

    let result = collect_working_diff(&repo, "working", None);
    assert!(result.success);
    assert_eq!(
        result.untracked.as_deref(),
        Some(vec!["new.py".to_string()].as_slice())
    );
    let diff = result.diff.unwrap_or_default();
    assert!(diff.contains("+print('untracked')"), "diff: {diff}");
}

#[test]
fn empty_path_list_behaves_like_no_path_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_fixture(&tmp);
    fs::write(repo.join("new.py"), "print('untracked')\n").unwrap();

    let empty_paths: Vec<String> = Vec::new();
    let result = collect_working_diff(&repo, "working", Some(&empty_paths));
    assert!(result.success);
    assert_eq!(
        result.untracked.as_deref(),
        Some(vec!["new.py".to_string()].as_slice())
    );
}

#[test]
fn untracked_cap_reports_more_not_shown() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_fixture(&tmp);
    for i in 0..55 {
        fs::write(repo.join(format!("u{i}.py")), "x\n").unwrap();
    }
    let result = collect_working_diff(&repo, "working", None);
    assert!(result.success);
    let diff = result.diff.unwrap_or_default();
    assert!(
        diff.contains("... (5 more untracked files not shown)"),
        "diff tail: {}",
        &diff[diff.len().saturating_sub(200)..]
    );
}

#[test]
fn pathspec_restricts_diff() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_fixture(&tmp);
    fs::write(repo.join("tracked.py"), "print('changed')\n").unwrap();
    fs::write(repo.join("other.py"), "print('other')\n").unwrap();

    let paths = vec!["tracked.py".to_string()];
    let result = collect_working_diff(&repo, "working", Some(&paths));
    assert!(result.success);
    let diff = result.diff.unwrap_or_default();
    assert!(diff.contains("tracked.py"), "diff: {diff}");
    assert!(!diff.contains("other.py"), "pathspec leaked: {diff}");
}

#[test]
fn not_a_git_repository() {
    let tmp = tempfile::tempdir().unwrap();
    let plain = tmp.path().join("plain");
    fs::create_dir_all(&plain).unwrap();
    let result = collect_working_diff(&plain, "working", None);
    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("Not a git repository."));
}

#[test]
fn unknown_mode_takes_precedence_over_git_availability() {
    let tmp = tempfile::tempdir().unwrap();
    let plain = tmp.path().join("plain");
    fs::create_dir_all(&plain).unwrap();
    let result = collect_working_diff(&plain, "bogus", None);
    assert!(!result.success);
    assert!(result
        .error
        .unwrap_or_default()
        .contains("Unknown mode 'bogus'"));
}
