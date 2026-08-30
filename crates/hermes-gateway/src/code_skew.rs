//! Detect when the gateway is running stale code after a hot `git pull`.
//!
//! PARITY: `gateway/code_skew.py` @ b9aa928 (whole module).
//!
//! The gateway is a single long-lived process; its module table is frozen at
//! boot. If the checkout is updated underneath it (a manual `git pull`, or
//! the window before `hermes update`'s graceful restart fires), a first-time
//! lazy import on a new code path can resolve a freshly-pulled consumer
//! module against a stale cached dependency → ImportError.
//!
//! We snapshot the checkout revision at gateway startup and compare on
//! demand, so risky callers (e.g. `/model` switching) can refuse with a clear
//! "restart the gateway" message instead of crashing on a cryptic import
//! error. If the revision can't be read (non-git install, IO error), the boot
//! snapshot stays `None` and skew detection no-ops — it never produces a
//! false positive.
//!
//! LAYERING NOTE: upstream anchors `_PROJECT_ROOT` at `__file__`'s parent's
//! parent. A Rust library has no such runtime anchor, so [`project_root`]
//! resolves the compile-time workspace root and honours a
//! `HERMES_PROJECT_ROOT` environment override; the `_at` forms take an
//! explicit root, which is the equivalent of the upstream tests patching
//! `_fingerprint`.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use hermes_cli::git_revision;

/// PARITY: `_PROJECT_ROOT` (upstream line 21).
///
/// `HERMES_PROJECT_ROOT` overrides the compile-time anchor so a packaged
/// install can point at its own checkout.
pub fn project_root() -> PathBuf {
    if let Some(root) = std::env::var_os("HERMES_PROJECT_ROOT") {
        let root = PathBuf::from(root);
        if !root.as_os_str().is_empty() {
            return root;
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// PARITY: `_boot_fingerprint` module global (upstream line 22).
static BOOT_FINGERPRINT: RwLock<Option<String>> = RwLock::new(None);

/// Test-only seam standing in for the upstream tests' `monkeypatch` of
/// `_fingerprint` / `_boot_fingerprint`. When set, [`fingerprint`] returns
/// this value instead of reading disk.
static FINGERPRINT_OVERRIDE: RwLock<Option<Option<String>>> = RwLock::new(None);

/// Current checkout fingerprint, reusing the CLI's git-rev reader.
///
/// PARITY: `_fingerprint` (upstream lines 25-33). Upstream wraps the import
/// and call in a broad `except Exception: return None` — fail-open, never a
/// false positive.
pub fn fingerprint_at(project_root: &Path) -> Option<String> {
    git_revision::read_git_revision_fingerprint(project_root)
}

fn fingerprint() -> Option<String> {
    if let Ok(guard) = FINGERPRINT_OVERRIDE.read() {
        if let Some(value) = guard.as_ref() {
            return value.clone();
        }
    }
    fingerprint_at(&project_root())
}

/// Snapshot the checkout revision at gateway startup (idempotent).
///
/// PARITY: `record_boot_fingerprint` (upstream lines 36-39).
pub fn record_boot_fingerprint() {
    let mut boot = match BOOT_FINGERPRINT.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if boot.is_none() {
        *boot = fingerprint();
    }
}

/// Render a `git:<ref>:<sha>` fingerprint as a compact label.
///
/// PARITY: `_short` (upstream lines 42-48).
pub fn short(fingerprint: &str) -> String {
    let sha = fingerprint.rsplit(':').next().unwrap_or("");
    if !sha.is_empty() && sha != "unresolved" && sha.len() > 10 {
        return sha.chars().take(10).collect();
    }
    if sha.is_empty() {
        // Python `return sha or fingerprint` — empty falls back to the whole
        // fingerprint.
        return fingerprint.to_string();
    }
    sha.to_string()
}

/// Return `(boot_rev, disk_rev)` short labels if the checkout drifted since
/// boot, else `None`.
///
/// PARITY: `detect_code_skew` (upstream lines 51-58).
pub fn detect_code_skew() -> Option<(String, String)> {
    let boot = BOOT_FINGERPRINT
        .read()
        .ok()
        .and_then(|guard| guard.clone())?;
    let current = fingerprint();
    if current.is_none() || current.as_deref() == Some(boot.as_str()) {
        return None;
    }
    let current = current?;
    Some((short(&boot), short(&current)))
}

/// Reset the module globals — the Rust stand-in for the upstream tests'
/// autouse `_reset_boot_fingerprint` fixture.
#[doc(hidden)]
pub fn reset_for_tests() {
    *BOOT_FINGERPRINT.write().unwrap_or_else(|e| e.into_inner()) = None;
    *FINGERPRINT_OVERRIDE
        .write()
        .unwrap_or_else(|e| e.into_inner()) = None;
}

/// Set the fingerprint-override test seam. `None` clears it.
#[doc(hidden)]
pub fn set_fingerprint_override_for_tests(value: Option<Option<String>>) {
    *FINGERPRINT_OVERRIDE
        .write()
        .unwrap_or_else(|e| e.into_inner()) = value;
}
