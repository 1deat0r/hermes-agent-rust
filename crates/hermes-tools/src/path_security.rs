//! Shared path validation helpers for tool implementations.
//!
//! PARITY: tools/path_security.py @ 5d59366 (whole module).
//!
//! PLUGIN-COMPAT (upstream lines 24-41, revert-scheduled): the lazy `logger`
//! re-export (`tools.approval.logger`, PEP 562 `__getattr__`) is PENDING with
//! the `tools.approval` row (missing) — Rust has no module-`__getattr__`, so
//! the re-export lands as a plain `pub use` when that module ports. Internal
//! code must not use it (upstream CI enforces the same via
//! `check_compat_pointers.py`).

use std::path::Path;

/// Resolve a path like Python's `Path.resolve()`: follows symlinks and
/// normalizes `..` even when the final component doesn't exist yet (Rust's
/// `canonicalize` requires existence).
fn resolve(path: &Path) -> std::io::Result<std::path::PathBuf> {
    if let Ok(c) = path.canonicalize() {
        return Ok(c);
    }
    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    let mut cur = path;
    loop {
        match cur.canonicalize() {
            Ok(c) => {
                let mut resolved = c;
                for comp in missing.iter().rev() {
                    resolved.push(comp);
                }
                return Ok(resolved);
            }
            Err(_) => match cur.file_name() {
                Some(name) => {
                    missing.push(name.to_os_string());
                    match cur.parent() {
                        Some(p) => cur = p,
                        None => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::NotFound,
                                "cannot resolve path",
                            ))
                        }
                    }
                }
                None => return Ok(path.to_path_buf()),
            },
        }
    }
}

// PARITY: `has_traversal_component` — cheap pre-check for a literal `..`
// path component before full resolution. `Path::components` normalizes, so
// the check matches on `Component::ParentDir` exactly (no substring false
// positives on `...` or `..b`). `"a/../b"` yields `[a, ParentDir, b]`
// lexically (no I/O) — `true`, matching `".." in Path.parts`.
pub fn has_traversal_component(path_str: &str) -> bool {
    use std::path::Component;
    Path::new(path_str)
        .components()
        .any(|c| matches!(c, Component::ParentDir))
}

/// Ensure *path* resolves to a location within *root*. Returns an error
/// message string when validation fails, or None when safe.
pub fn validate_within_dir(path: &Path, root: &Path) -> Option<String> {
    match (resolve(path), resolve(root)) {
        (Ok(resolved), Ok(root_resolved)) => {
            if resolved.starts_with(&root_resolved) {
                None
            } else {
                Some(format!(
                    "path {} resolves outside the allowed root {}",
                    resolved.display(),
                    root_resolved.display()
                ))
            }
        }
        (Err(e), _) => Some(format!("failed to resolve path {}: {e}", path.display())),
        (_, Err(e)) => Some(format!("failed to resolve root {}: {e}", root.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_within_dir() {
        let dir = std::env::temp_dir().join("hfs_path_test");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(validate_within_dir(&dir.join("a.txt"), &dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_outside_dir() {
        let dir = std::env::temp_dir().join("hfs_path_test2");
        std::fs::create_dir_all(&dir).unwrap();
        let outside = std::env::temp_dir();
        let err = validate_within_dir(&outside, &dir);
        assert!(err.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
