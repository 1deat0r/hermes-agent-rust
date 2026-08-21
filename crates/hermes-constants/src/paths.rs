//! Well-known Hermes paths.
//!
//! PARITY: hermes_constants.py lines 200–247 (optional-skills / optional-mcps /
//! bundled-skills dirs), 730–779 (`_legacy_path_has_content`), 780–798
//! (`get_hermes_dir`), 1293–1308 (`get_config_path`, `get_skills_dir`,
//! `get_env_path`).

use crate::home::get_hermes_home;
use std::path::{Path, PathBuf};

/// Optional-skills directory, honoring package-manager wrappers.
///
/// PARITY: `get_optional_skills_dir` (201–215): `HERMES_OPTIONAL_SKILLS` env →
/// caller `default` → `<home>/optional-skills`.
pub fn get_optional_skills_dir(default: Option<&Path>) -> PathBuf {
    get_optional_dir_with("HERMES_OPTIONAL_SKILLS", default, "optional-skills")
}

/// Optional-mcps directory, honoring package-manager wrappers.
///
/// PARITY: `get_optional_mcps_dir` (217–234) — mirrors optional-skills.
pub fn get_optional_mcps_dir(default: Option<&Path>) -> PathBuf {
    get_optional_dir_with("HERMES_OPTIONAL_MCPS", default, "optional-mcps")
}

/// Bundled skills directory.
///
/// PARITY: `get_bundled_skills_dir` (236–252): `HERMES_BUNDLED_SKILLS` env →
/// caller `default` → `<home>/skills`.
pub fn get_bundled_skills_dir(default: Option<&Path>) -> PathBuf {
    get_optional_dir_with("HERMES_BUNDLED_SKILLS", default, "skills")
}

fn get_optional_dir_with(env_key: &str, default: Option<&Path>, fallback_name: &str) -> PathBuf {
    let v = std::env::var(env_key).unwrap_or_default();
    if !v.trim().is_empty() {
        return PathBuf::from(v.trim());
    }
    if let Some(d) = default {
        return d.to_path_buf();
    }
    get_hermes_home().join(fallback_name)
}

/// Return `true` iff `path` exists and has content worth honouring.
///
/// A populated *directory* (any entry inside) counts. A non-directory file at
/// `path` also counts. An empty directory does **not** count. If the path
/// cannot be inspected, assume occupied. Symlinks are resolved before judging
/// content; a dangling symlink does **not** count.
///
/// PARITY: hermes_constants.py `_legacy_path_has_content` (730–779).
pub(crate) fn legacy_path_has_content(path: &Path) -> bool {
    let st = match std::fs::symlink_metadata(path) {
        Ok(st) => st,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return false,
        Err(_) => return true, // PermissionError on a parent → assume occupied
    };
    let file_type = st.file_type();
    if file_type.is_symlink() {
        // Resolve the link's target. A dangling symlink has no content and
        // must not shadow the new layout; a valid one is judged on its target.
        match std::fs::metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return false, // dangling
            Err(_) => return true, // can't resolve → assume occupied
            Ok(target) => {
                if !target.is_dir() {
                    return true;
                }
                // target is a directory — fall through to emptiness check
            }
        }
    } else if !file_type.is_dir() {
        return true;
    }
    // Directory (or symlink-to-directory): non-empty?
    let mut entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return true, // can't inspect → assume occupied
    };
    match entries.next() {
        Some(Ok(_)) => true,
        Some(Err(_)) => true,
        None => false,
    }
}

/// Resolve a Hermes subdirectory with backward compatibility.
///
/// New installs get the consolidated layout (e.g. `cache/images`). Existing
/// installs that already have the old path (e.g. `image_cache`) keep using
/// it — no migration required. A bare empty `<old_name>/` directory does not
/// count as "the legacy install is in use".
///
/// PARITY: hermes_constants.py `get_hermes_dir` (254–287).
pub fn get_hermes_dir(
    new_subpath: &str,
    old_name: &str,
    home: Option<&Path>,
) -> PathBuf {
    let home = home.map(|p| p.to_path_buf()).unwrap_or_else(get_hermes_home);
    let old_path = home.join(old_name);
    if legacy_path_has_content(&old_path) {
        old_path
    } else {
        home.join(new_subpath)
    }
}

/// Path to `config.yaml` under HERMES_HOME.
///
/// PARITY: hermes_constants.py `get_config_path` (1293–1300).
pub fn get_config_path() -> PathBuf {
    get_hermes_home().join("config.yaml")
}

/// Path to the skills directory under HERMES_HOME.
///
/// PARITY: hermes_constants.py `get_skills_dir` (1302–1306).
pub fn get_skills_dir() -> PathBuf {
    get_hermes_home().join("skills")
}

/// Path to the `.env` file under HERMES_HOME.
///
/// PARITY: hermes_constants.py `get_env_path` (1308–1311).
pub fn get_env_path() -> PathBuf {
    get_hermes_home().join(".env")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn legacy_empty_dir_returns_new() {
        let td = TempDir::new().unwrap();
        fs::create_dir(td.path().join("old")).unwrap();
        assert_eq!(
            get_hermes_dir("new/sub", "old", Some(td.path())),
            td.path().join("new/sub")
        );
    }

    #[test]
    fn legacy_file_counts_as_content() {
        let td = TempDir::new().unwrap();
        fs::write(td.path().join("old"), "x").unwrap();
        assert_eq!(
            get_hermes_dir("new/sub", "old", Some(td.path())),
            td.path().join("old")
        );
    }

    #[test]
    fn legacy_populated_dir_wins() {
        let td = TempDir::new().unwrap();
        fs::create_dir(td.path().join("old")).unwrap();
        fs::write(td.path().join("old/entry"), "x").unwrap();
        assert_eq!(
            get_hermes_dir("new/sub", "old", Some(td.path())),
            td.path().join("old")
        );
    }

    #[test]
    fn dangling_legacy_symlink_returns_new() {
        let td = TempDir::new().unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(td.path().join("does-not-exist"), td.path().join("old"))
                .unwrap();
            assert_eq!(
                get_hermes_dir("new/sub", "old", Some(td.path())),
                td.path().join("new/sub")
            );
        }
        #[cfg(not(unix))]
        {
            let _ = td;
        }
    }

    #[test]
    fn symlink_to_populated_dir_returns_legacy() {
        let td = TempDir::new().unwrap();
        fs::create_dir(td.path().join("target")).unwrap();
        fs::write(td.path().join("target/entry"), "x").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(td.path().join("target"), td.path().join("old")).unwrap();
            assert_eq!(
                get_hermes_dir("new/sub", "old", Some(td.path())),
                td.path().join("old")
            );
        }
        #[cfg(not(unix))]
        {
            let _ = td;
        }
    }

    #[test]
    fn optional_dir_env_wins() {
        unsafe { std::env::set_var("HERMES_OPTIONAL_SKILLS", "/pkg/optskills") };
        assert_eq!(
            get_optional_skills_dir(None),
            PathBuf::from("/pkg/optskills")
        );
        unsafe { std::env::remove_var("HERMES_OPTIONAL_SKILLS") };
    }

    #[test]
    fn config_path_uses_home() {
        let td = TempDir::new().unwrap();
        unsafe { std::env::set_var("HERMES_HOME", td.path()) };
        assert_eq!(get_config_path(), td.path().join("config.yaml"));
        unsafe { std::env::remove_var("HERMES_HOME") };
    }
}
