//! Git checkout fingerprinting without spawning `git`.
//!
//! PARITY: `hermes_cli/main.py` `_read_packed_ref` (upstream lines ~820-836)
//! and `_read_git_revision_fingerprint` (upstream lines 840-882) @ b9aa928.
//!
//! Upstream keeps these module-private inside `hermes_cli/main.py`; they are
//! public here because `gateway/code_skew.py` imports
//! `_read_git_revision_fingerprint` across the package boundary (the Rust
//! crate boundary matches that seam).
//!
//! Behaviour: a cheap, spawn-free `git:<ref>:<sha>` fingerprint of a
//! checkout. Any `OSError` (missing `.git`, unreadable files) makes the whole
//! read return `None` — callers treat that as "no revision information" and
//! fall back, never as a hard error. Non-UTF-8 bytes are replaced, matching
//! upstream's `errors="replace"` reads.

use std::fs;
use std::path::{Path, PathBuf};

/// `read_text(encoding="utf-8", errors="replace")` equivalent: lossy decode,
/// error → `None` (the OSError arm).
fn read_text_lossy(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// PARITY: `Path.resolve()` (non-strict) — canonicalize when the path exists,
/// otherwise keep the joined path (Python resolves as far as it can).
fn resolve_non_strict(path: &Path) -> PathBuf {
    match fs::canonicalize(path) {
        Ok(resolved) => resolved,
        Err(_) => path.to_path_buf(),
    }
}

/// Look up a ref in `<common_dir>/packed-refs` without spawning git.
///
/// packed-refs lines look like `<sha> <ref>` with optional `^<sha>` peel
/// lines and `#`-prefixed comments / `# pack-refs with:` header.
///
/// PARITY: `_read_packed_ref` (upstream `_read_packed_ref`, whole function).
pub fn read_packed_ref(common_dir: &Path, ref_name: &str) -> Option<String> {
    let text = read_text_lossy(&common_dir.join("packed-refs"))?;
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') || line.starts_with('^') {
            continue;
        }
        if let Some((sha, candidate)) = line.split_once(' ') {
            if candidate.trim() == ref_name {
                return Some(sha.trim().to_string());
            }
        }
    }
    None
}

/// Return a cheap checkout fingerprint (`git:<ref>:<sha>`) without spawning
/// git.
///
/// Handles: detached HEAD (fingerprint `git:HEAD:<head>`), loose refs in the
/// worktree gitdir or the common dir, packed refs, and the `unresolved`
/// marker when the ref name is known but no sha can be found (still stable
/// across launches, so callers only invalidate after `hermes update`).
///
/// PARITY: `_read_git_revision_fingerprint` (upstream lines 840-882).
pub fn read_git_revision_fingerprint(repo_root: &Path) -> Option<String> {
    let mut git_dir = repo_root.join(".git");
    // `if git_dir.is_file():` — worktrees/checkouts linked via a `.git` file.
    if git_dir.is_file() {
        let text = read_text_lossy(&git_dir)?;
        for line in text.lines() {
            let (key, value) = line.split_once(':').unwrap_or(("", ""));
            if key.trim() == "gitdir" && !value.trim().is_empty() {
                git_dir = resolve_non_strict(&repo_root.join(value.trim()));
                break;
            }
        }
    }
    // Worktrees point HEAD at a per-worktree gitdir but pack their refs in
    // the main repo's gitdir (referenced via `commondir`). Resolve that up
    // front so packed-refs lookups hit the right file.
    let mut common_dir = git_dir.clone();
    if git_dir.join("commondir").exists() {
        if let Some(rel) = read_text_lossy(&git_dir.join("commondir")) {
            let rel = rel.trim();
            if !rel.is_empty() {
                common_dir = resolve_non_strict(&git_dir.join(rel));
            }
        }
    }
    let head = read_text_lossy(&git_dir.join("HEAD"))?.trim().to_string();
    if let Some(reference) = head.strip_prefix("ref:") {
        let reference = reference.trim();
        // Loose refs may live in the worktree gitdir OR the common dir
        // (branches created via `git worktree add` typically live in the
        // common dir's refs/heads/).
        for candidate in [&git_dir, &common_dir] {
            let ref_file = candidate.join(reference);
            if ref_file.exists() {
                return Some(format!(
                    "git:{reference}:{}",
                    read_text_lossy(&ref_file)?.trim()
                ));
            }
        }
        if let Some(packed_sha) = read_packed_ref(&common_dir, reference) {
            return Some(format!("git:{reference}:{packed_sha}"));
        }
        // Ref name is known but unresolved — still stable across launches,
        // and the version/release fallback in the caller will invalidate
        // after `hermes update`.
        return Some(format!("git:{reference}:unresolved"));
    }
    Some(format!("git:HEAD:{head}"))
}
