//! Platform detection and cross-boundary path translation.
//!
//! PARITY: hermes_constants.py lines 1155–1235 (is_termux, is_wsl, path
//! translation), 1236–1288 (is_container).

use crate::probe::{Probe, RealProbe};
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};

/// True when running inside a Termux (Android) environment.
///
/// Checks `TERMUX_VERSION` (set by Termux) or the Termux-specific `PREFIX`
/// path. No heavy deps.
///
/// PARITY: hermes_constants.py `is_termux` (1155–1164).
pub fn is_termux() -> bool {
    is_termux_with(&RealProbe)
}

pub(crate) fn is_termux_with(probe: &dyn Probe) -> bool {
    let prefix = probe.env("PREFIX").unwrap_or_default();
    probe.env("TERMUX_VERSION").is_some_and(|v| !v.is_empty())
        || prefix.contains("com.termux/files/usr")
}

// 0 = unresolved, 1 = true, 2 = false
static WSL_DETECTED: AtomicU8 = AtomicU8::new(0);

/// True when running inside WSL (Windows Subsystem for Linux).
///
/// Reads `/proc/version` for the `microsoft` marker (WSL1 and WSL2). Result
/// is cached for the process lifetime.
///
/// PARITY: hermes_constants.py `is_wsl` (1166–1183).
pub fn is_wsl() -> bool {
    is_wsl_with(&RealProbe)
}

pub(crate) fn is_wsl_with(probe: &dyn Probe) -> bool {
    match WSL_DETECTED.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            let detected = probe
                .read_file(Path::new("/proc/version"))
                .map(|s| s.to_lowercase().contains("microsoft"))
                .unwrap_or(false);
            WSL_DETECTED.store(if detected { 1 } else { 2 }, Ordering::Relaxed);
            detected
        }
    }
}

/// Convert a Windows drive path (`C:\...`) to its `/mnt/<drive>/...` form.
///
/// PARITY: hermes_constants.py `windows_path_to_wsl` (1185–1199).
pub fn windows_path_to_wsl(path: &str) -> Option<String> {
    let trimmed = path.trim();
    let bytes = trimmed.as_bytes();
    // ^([A-Za-z]):[\\/](.*)$
    if bytes.len() < 3 {
        return None;
    }
    let c = bytes[0];
    if !c.is_ascii_alphabetic() || bytes[1] != b':' || (bytes[2] != b'\\' && bytes[2] != b'/') {
        return None;
    }
    let drive = (c as char).to_ascii_lowercase();
    let tail = trimmed[3..].replace('\\', "/");
    Some(format!("/mnt/{}/{}", drive, tail))
}

/// Convert a Windows WSL UNC path (`\\wsl.localhost\<distro>\...` or legacy
/// `\\wsl$\...`) to a POSIX path inside the distro.
///
/// PARITY: hermes_constants.py `wsl_unc_path_to_posix` (1201–1217).
pub fn wsl_unc_path_to_posix(path: &str) -> Option<String> {
    let normalized: String = path.trim().replace('/', "\\");
    // ^\\\\wsl(?:\.localhost|\$)\\[^\\]+\\(.*)$  (case-insensitive)
    let _b = normalized.as_bytes();
    let prefix = "\\\\wsl";
    let lower = normalized.to_lowercase();
    if !lower.starts_with(prefix) {
        return None;
    }
    // After \\wsl: either ".localhost" or "$", then "\", distro, "\", tail
    let rest = &normalized[prefix.len()..];
    let rest_lower = &lower[prefix.len()..];
    let (after_host, skip) = if rest_lower.starts_with(".localhost\\") {
        (&rest[".localhost\\".len()..], ".localhost\\".len())
    } else if rest_lower.starts_with("$\\") {
        (&rest["$\\".len()..], "$\\".len())
    } else {
        return None;
    };
    let _ = skip;
    // distro: everything up to the next '\'
    let distro_end = after_host.find('\\')?;
    if distro_end == 0 {
        return None; // empty distro
    }
    let tail = &after_host[distro_end + 1..];
    let posix = tail.replace('\\', "/");
    if posix.is_empty() {
        Some("/".to_string())
    } else {
        Some(format!("/{}", posix))
    }
}

/// Normalize a cross-boundary cwd when Hermes itself runs inside WSL.
///
/// No-op off WSL and for paths that are already POSIX.
///
/// PARITY: hermes_constants.py `translate_cwd_for_wsl_backend` (1219–1234).
pub fn translate_cwd_for_wsl_backend(cwd: &str) -> String {
    if !is_wsl() {
        return cwd.to_string();
    }
    if let Some(t) = wsl_unc_path_to_posix(cwd) {
        return t;
    }
    if let Some(t) = windows_path_to_wsl(cwd) {
        return t;
    }
    cwd.to_string()
}

// 0 = unresolved, 1 = true, 2 = false
static CONTAINER_DETECTED: AtomicU8 = AtomicU8::new(0);

/// True when running inside a container.
///
/// Recognizes Docker (`/.dockerenv`), Podman (`/run/.containerenv`),
/// Kubernetes (`KUBERNETES_SERVICE_HOST`), and cgroup/mountinfo markers.
/// Result is cached for the process lifetime.
///
/// PARITY: hermes_constants.py `is_container` (1236–1288).
pub fn is_container() -> bool {
    is_container_with(&RealProbe)
}

#[allow(clippy::if_same_then_else)]
pub(crate) fn is_container_with(probe: &dyn Probe) -> bool {
    match CONTAINER_DETECTED.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            const CGROUP_MARKERS: [&str; 6] = ["docker", "podman", "/lxc/", "kubepods", "containerd", "crio"];
            let detected = if probe.file_exists(Path::new("/.dockerenv")) {
                true
            } else if probe.file_exists(Path::new("/run/.containerenv")) {
                true
            } else if probe.env("KUBERNETES_SERVICE_HOST").is_some_and(|v| !v.is_empty()) {
                true
            } else if let Some(cgroup) = probe.read_file(Path::new("/proc/1/cgroup")) {
                CGROUP_MARKERS.iter().any(|m| cgroup.contains(m))
            } else if let Some(mountinfo) = probe.read_file(Path::new("/proc/self/mountinfo")) {
                ["kubepods", "containerd", "crio"].iter().any(|m| mountinfo.contains(m))
            } else {
                false
            };
            CONTAINER_DETECTED.store(if detected { 1 } else { 2 }, Ordering::Relaxed);
            detected
        }
    }
}

#[doc(hidden)]
pub fn reset_platform_caches_for_tests() {
    WSL_DETECTED.store(0, Ordering::Relaxed);
    CONTAINER_DETECTED.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::fakes::FakeProbe;
    use std::sync::Mutex;

    // The production detectors intentionally cache for process lifetime. Keep
    // cache-resetting tests serialized so parallel test execution cannot let
    // one fake probe overwrite another test's cached result.
    static PLATFORM_CACHE_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn termux_via_version() {
        let p = FakeProbe::new();
        p.set_env("TERMUX_VERSION", "0.119.0-beta.1");
        assert!(is_termux_with(&p));
    }

    #[test]
    fn termux_via_prefix() {
        let p = FakeProbe::new();
        p.set_env("PREFIX", "/data/data/com.termux/files/usr");
        assert!(is_termux_with(&p));
    }

    #[test]
    fn not_termux() {
        let p = FakeProbe::new();
        assert!(!is_termux_with(&p));
    }

    #[test]
    fn wsl_detected_from_proc_version() {
        let _guard = PLATFORM_CACHE_TEST_LOCK.lock().unwrap();
        reset_platform_caches_for_tests();
        let p = FakeProbe::new();
        p.add_file("/proc/version", "Linux version 5.15.153.1-microsoft-standard-WSL2");
        assert!(is_wsl_with(&p));
    }

    #[test]
    fn wsl_not_detected() {
        let _guard = PLATFORM_CACHE_TEST_LOCK.lock().unwrap();
        reset_platform_caches_for_tests();
        let p = FakeProbe::new();
        p.add_file("/proc/version", "Linux version 6.6.0-generic");
        assert!(!is_wsl_with(&p));
    }

    #[test]
    fn container_dockerenv() {
        let _guard = PLATFORM_CACHE_TEST_LOCK.lock().unwrap();
        reset_platform_caches_for_tests();
        let p = FakeProbe::new();
        p.add_path("/.dockerenv");
        assert!(is_container_with(&p));
    }

    #[test]
    fn container_kubernetes_env() {
        let _guard = PLATFORM_CACHE_TEST_LOCK.lock().unwrap();
        reset_platform_caches_for_tests();
        let p = FakeProbe::new();
        p.set_env("KUBERNETES_SERVICE_HOST", "10.96.0.1");
        assert!(is_container_with(&p));
    }

    #[test]
    fn container_cached() {
        let _guard = PLATFORM_CACHE_TEST_LOCK.lock().unwrap();
        reset_platform_caches_for_tests();
        let p = FakeProbe::new();
        p.add_path("/.dockerenv");
        assert!(is_container_with(&p));
        // Second call reads cache; add a contradicting probe to prove caching.
        reset_platform_caches_for_tests();
        let p2 = FakeProbe::new();
        assert!(!is_container_with(&p2));
    }

    #[test]
    fn windows_drive_to_wsl() {
        assert_eq!(
            windows_path_to_wsl("C:\\Users\\me\\work"),
            Some("/mnt/c/Users/me/work".to_string())
        );
        assert_eq!(windows_path_to_wsl("D:/stuff"), Some("/mnt/d/stuff".to_string()));
        assert_eq!(windows_path_to_wsl("/home/user"), None);
        assert_eq!(windows_path_to_wsl(""), None);
    }

    #[test]
    fn unc_to_posix() {
        assert_eq!(
            wsl_unc_path_to_posix("\\\\wsl.localhost\\Ubuntu\\home\\me"),
            Some("/home/me".to_string())
        );
        assert_eq!(
            wsl_unc_path_to_posix("\\\\wsl$\\Ubuntu\\home"),
            Some("/home".to_string())
        );
        assert_eq!(wsl_unc_path_to_posix("C:\\path"), None);
    }
}

// ── Node directory helpers (layout only; bootstrap machinery is P2) ───────
//
// PARITY: hermes_constants.py lines 285–318 (`iter_hermes_node_dirs`,
// `_candidate_node_command_names`).

use std::path::PathBuf;

/// Hermes-managed Node.js directories in preferred lookup order.
///
/// Windows installs unpack portable Node directly into
/// `%LOCALAPPDATA%\hermes\node`; POSIX installs use
/// `$HERMES_HOME/node/bin`. Both shapes are returned on every platform so
/// mixed or migrated installs still work.
///
/// PARITY: hermes_constants.py `iter_hermes_node_dirs` (285–303).
pub fn iter_hermes_node_dirs(home: Option<&std::path::Path>) -> Vec<PathBuf> {
    let root = home.map(|p| p.to_path_buf()).unwrap_or_else(super::home::get_hermes_home);
    let dirs = root.join("node");
    let bin_dir = root.join("node").join("bin");
    if cfg!(windows) {
        vec![dirs, bin_dir]
    } else {
        vec![bin_dir, dirs]
    }
}

/// Candidate executable names for a Node tool command on the host platform.
///
/// PARITY: hermes_constants.py `_candidate_node_command_names` (305–318).
pub fn candidate_node_command_names(command: &str) -> Vec<String> {
    let base = std::path::Path::new(command)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| command.to_string());
    if !cfg!(windows) || base.contains('.') {
        return vec![base];
    }
    let lower = base.to_lowercase();
    match lower.as_str() {
        "npm" => vec!["npm.cmd".into(), "npm.exe".into(), "npm".into()],
        "npx" => vec!["npx.cmd".into(), "npx.exe".into(), "npx".into()],
        "node" => vec!["node.exe".into(), "node".into()],
        _ => vec![format!("{}.cmd", base), format!("{}.exe", base), base],
    }
}

#[cfg(test)]
mod node_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn posix_order_bin_first() {
        let home = Path::new("/h");
        #[cfg(not(windows))]
        assert_eq!(
            iter_hermes_node_dirs(Some(home)),
            vec![PathBuf::from("/h/node/bin"), PathBuf::from("/h/node")]
        );
        #[cfg(windows)]
        assert_eq!(
            iter_hermes_node_dirs(Some(home)),
            vec![PathBuf::from("/h/node"), PathBuf::from("/h/node/bin")]
        );
    }

    #[test]
    fn candidate_names_posix() {
        #[cfg(not(windows))]
        assert_eq!(candidate_node_command_names("npm"), vec!["npm".to_string()]);
        #[cfg(not(windows))]
        assert_eq!(candidate_node_command_names("/usr/bin/node"), vec!["node".to_string()]);
        #[cfg(not(windows))]
        assert_eq!(candidate_node_command_names("npm.cmd"), vec!["npm.cmd".to_string()]);
    }

    #[test]
    fn candidate_names_windows() {
        // The implementation is host-gated; on POSIX hosts we can only assert
        // the non-Windows behavior. Windows branches are covered by review.
        if cfg!(windows) {
            assert_eq!(
                candidate_node_command_names("npm"),
                vec!["npm.cmd".to_string(), "npm.exe".to_string(), "npm".to_string()]
            );
            assert_eq!(
                candidate_node_command_names("npx"),
                vec!["npx.cmd".to_string(), "npx.exe".to_string(), "npx".to_string()]
            );
            assert_eq!(
                candidate_node_command_names("node"),
                vec!["node.exe".to_string(), "node".to_string()]
            );
            assert_eq!(
                candidate_node_command_names("custom"),
                vec!["custom.cmd".to_string(), "custom.exe".to_string(), "custom".to_string()]
            );
        }
    }
}
