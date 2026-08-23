//! File passthrough registry for remote terminal backends.
//!
//! PARITY: tools/credential_files.py @ b9aa928 (530 LOC, ported 1:1).
//!
//! Remote backends (Docker, Modal, SSH) create sandboxes with no host files.
//! This module ensures credential files, skill directories, and host-side
//! cache directories are mounted/synced into those sandboxes.
//!
//! Seams (documented divergences until their home crates land):
//! - `terminal.credential_files` config: upstream reads config.yaml via
//!   hermes_cli.config; the Rust config crate is P3, so a setter seam
//!   (`set_terminal_credential_files`) feeds the same cache. Default: empty,
//!   matching upstream when the config section is absent.
//! - `agent.skill_utils.get_external_skills_dirs`: upstream swallows
//!   ImportError → no external dirs; the Rust seam defaults to empty.
//! - `atexit` temp-dir cleanup in `_safe_skills_path`: Rust has no atexit;
//!   sanitized copies live under std::env::temp_dir and are removed best-effort
//!   on the next call (same reuse pattern), otherwise the OS tmp cleaner.
//! - Python's `ContextVar` registry is task-local; Rust currently uses a
//!   thread-local registry until the async session-context layer lands.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use once_cell::sync::Lazy;
use serde_json::Value;

use crate::file_safety::get_read_block_error;
use crate::path_security::validate_within_dir;

// Session-scoped list of credential files to mount (upstream ContextVar).
thread_local! {
    // Python dicts preserve insertion order; keep the same property instead
    // of using HashMap, because remote backends consume mounts in this order.
    static REGISTERED: RefCell<Vec<(String, String)>> = RefCell::new(Vec::new());
}



/// Config-sourced credential paths (loaded once per process like upstream;
/// raw strings are resolved to host/container pairs at load time).
fn config_files() -> &'static Mutex<Option<Vec<String>>> {
    static FILES: Lazy<Mutex<Option<Vec<String>>>> = Lazy::new(|| Mutex::new(None));
    &FILES
}

/// External skills directories seam (upstream `get_external_skills_dirs`).
fn external_skills_dirs() -> &'static Mutex<Vec<PathBuf>> {
    static DIRS: Lazy<Mutex<Vec<PathBuf>>> = Lazy::new(|| Mutex::new(Vec::new()));
    &DIRS
}

/// Feed `terminal.credential_files` from config (P3 config crate calls this).
pub fn set_terminal_credential_files(entries: Option<Vec<String>>) {
    *config_files().lock().unwrap() = Some(entries.unwrap_or_default());
}

pub fn reset_terminal_credential_files_for_tests() {
    *config_files().lock().unwrap() = None;
}

/// Install external skills directories (agent crate seam).
pub fn set_external_skills_dirs(dirs: Vec<PathBuf>) {
    *external_skills_dirs().lock().unwrap() = dirs;
}

#[derive(Clone, Debug)]
pub struct ConfigFile {
    pub host_path: String,
    pub container_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mount {
    pub host_path: String,
    pub container_path: String,
}

fn resolve_hermes_home() -> PathBuf {
    hermes_constants::home::get_hermes_home()
}

/// Register a credential file for mounting into remote sandboxes.
///
/// `relative_path` is relative to `HERMES_HOME`. Returns true when the file
/// exists on the host and was registered.
pub fn register_credential_file(relative_path: &str, container_base: &str) -> bool {
    let hermes_home = resolve_hermes_home();

    // Reject absolute paths — they bypass the HERMES_HOME sandbox entirely.
    if Path::new(relative_path).is_absolute() {
        log::warn!(
            "credential_files: rejected absolute path {:?} (must be relative to HERMES_HOME)",
            relative_path,
        );
        return false;
    }

    let host_path = hermes_home.join(relative_path);

    // Resolve symlinks and normalise `..` before the containment check so
    // traversal cannot escape HERMES_HOME.
    if let Some(err) = validate_within_dir(&host_path, &hermes_home) {
        log::warn!(
            "credential_files: rejected path traversal {:?} ({})",
            relative_path,
            err,
        );
        return false;
    }

    let resolved = host_path.canonicalize().unwrap_or(host_path);
    if !resolved.is_file() {
        log::debug!("credential_files: skipping {} (not found)", resolved.display());
        return false;
    }

    // Master credential stores are never mountable, even inside HERMES_HOME.
    // Fails CLOSED per #67665: if the canonical guard can't be consulted we
    // refuse rather than risk bind-mounting auth.json into a sandbox.
    if let Some(denied) = get_read_block_error(&resolved.to_string_lossy()) {
        log::warn!(
            "credential_files: refused {:?} — it is a credential store the agent is denied from reading ({})",
            relative_path,
            denied,
        );
        return false;
    }

    let container_path = format!("{}/{}", container_base.trim_end_matches('/'), relative_path);
    REGISTERED.with(|slot| {
        let mut registered = slot.borrow_mut();
        let host_path = resolved.to_string_lossy().into_owned();
        if let Some((_, existing_host_path)) = registered
            .iter_mut()
            .find(|(existing_container_path, _)| existing_container_path == &container_path)
        {
            // Like dict assignment, replacing an existing key does not move
            // it to the end of the insertion order.
            *existing_host_path = host_path;
        } else {
            registered.push((container_path.clone(), host_path));
        }
    });
    log::debug!("credential_files: registered {} -> {}", resolved.display(), container_path);
    true
}

/// Register multiple credential files from skill frontmatter entries.
///
/// Each entry is either a string (relative path) or an object with a `path`
/// key. Returns the relative paths that were NOT found on the host.
pub fn register_credential_files(entries: &[Value], container_base: &str) -> Vec<String> {
    let mut missing = Vec::new();
    for entry in entries {
        let rel_path = match entry {
            Value::String(s) => s.trim().to_string(),
            Value::Object(_) => entry
                .get("path")
                .or_else(|| entry.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string(),
            _ => continue,
        };
        if rel_path.is_empty() {
            continue;
        }
        if !register_credential_file(&rel_path, container_base) {
            missing.push(rel_path);
        }
    }
    missing
}

fn load_config_files() -> Vec<ConfigFile> {
    let guard = config_files().lock().unwrap();
    let Some(entries) = guard.as_ref() else {
        return Vec::new();
    };
    let hermes_home = resolve_hermes_home();
    let mut result = Vec::new();
    for rel in entries {
        let rel = rel.trim();
        if rel.is_empty() {
            continue;
        }
        if Path::new(rel).is_absolute() {
            log::warn!("credential_files: rejected absolute config path {:?}", rel);
            continue;
        }
        let host_path = hermes_home.join(rel);
        if let Some(err) = validate_within_dir(&host_path, &hermes_home) {
            log::warn!("credential_files: rejected config path traversal {:?} ({})", rel, err);
            continue;
        }
        let resolved_path = host_path.canonicalize().unwrap_or(host_path);
        if resolved_path.is_file() {
            let container_path = format!("/root/.hermes/{rel}");
            result.push(ConfigFile {
                host_path: resolved_path.to_string_lossy().into_owned(),
                container_path,
            });
        }
    }
    result
}

/// Return all credential files that should be mounted into remote sandboxes.
pub fn get_credential_file_mounts() -> Vec<Mount> {
    let registered_files: Vec<(String, String)> =
        REGISTERED.with(|slot| slot.borrow().clone());
    let mut mounts: Vec<(String, String)> = Vec::new(); // (container, host)

    // Skill-registered files (re-check existence; may have been deleted).
    for (container_path, host_path) in registered_files {
        if Path::new(&host_path).is_file() {
            mounts.push((container_path, host_path));
        }
    }

    // Config-based files.
    for entry in load_config_files() {
        if !mounts.iter().any(|(cp, _)| *cp == entry.container_path)
            && Path::new(&entry.host_path).is_file()
        {
            mounts.push((entry.container_path, entry.host_path));
        }
    }

    mounts
        .into_iter()
        .map(|(container_path, host_path)| Mount { host_path, container_path })
        .collect()
}

/// Return mount info for all skill directories (local + external).
pub fn get_skills_directory_mount(container_base: &str) -> Vec<Mount> {
    let mut mounts = Vec::new();
    let hermes_home = resolve_hermes_home();
    let skills_dir = hermes_home.join("skills");
    if skills_dir.is_dir() {
        let host_path = safe_skills_path(&skills_dir);
        mounts.push(Mount {
            host_path,
            container_path: format!("{}/skills", container_base.trim_end_matches('/')),
        });
    }

    // External skill dirs (seam; upstream get_external_skills_dirs).
    let ext = external_skills_dirs().lock().unwrap().clone();
    for (idx, ext_dir) in ext.iter().enumerate() {
        if ext_dir.is_dir() {
            let host_path = safe_skills_path(ext_dir);
            mounts.push(Mount {
                host_path,
                container_path: format!(
                    "{}/external_skills/{}",
                    container_base.trim_end_matches('/'),
                    idx
                ),
            });
        }
    }
    mounts
}

static SAFE_SKILLS_TEMPDIR: Lazy<Mutex<Option<PathBuf>>> = Lazy::new(|| Mutex::new(None));

/// Return `skills_dir` if symlink-free, else a sanitized temp copy.
fn safe_skills_path(skills_dir: &Path) -> String {
    let has_symlink = walkdir_symlink_exists(skills_dir);
    if !has_symlink {
        return skills_dir.to_string_lossy().into_owned();
    }

    // Reuse the same temp dir across calls to avoid accumulation.
    {
        let mut guard = SAFE_SKILLS_TEMPDIR.lock().unwrap();
        if let Some(existing) = guard.take() {
            if existing.is_dir() {
                let _ = std::fs::remove_dir_all(&existing);
            }
        }
    }

    let safe_dir = std::env::temp_dir().join(format!(
        "hermes-skills-safe-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    copy_tree_skipping_symlinks(skills_dir, &safe_dir);

    *SAFE_SKILLS_TEMPDIR.lock().unwrap() = Some(safe_dir.clone());
    log::info!("credential_files: created symlink-safe skills copy at {}", safe_dir.display());
    safe_dir.to_string_lossy().into_owned()
}

fn walkdir_symlink_exists(dir: &Path) -> bool {
    fn visit(p: &Path) -> bool {
        if p.is_symlink() {
            return true;
        }
        if p.is_dir() {
            if let Ok(rd) = std::fs::read_dir(p) {
                for e in rd.flatten() {
                    if visit(&e.path()) {
                        return true;
                    }
                }
            }
        }
        false
    }
    visit(dir)
}

/// Copy a tree skipping symlinks entirely (regular files + dirs only).
fn copy_tree_skipping_symlinks(src: &Path, dst: &Path) {
    let _ = std::fs::create_dir_all(dst);
    fn visit(src: &Path, dst: &Path) {
        let Ok(rd) = std::fs::read_dir(src) else {
            return;
        };
        for e in rd.flatten() {
            let sp = e.path();
            if sp.is_symlink() {
                continue;
            }
            let dp = dst.join(e.file_name());
            if sp.is_dir() {
                let _ = std::fs::create_dir_all(&dp);
                visit(&sp, &dp);
            } else if sp.is_file() {
                let _ = std::fs::copy(&sp, &dp);
            }
        }
    }
    visit(src, dst);
}

/// Yield individual (host_path, container_path) entries for skills files.
pub fn iter_skills_files(container_base: &str) -> Vec<Mount> {
    let mut result = Vec::new();
    let hermes_home = resolve_hermes_home();
    let skills_dir = hermes_home.join("skills");
    if skills_dir.is_dir() {
        let container_root = format!("{}/skills", container_base.trim_end_matches('/'));
        collect_files(&skills_dir, &container_root, &mut result);
    }

    let ext = external_skills_dirs().lock().unwrap().clone();
    for (idx, ext_dir) in ext.iter().enumerate() {
        if !ext_dir.is_dir() {
            continue;
        }
        let container_root = format!(
            "{}/external_skills/{}",
            container_base.trim_end_matches('/'),
            idx
        );
        collect_files(ext_dir, &container_root, &mut result);
    }
    result
}

fn collect_files(dir: &Path, container_root: &str, out: &mut Vec<Mount>) {
    /// Walk *dir*; *root* is the original scan root so relative container
    /// paths stay anchored there across recursion levels.
    fn visit(dir: &Path, root: &Path, container_root: &str, out: &mut Vec<Mount>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_symlink() {
                continue;
            }
            if p.is_dir() {
                visit(&p, root, container_root, out);
            } else if p.is_file() {
                let rel = p
                    .strip_prefix(root)
                    .unwrap_or(Path::new(""))
                    .to_string_lossy()
                    .into_owned();
                out.push(Mount {
                    host_path: p.to_string_lossy().into_owned(),
                    container_path: format!("{container_root}/{rel}"),
                });
            }
        }
    }
    visit(dir, dir, container_root, out);
}

// Cache directories mirrored into remote backends: (new_subpath, old_name)
// matching hermes_constants get_hermes_dir.
const CACHE_DIRS: &[(&str, &str)] = &[
    ("cache/documents", "document_cache"),
    ("cache/images", "image_cache"),
    ("cache/audio", "audio_cache"),
    ("cache/videos", "video_cache"),
    ("cache/screenshots", "browser_screenshots"),
    ("cache/web", "web_cache"),
    ("cache/delegation", "delegation_cache"),
    // Desktop/clipboard/PDF uploads land in the flat top-level `images/` dir
    // (tui_gateway attach RPCs) — #69575.
    ("images", "images"),
];

/// Return mount entries for each cache directory that exists on disk.
pub fn get_cache_directory_mounts(container_base: &str) -> Vec<Mount> {
    let mut mounts = Vec::new();
    for (new_subpath, old_name) in CACHE_DIRS {
        let host_dir = hermes_constants::paths::get_hermes_dir(new_subpath, old_name, None);
        if host_dir.is_dir() {
            mounts.push(Mount {
                host_path: host_dir.to_string_lossy().into_owned(),
                container_path: format!("{}/{}", container_base.trim_end_matches('/'), new_subpath),
            });
        }
    }
    mounts
}

/// Map a host cache path to its mounted path under `container_base`.
///
/// Returns the POSIX container path when `host_path` lives under one of the
/// auto-mounted cache directories, else `None`.
pub fn map_cache_path_to_container(host_path: &str, container_base: &str) -> Option<String> {
    let path = Path::new(host_path);
    for mount in get_cache_directory_mounts(container_base) {
        let host_dir = Path::new(&mount.host_path);
        if let Ok(rel) = path.strip_prefix(host_dir) {
            return Some(format!(
                "{}/{}",
                mount.container_path.trim_end_matches('/'),
                rel.to_string_lossy()
            ));
        }
    }
    None
}

fn is_docker_backend() -> bool {
    std::env::var("TERMINAL_ENV").unwrap_or_else(|_| "local".to_string()) == "docker"
}

/// Translate a sandbox/container cache path back to its host path.
pub fn from_agent_visible_cache_path(container_path: &str, container_base: &str) -> String {
    if !is_docker_backend() {
        return container_path.to_string();
    }
    let path = Path::new(container_path);
    for mount in get_cache_directory_mounts(container_base) {
        let container_dir = Path::new(&mount.container_path);
        if let Ok(rel) = path.strip_prefix(container_dir) {
            return Path::new(&mount.host_path)
                .join(rel)
                .to_string_lossy()
                .into_owned();
        }
    }
    container_path.to_string()
}

/// Translate a host cache path to its mounted path inside the sandbox.
pub fn to_agent_visible_cache_path(host_path: &str, container_base: &str) -> String {
    if !is_docker_backend() {
        return host_path.to_string();
    }
    match map_cache_path_to_container(host_path, container_base) {
        Some(mapped) => mapped,
        None => host_path.to_string(),
    }
}

/// Return individual (host_path, container_path) entries for cache files.
pub fn iter_cache_files(container_base: &str) -> Vec<Mount> {
    let mut result = Vec::new();
    for (new_subpath, old_name) in CACHE_DIRS {
        let host_dir = hermes_constants::paths::get_hermes_dir(new_subpath, old_name, None);
        if !host_dir.is_dir() {
            continue;
        }
        let container_root = format!("{}/{}", container_base.trim_end_matches('/'), new_subpath);
        collect_files(&host_dir, &container_root, &mut result);
    }
    result
}

/// Reset the skill-scoped registry (e.g. on session reset).
pub fn clear_credential_files() {
    REGISTERED.with(|slot| slot.borrow_mut().clear());
}
