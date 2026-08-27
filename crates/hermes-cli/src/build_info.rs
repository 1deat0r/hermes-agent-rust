//! Baked-in build metadata for Hermes Agent.
//!
//! PARITY: `hermes_cli/build_info.py` @ b9aa928 (whole module, lines 1-56).
//!
//! Source installs report their git revision live via `git rev-parse` (see
//! `hermes_cli/dump.py` and `hermes_cli/banner.py`). That does not work inside
//! the published Docker image because `.dockerignore` excludes `.git`, so those
//! callsites fall back to `"(unknown)"` or drop the banner suffix entirely.
//!
//! To make `hermes dump` and the startup banner identify the exact commit the
//! image was built from, the Docker build writes the build-time
//! `$HERMES_GIT_SHA` arg into `<project_root>/.hermes_build_sha`. This module is
//! the single read-side helper consumed by both callsites, so the file path and
//! missing-file behaviour stay consistent.
//!
//! Behaviour: `None` when the file is absent (source installs and dev images
//! built without the `HERMES_GIT_SHA` build-arg fall through to live-git
//! resolution in the caller); `None` on any IO or decoding error, because the
//! build SHA is a nice-to-have for support triage and nothing in the CLI may
//! crash because of it; and truncation to `short` characters to match the
//! `git rev-parse --short=8` format used throughout the codebase.
//!
//! LAYERING NOTE: the source resolves the path from `__file__`'s parent's
//! parent, i.e. the checkout or wheel root the module was imported from. A Rust
//! library has no such anchor at runtime, so [`build_sha_file`] resolves the
//! repository root recorded at compile time and honours the `HERMES_BUILD_SHA_FILE`
//! environment override; every function also has an explicit-path form, which is
//! the equivalent of the upstream tests patching `_BUILD_SHA_FILE`.

use std::path::{Path, PathBuf};

/// Default truncation width, matching `get_build_sha(short: int = 8)`.
pub const DEFAULT_SHORT: usize = 8;

/// PARITY: `_BUILD_SHA_FILE` (upstream line 31).
///
/// `HERMES_BUILD_SHA_FILE` overrides the compile-time repository root so a
/// packaged install can point at its own metadata file.
pub fn build_sha_file() -> PathBuf {
    if let Some(override_path) = std::env::var_os("HERMES_BUILD_SHA_FILE") {
        let path = PathBuf::from(override_path);
        if !path.as_os_str().is_empty() {
            return path;
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join(".hermes_build_sha"))
        .unwrap_or_else(|| PathBuf::from(".hermes_build_sha"))
}

/// Read the baked-in build SHA from [`build_sha_file`].
///
/// PARITY: `get_build_sha` (upstream lines 34-56).
pub fn get_build_sha(short: usize) -> Option<String> {
    get_build_sha_at(&build_sha_file(), short)
}

/// Explicit-path form of [`get_build_sha`], the equivalent of the upstream
/// tests patching `_BUILD_SHA_FILE`. `short` keeps Python's keyword semantics:
/// `0` (falsy) and any negative width return the whole value.
pub fn get_build_sha_with(path: &Path, short: i64) -> Option<String> {
    let sha = read_sha(path)?;
    Some(truncate(&sha, short))
}

/// Explicit-path form of [`get_build_sha`].
pub fn get_build_sha_at(path: &Path, short: usize) -> Option<String> {
    let sha = read_sha(path)?;
    Some(truncate(&sha, short as i64))
}

fn read_sha(path: &Path) -> Option<String> {
    // `if not _BUILD_SHA_FILE.is_file(): return None` — a directory or any
    // other non-regular entry reports no build sha rather than raising.
    if !path.is_file() {
        return None;
    }
    let sha = std::fs::read_to_string(path).ok()?.trim().to_string();
    (!sha.is_empty()).then_some(sha)
}

/// `sha[:short] if short and short > 0 else sha`, sliced by code point like
/// Python. A zero (falsy) or negative width returns the whole value.
fn truncate(sha: &str, short: i64) -> String {
    if short <= 0 {
        return sha.to_string();
    }
    sha.chars().take(short as usize).collect()
}
