//! Venv layout helpers.
//!
//! PARITY: hermes_constants.py lines 1370–1404 (`venv_bin_dir`,
//! `venv_python_path`).

use crate::home::Platform;
use std::path::{Path, PathBuf};

/// Directory holding a venv's executables (`Scripts` / `bin`).
///
/// `windows` lets callers pass their own platform verdict, mirroring the
/// `windows: bool | None` parameter (tests patch platform predicates).
/// Defaults to the host platform.
///
/// PARITY: hermes_constants.py `venv_bin_dir` (1370–1391).
pub fn venv_bin_dir(venv_dir: impl AsRef<Path>, windows: Option<Platform>) -> PathBuf {
    let windows = windows.unwrap_or_else(Platform::host) == Platform::Windows;
    PathBuf::from(venv_dir.as_ref()).join(if windows { "Scripts" } else { "bin" })
}

/// Path to the Python interpreter inside `venv_dir` (may not exist).
///
/// PARITY: hermes_constants.py `venv_python_path` (1393–1399).
pub fn venv_python_path(venv_dir: impl AsRef<Path>, windows: Option<Platform>) -> PathBuf {
    venv_bin_dir(venv_dir, windows).join(if windows.unwrap_or_else(Platform::host) == Platform::Windows {
        "python.exe"
    } else {
        "python"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posix_layout() {
        assert_eq!(
            venv_bin_dir("/venv", Some(Platform::Posix)),
            PathBuf::from("/venv/bin")
        );
        assert_eq!(
            venv_python_path("/venv", Some(Platform::Posix)),
            PathBuf::from("/venv/bin/python")
        );
    }

    #[test]
    fn windows_layout() {
        assert_eq!(
            venv_bin_dir("/venv", Some(Platform::Windows)),
            PathBuf::from("/venv/Scripts")
        );
        assert_eq!(
            venv_python_path("/venv", Some(Platform::Windows)),
            PathBuf::from("/venv/Scripts/python.exe")
        );
    }
}
