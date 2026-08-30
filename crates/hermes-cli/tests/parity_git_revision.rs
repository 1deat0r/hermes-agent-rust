//! Parity tests for `hermes_cli/main.py` `_read_packed_ref` and
//! `_read_git_revision_fingerprint` @ b9aa928.
//!
//! Upstream has no dedicated test file for these two helpers (they are
//! exercised indirectly through `tests/test_code_skew.py` and the dump /
//! banner surfaces) — the missing-test gap is noted in the ledger. These
//! cases derive from the upstream code as oracle: real `.git` fixtures built
//! in a temp dir, covering detached HEAD, loose refs, packed refs, the
//! `unresolved` marker, worktree `.git`-file indirection with `commondir`,
//! and the OSError fail-open.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

#[test]
fn detached_head_fingerprints_as_git_head() {
    let td = TempDir::new().unwrap();
    let repo = td.path();
    write(
        &repo.join(".git/HEAD"),
        "0123456789abcdef0123456789abcdef01234567\n",
    );
    assert_eq!(
        hermes_cli::git_revision::read_git_revision_fingerprint(repo).unwrap(),
        "git:HEAD:0123456789abcdef0123456789abcdef01234567"
    );
}

#[test]
fn loose_ref_read_from_gitdir() {
    let td = TempDir::new().unwrap();
    let repo = td.path();
    write(&repo.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(
        &repo.join(".git/refs/heads/main"),
        "1111111111111111111111111111111111111111\n",
    );
    assert_eq!(
        hermes_cli::git_revision::read_git_revision_fingerprint(repo).unwrap(),
        "git:refs/heads/main:1111111111111111111111111111111111111111"
    );
}

#[test]
fn packed_ref_used_when_loose_missing() {
    let td = TempDir::new().unwrap();
    let repo = td.path();
    write(&repo.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(
        &repo.join(".git/packed-refs"),
        "# pack-refs with: peeled fully-peeled sorted \n\
         2222222222222222222222222222222222222222 refs/remotes/origin/main\n\
         ^3333333333333333333333333333333333333333\n\
         4444444444444444444444444444444444444444 refs/heads/main\n",
    );
    assert_eq!(
        hermes_cli::git_revision::read_git_revision_fingerprint(repo).unwrap(),
        "git:refs/heads/main:4444444444444444444444444444444444444444"
    );
}

#[test]
fn known_but_missing_ref_reports_unresolved() {
    let td = TempDir::new().unwrap();
    let repo = td.path();
    write(&repo.join(".git/HEAD"), "ref: refs/heads/feature\n");
    assert_eq!(
        hermes_cli::git_revision::read_git_revision_fingerprint(repo).unwrap(),
        "git:refs/heads/feature:unresolved"
    );
}

#[test]
fn worktree_git_file_resolves_commondir_for_packed_refs() {
    let td = TempDir::new().unwrap();
    let repo = td.path();
    // Main repo packs the branch; the worktree points at its own gitdir.
    write(&repo.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(
        &repo.join(".git/packed-refs"),
        "5555555555555555555555555555555555555555 refs/heads/main\n",
    );
    let worktree_git = repo.join(".git/worktrees/w1");
    write(&worktree_git.join("HEAD"), "ref: refs/heads/main\n");
    write(&worktree_git.join("commondir"), "../..\n");
    // `.git` of the worktree checkout is a file.
    let checkout = td.path().join("checkout");
    fs::create_dir_all(&checkout).unwrap();
    write(
        &checkout.join(".git"),
        &format!("gitdir: {}\n", worktree_git.display()),
    );
    assert_eq!(
        hermes_cli::git_revision::read_git_revision_fingerprint(&checkout).unwrap(),
        "git:refs/heads/main:5555555555555555555555555555555555555555"
    );
}

#[test]
fn non_git_directory_fails_open_to_none() {
    let td = TempDir::new().unwrap();
    assert!(hermes_cli::git_revision::read_git_revision_fingerprint(td.path()).is_none());
}

#[test]
fn read_packed_ref_skips_comments_peels_and_mismatches() {
    let td = TempDir::new().unwrap();
    let dir = td.path();
    write(
        &dir.join("packed-refs"),
        "# pack-refs with: peeled fully-peeled sorted \n\
         aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa refs/heads/a\n\
         ^bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n\
         cccccccccccccccccccccccccccccccccccccccc refs/tags/a\n",
    );
    let read = |ref_name: &str| hermes_cli::git_revision::read_packed_ref(dir, ref_name);
    assert_eq!(
        read("refs/heads/a").unwrap(),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert_eq!(
        read("refs/tags/a").unwrap(),
        "cccccccccccccccccccccccccccccccccccccccc"
    );
    assert_eq!(read("refs/heads/b"), None);
    // No leading-sha peel lines (`^...`) are ever matched as refs.
    assert_eq!(read("^bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"), None);
}

#[test]
fn read_packed_ref_missing_file_is_none() {
    let td = TempDir::new().unwrap();
    assert!(hermes_cli::git_revision::read_packed_ref(td.path(), "refs/heads/main").is_none());
}
