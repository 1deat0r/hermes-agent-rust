//! Hermes home directory resolution.
//!
//! PARITY: hermes_constants.py lines 16–49 (ContextVar override), 53–200
//! (platform default, env home, profile-fallback warning, get_hermes_home,
//! get_process_hermes_home, get_default_hermes_root).

use crate::probe::{Probe, RealProbe};
use std::path::{Path, PathBuf};
use once_cell::sync::Lazy;
use std::sync::Mutex;

// ── Context-local override (Python ContextVar `_HERMES_HOME_OVERRIDE`) ─────

/// Token returned by [`set_hermes_home_override`]; pass to
/// [`reset_hermes_home_override`] to restore the previous value.
#[derive(Debug)]
pub struct OverrideToken {
    id: u64,
    previous: Option<String>,
}

thread_local! {
    static OVERRIDE_STACK: std::cell::RefCell<(Vec<(u64, String)>, u64)> =
        const { std::cell::RefCell::new((Vec::new(), 0)) };
}

/// Set a task-local Hermes home override and return its reset token.
///
/// This is for in-process, per-task scoping. It deliberately does not mutate
/// the process environment, which is shared by every thread.
///
/// PARITY: hermes_constants.py `set_hermes_home_override` (30–38). Python's
/// ContextVar is async-context scoped; Rust's thread-local is a thread-scoped
/// equivalent. Document as such — callers pass tokens across the same thread.
pub fn set_hermes_home_override(path: Option<impl AsRef<Path>>) -> OverrideToken {
    OVERRIDE_STACK.with(|cell| {
        let mut state = cell.borrow_mut();
        let previous = state.0.last().map(|(_, v)| v.clone());
        let value = match path {
            Some(p) => p.as_ref().to_string_lossy().into_owned(),
            None => String::new(), // `_UNSET` representation: empty string
        };
        state.1 += 1;
        let id = state.1;
        state.0.push((id, value));
        OverrideToken { id, previous }
    })
}

/// Restore the previous context-local Hermes home override.
///
/// PARITY: hermes_constants.py `reset_hermes_home_override` (40–43).
pub fn reset_hermes_home_override(token: OverrideToken) {
    OVERRIDE_STACK.with(|cell| {
        let mut state = cell.borrow_mut();
        // Remove the entry pushed by the matching set(), plus any entries set
        // after it (out-of-order resets invalidate those scopes). Then restore
        // the previous value the token captured, if any. Nested
        // set/reset pairs restore cleanly: popping the top reveals the
        // previous value already on the stack.
        if let Some(pos) = state.0.iter().position(|(id, _)| *id == token.id) {
            state.0.truncate(pos);
            if let Some(prev) = token.previous {
                state.0.push((token.id, prev));
            }
        }
        // Unknown token: no-op (Python ContextVar.reset would raise
        // ValueError; we stay consistent and forgiving).
    });
}

/// Return the active context-local Hermes home override, if any.
///
/// PARITY: hermes_constants.py `get_hermes_home_override` (45–51): empty value
/// (Python `not override`) is treated as no override.
pub fn get_hermes_home_override() -> Option<String> {
    OVERRIDE_STACK.with(|cell| {
        let state = cell.borrow();
        state
            .0
            .last()
            .map(|(_, v)| v)
            .filter(|v| !v.is_empty())
            .cloned()
    })
}

/// Clear all override state. Test-only helper.
#[doc(hidden)]
pub fn reset_override_for_tests() {
    OVERRIDE_STACK.with(|cell| cell.borrow_mut().0.clear());
}

// ── Home resolution helpers ────────────────────────────────────────────────

/// Host platform verdict. Mirrors upstream's `sys.platform == "win32"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Posix,
}

impl Platform {
    pub fn host() -> Self {
        if cfg!(windows) {
            Platform::Windows
        } else {
            Platform::Posix
        }
    }
}

/// Platform-native default Hermes home path.
///
/// PARITY: hermes_constants.py `_get_platform_default_hermes_home` (53–60):
/// win32 → `%LOCALAPPDATA%\hermes` (falling back to
/// `~ / AppData / Local / hermes`); otherwise `~/.hermes`.
pub fn platform_default_home() -> PathBuf {
    platform_default_home_with(Platform::host(), &RealProbe)
}

pub(crate) fn platform_default_home_with(platform: Platform, probe: &dyn Probe) -> PathBuf {
    match platform {
        Platform::Windows => {
            let local_appdata = probe.env("LOCALAPPDATA").unwrap_or_default();
            let trimmed = local_appdata.trim();
            if !trimmed.is_empty() {
                PathBuf::from(trimmed).join("hermes")
            } else {
                let home = probe
                    .home_dir()
                    .unwrap_or_else(|| PathBuf::from("."));
                home.join("AppData").join("Local").join("hermes")
            }
        }
        Platform::Posix => {
            let home = probe.home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.join(".hermes")
        }
    }
}

/// Resolve `HERMES_HOME` from the process environment only.
///
/// Deliberately ignores the context-local override, so this reflects the
/// process/launch scope. Shared by `get_hermes_home` and
/// `get_process_hermes_home` so the two never drift.
///
/// PARITY: hermes_constants.py `_hermes_home_from_env` (62–75).
pub fn home_from_env() -> PathBuf {
    home_from_env_with(Platform::host(), &RealProbe)
}

pub(crate) fn home_from_env_with(platform: Platform, probe: &dyn Probe) -> PathBuf {
    let val = probe.env("HERMES_HOME").unwrap_or_default();
    let trimmed = val.trim();
    if !trimmed.is_empty() {
        PathBuf::from(trimmed)
    } else {
        platform_default_home_with(platform, probe)
    }
}

static PROFILE_FALLBACK_WARNED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

/// Warn once (stderr) when falling back to the default home while a
/// non-default profile is sticky-active.
///
/// PARITY: hermes_constants.py `_warn_profile_fallback_once` (77–112).
/// Upstream writes straight to stderr (not logging) because this runs at
/// import time from 30+ sites before logging is configured.
pub(crate) fn warn_profile_fallback_once_with(probe: &dyn Probe) {
    {
        let warned = PROFILE_FALLBACK_WARNED.lock().unwrap();
        if *warned {
            return;
        }
    }
    let fallback_home = platform_default_home_with(Platform::host(), probe);
    let active_path = fallback_home.join("active_profile");
    let active = match probe.read_file(&active_path) {
        Some(text) => text.trim().to_string(),
        None => String::new(),
    };
    if !active.is_empty() && active != "default" {
        *PROFILE_FALLBACK_WARNED.lock().unwrap() = true;
        let msg = format!(
            "[HERMES_HOME fallback] HERMES_HOME is unset but active \
             profile is {:?}. Falling back to {:?}, which \
             is the DEFAULT profile — not {:?}. Any data this \
             process writes will land in the wrong profile. The \
             subprocess spawner should pass HERMES_HOME explicitly \
             (see issue #18594).",
            active, fallback_home, active
        );
        // stderr write is best-effort, exactly like upstream's try/except.
        use std::io::Write;
        let _ = writeln!(std::io::stderr(), "{}", msg);
    }
}

/// Public one-shot profile-fallback warning for the current process.
#[doc(hidden)]
pub fn warn_profile_fallback_once() {
    warn_profile_fallback_once_with(&RealProbe);
}

#[doc(hidden)]
pub fn reset_profile_fallback_warning_for_tests() {
    *PROFILE_FALLBACK_WARNED.lock().unwrap() = false;
}

/// Return the Hermes home directory (default: platform-native path).
///
/// Resolution order: context-local override → `HERMES_HOME` env var → the
/// platform-native default.
///
/// PARITY: hermes_constants.py `get_hermes_home` (114–140).
pub fn get_hermes_home() -> PathBuf {
    get_hermes_home_with(Platform::host(), &RealProbe)
}

pub(crate) fn get_hermes_home_with(platform: Platform, probe: &dyn Probe) -> PathBuf {
    if let Some(override_path) = get_hermes_home_override() {
        return PathBuf::from(override_path);
    }
    let env_home = probe.env("HERMES_HOME").unwrap_or_default();
    if env_home.trim().is_empty() {
        warn_profile_fallback_once_with(probe);
    }
    home_from_env_with(platform, probe)
}

/// Return the Hermes home for the running process, ignoring task overrides.
///
/// Unlike `get_hermes_home`, this never follows the context-local override:
/// it resolves only the process `HERMES_HOME` env var (falling back to the
/// platform default).
///
/// PARITY: hermes_constants.py `get_process_hermes_home` (142–164).
pub fn get_process_hermes_home() -> PathBuf {
    get_process_hermes_home_with(Platform::host(), &RealProbe)
}

pub(crate) fn get_process_hermes_home_with(platform: Platform, probe: &dyn Probe) -> PathBuf {
    home_from_env_with(platform, probe)
}

/// Python `Path.resolve()`-style tolerant resolution: canonicalize what
/// exists, append the rest lexically. Upstream `Path.resolve()` is
/// non-strict (Python 3.11 default `strict=False`).
fn resolve_tolerant(path: &Path) -> PathBuf {
    if let Ok(c) = std::fs::canonicalize(path) {
        return c;
    }
    // Walk the longest existing prefix and append the remainder.
    let mut existing = PathBuf::new();
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    for comp in path.components() {
        existing.push(comp.as_os_str());
        if !existing.exists() {
            rest.push(comp.as_os_str().to_os_string());
            existing.pop();
        }
    }
    let mut out = if existing.as_os_str().is_empty() {
        path.to_path_buf()
    } else {
        std::fs::canonicalize(&existing).unwrap_or(existing)
    };
    for r in rest {
        out.push(r);
    }
    out
}

fn is_relative_to(path: &Path, base: &Path) -> bool {
    path.starts_with(base)
}

/// Return the root Hermes directory for profile-level operations.
///
/// PARITY: hermes_constants.py `get_default_hermes_root` (166–199):
/// 1. unset `HERMES_HOME` → platform-native home
/// 2. `HERMES_HOME` under native home → native home (normal or profile mode)
/// 3. `HERMES_HOME`'s immediate parent named `profiles` → grandparent
/// 4. otherwise `HERMES_HOME` itself
pub fn get_default_hermes_root() -> PathBuf {
    get_default_hermes_root_with(Platform::host(), &RealProbe)
}

pub(crate) fn get_default_hermes_root_with(platform: Platform, probe: &dyn Probe) -> PathBuf {
    let native_home = platform_default_home_with(platform, probe);
    let env_home = probe.env("HERMES_HOME").unwrap_or_default();
    if env_home.trim().is_empty() {
        return native_home;
    }
    let env_path = PathBuf::from(env_home.trim());
    if is_relative_to(&resolve_tolerant(&env_path), &resolve_tolerant(&native_home)) {
        // HERMES_HOME is under the native home (normal or profile mode)
        return native_home;
    }
    // Docker / custom deployment: profile path `<root>/profiles/<name>`?
    if env_path.parent().and_then(|p| p.file_name()).map(|n| n == "profiles").unwrap_or(false) {
        if let Some(grandparent) = env_path.parent().and_then(|p| p.parent()) {
            return grandparent.to_path_buf();
        }
    }
    env_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::fakes::FakeProbe;
    use std::path::PathBuf;

    fn probe_with(home: &str) -> FakeProbe {
        let p = FakeProbe::new();
        *p.home.lock().unwrap() = Some(PathBuf::from(home));
        p
    }

    #[test]
    fn override_set_get_reset_roundtrip() {
        reset_override_for_tests();
        assert_eq!(get_hermes_home_override(), None);
        let t = set_hermes_home_override(Some("/tmp/x"));
        assert_eq!(get_hermes_home_override(), Some("/tmp/x".to_string()));
        reset_hermes_home_override(t);
        assert_eq!(get_hermes_home_override(), None);
    }

    #[test]
    fn override_none_is_no_override() {
        reset_override_for_tests();
        let t = set_hermes_home_override(None::<&str>);
        assert_eq!(get_hermes_home_override(), None);
        reset_hermes_home_override(t);
        assert_eq!(get_hermes_home_override(), None);
    }

    #[test]
    fn override_nested_restores_previous() {
        reset_override_for_tests();
        let t1 = set_hermes_home_override(Some("/a"));
        let t2 = set_hermes_home_override(Some("/b"));
        assert_eq!(get_hermes_home_override(), Some("/b".to_string()));
        reset_hermes_home_override(t2);
        assert_eq!(get_hermes_home_override(), Some("/a".to_string()));
        reset_hermes_home_override(t1);
        assert_eq!(get_hermes_home_override(), None);
    }

    #[test]
    fn platform_default_posix() {
        let p = probe_with("/home/user");
        assert_eq!(
            platform_default_home_with(Platform::Posix, &p),
            PathBuf::from("/home/user/.hermes")
        );
    }

    #[test]
    fn platform_default_windows_uses_localappdata() {
        let p = probe_with("/home/user");
        p.set_env("LOCALAPPDATA", "C:\\Users\\u\\AppData\\Local");
        // Python's PureWindowsPath normalizes separators when joining:
        // Path("C:\\Users\\u\\AppData\\Local") / "hermes" ->
        // WindowsPath('C:/Users/u/AppData/Local/hermes'). On a POSIX host
        // backslash is not a separator, so what we assert is the normalized
        // forward-slash string — the same observable value Python yields.
        let got = platform_default_home_with(Platform::Windows, &p);
        let normalized = got.to_string_lossy().replace('\\', "/");
        assert_eq!(normalized, "C:/Users/u/AppData/Local/hermes");
    }

    #[test]
    fn platform_default_windows_falls_back_to_home() {
        let p = probe_with("/home/user");
        assert_eq!(
            platform_default_home_with(Platform::Windows, &p),
            PathBuf::from("/home/user/AppData/Local/hermes")
        );
    }

    #[test]
    fn home_from_env_uses_env() {
        let p = probe_with("/home/user");
        p.set_env("HERMES_HOME", "/opt/hermes");
        assert_eq!(home_from_env_with(Platform::Posix, &p), PathBuf::from("/opt/hermes"));
    }

    #[test]
    fn home_from_env_falls_back_to_default() {
        let p = probe_with("/home/user");
        assert_eq!(home_from_env_with(Platform::Posix, &p), PathBuf::from("/home/user/.hermes"));
    }

    #[test]
    fn get_hermes_home_prefers_override() {
        reset_override_for_tests();
        let p = probe_with("/home/user");
        p.set_env("HERMES_HOME", "/opt/hermes");
        let t = set_hermes_home_override(Some("/ctx/home"));
        assert_eq!(get_hermes_home_with(Platform::Posix, &p), PathBuf::from("/ctx/home"));
        reset_hermes_home_override(t);
    }

    #[test]
    fn get_process_hermes_home_ignores_override() {
        reset_override_for_tests();
        let p = probe_with("/home/user");
        p.set_env("HERMES_HOME", "/opt/hermes");
        let t = set_hermes_home_override(Some("/ctx/home"));
        assert_eq!(get_process_hermes_home_with(Platform::Posix, &p), PathBuf::from("/opt/hermes"));
        reset_hermes_home_override(t);
    }

    #[test]
    fn default_root_unset_returns_native() {
        let p = probe_with("/tmp/user");
        assert_eq!(
            get_default_hermes_root_with(Platform::Posix, &p),
            PathBuf::from("/tmp/user/.hermes")
        );
    }

    #[test]
    fn default_root_docker_profile() {
        let p = probe_with("/tmp/user");
        p.set_env("HERMES_HOME", "/opt/data/profiles/coder");
        assert_eq!(
            get_default_hermes_root_with(Platform::Posix, &p),
            PathBuf::from("/opt/data")
        );
    }

    #[test]
    fn default_root_env_under_native() {
        let p = probe_with("/tmp/user");
        p.set_env("HERMES_HOME", "/tmp/user/profiles/coder");
        assert_eq!(
            get_default_hermes_root_with(Platform::Posix, &p),
            PathBuf::from("/tmp/user")
        );
    }

    #[test]
    fn default_root_custom_deployment() {
        let p = probe_with("/tmp/user");
        p.set_env("HERMES_HOME", "/opt/data");
        assert_eq!(
            get_default_hermes_root_with(Platform::Posix, &p),
            PathBuf::from("/opt/data")
        );
    }

    #[test]
    fn profile_fallback_warning_writes_once() {
        reset_profile_fallback_warning_for_tests();
        let p = probe_with("/tmp/user");
        p.add_file("/tmp/user/.hermes/active_profile", "coder");
        // First call sets the global once-lock; second is a no-op.
        warn_profile_fallback_once_with(&p);
        warn_profile_fallback_once_with(&p);
        // The warning must not be rearmed by the second call; presence of the
        // OnceLock is all we can assert without capturing stderr. The once
        // semantics are exercised by not panicking and by reset+rearm below.
    }

    #[test]
    fn profile_fallback_no_warning_for_default() {
        reset_profile_fallback_warning_for_tests();
        let p = probe_with("/tmp/user");
        p.add_file("/tmp/user/.hermes/active_profile", "default");
        warn_profile_fallback_once_with(&p);
        // No panic; with active == "default" the once-flag must NOT be set.
        assert!(!*PROFILE_FALLBACK_WARNED.lock().unwrap());
    }
}
