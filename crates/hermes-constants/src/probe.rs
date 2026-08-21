//! Environment / filesystem probe abstraction.
//!
//! Upstream `hermes_constants.py` reads `os.environ`, `Path.home()`, and a few
//! well-known files (`/proc/version`, `/.dockerenv`, `/proc/1/cgroup`). Tests
//! monkeypatch those reads. This trait is the Rust equivalent: the public
//! functions always use [`RealProbe`]; unit tests inject a fake probe.
//!
//! PARITY: hermes_constants.py lines 1–18 (imports), 53–78, 1155–1282.

use std::path::{Path, PathBuf};

/// Abstraction over the environment and the few filesystem probes the
/// platform-detection helpers need.
pub trait Probe {
    /// Read an environment variable (`None` when unset/empty-equivalent is
    /// caller's job; mirror Python `os.environ.get(k, "")`).
    fn env(&self, key: &str) -> Option<String>;
    /// Is `path` present on disk (Python `os.path.exists`)?
    fn file_exists(&self, path: &Path) -> bool;
    /// Read a whole file as UTF-8 (Python `open(path).read()`; returns `None`
    /// on any I/O error).
    fn read_file(&self, path: &Path) -> Option<String>;
    /// The user's home directory (Python `Path.home()`).
    fn home_dir(&self) -> Option<PathBuf>;
}

/// The real environment / filesystem. Used by all public API.
pub struct RealProbe;

impl Probe for RealProbe {
    fn env(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    fn file_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn read_file(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    fn home_dir(&self) -> Option<PathBuf> {
        // Python's `Path.home()` expands `~` -> $HOME (or passwd). We read
        // $HOME first (so tests can repoint it, as upstream tests repoint
        // `Path.home`), then %USERPROFILE% on Windows.
        if let Some(h) = std::env::var_os("HOME") {
            let h: PathBuf = h.into();
            if !h.as_os_str().is_empty() {
                return Some(h);
            }
        }
        #[cfg(windows)]
        if let Some(h) = std::env::var_os("USERPROFILE") {
            return Some(h.into());
        }
        // Last resort so platform_default_home stays total even in a weird
        // container; upstream would raise RuntimeError from Path.home().
        std::env::current_dir().ok()
    }
}

#[cfg(test)]
pub(crate) mod fakes {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Deterministic probe for unit tests.
    #[derive(Default)]
    pub struct FakeProbe {
        pub env: Mutex<HashMap<String, String>>,
        pub files: Mutex<HashMap<PathBuf, String>>, // path -> content
        pub exists: Mutex<Vec<PathBuf>>,            // paths that exist w/o content
        pub home: Mutex<Option<PathBuf>>,
    }

    impl FakeProbe {
        pub fn new() -> Self {
            Self::default()
        }
        pub fn set_env(&self, k: &str, v: &str) {
            self.env.lock().unwrap().insert(k.to_string(), v.to_string());
        }
        pub fn add_file(&self, p: impl AsRef<Path>, content: impl AsRef<str>) {
            self.files
                .lock()
                .unwrap()
                .insert(p.as_ref().to_path_buf(), content.as_ref().to_string());
        }
        pub fn add_path(&self, p: impl AsRef<Path>) {
            self.exists.lock().unwrap().push(p.as_ref().to_path_buf());
        }
    }

    impl Probe for FakeProbe {
        fn env(&self, key: &str) -> Option<String> {
            self.env.lock().unwrap().get(key).cloned()
        }
        fn file_exists(&self, path: &Path) -> bool {
            self.exists.lock().unwrap().contains(&path.to_path_buf())
                || self.files.lock().unwrap().contains_key(path)
        }
        fn read_file(&self, path: &Path) -> Option<String> {
            self.files.lock().unwrap().get(path).cloned()
        }
        fn home_dir(&self) -> Option<PathBuf> {
            self.home.lock().unwrap().clone()
        }
    }
}
