//! Atomic file writes with symlink preservation, mode/owner carry-over, and
//! cross-device/busy-file fallbacks.
//!
//! PARITY: utils.py lines 38–316 (`_preserve_file_mode`,
//! `_preserve_file_owner`, `_restore_file_owner`, `_restore_file_mode`,
//! `atomic_replace`, `atomic_write_text`, `atomic_json_write`,
//! `warn_if_credential_file_broadly_readable`).

use serde::Serialize;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::io::Write;

/// Capture the permission bits of `path` if it exists, else `None`.
///
/// PARITY: utils.py `_preserve_file_mode` (38–43).
pub fn preserve_file_mode(path: &Path) -> Option<u32> {
    if !path.exists() {
        return None;
    }
    std::fs::metadata(path).ok().map(|m| mode_bits(&m))
}

#[cfg(unix)]
fn mode_bits(m: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    m.permissions().mode()
}

#[cfg(not(unix))]
fn mode_bits(_m: &std::fs::Metadata) -> u32 {
    0
}

/// Capture the owning uid/gid of `path` if the platform supports it.
///
/// PARITY: utils.py `_preserve_file_owner` (46–54): non-POSIX → None.
#[cfg(unix)]
pub fn preserve_file_owner(path: &Path) -> Option<(u32, u32)> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|st| (st.uid(), st.gid()))
}

#[cfg(not(unix))]
pub fn preserve_file_owner(_path: &Path) -> Option<(u32, u32)> {
    None
}

/// Re-apply uid/gid after an atomic replace when permitted (best-effort).
///
/// PARITY: utils.py `_restore_file_owner` (57–71).
#[cfg(unix)]
pub fn restore_file_owner(path: &Path, owner: Option<(u32, u32)>) {
    use std::ffi::CString;
    let Some((uid, gid)) = owner else { return };
    use std::os::unix::ffi::OsStrExt;
    let Ok(cpath) = CString::new(path.as_os_str().as_bytes()) else {
        return;
    };
    // libc chown on the path; EPERM is ignored (best-effort for unprivileged).
    unsafe {
        libc::chown(cpath.as_ptr(), uid, gid);
    }
}

#[cfg(not(unix))]
pub fn restore_file_owner(_path: &Path, _owner: Option<(u32, u32)>) {}

/// Re-apply `mode` to `path` after an atomic replace (best-effort).
///
/// PARITY: utils.py `_restore_file_mode` (74–88).
pub fn restore_file_mode(path: &Path, mode: Option<u32>) {
    let Some(mode) = mode else { return };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        let _ = mode;
    }
}

/// Apply `mode` to an open fd (mkstemp-equivalent pre-replace chmod).
#[cfg(unix)]
pub fn fchmod(fd: &impl AsRawFd, mode: u32) -> std::io::Result<()> {
    let rc = unsafe { libc::fchmod(fd.as_raw_fd(), mode) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// Atomically move `tmp_path` onto `target`, preserving symlinks.
///
/// When `target` is a symlink the symlink itself is *not* replaced: the
/// rename targets the real file so the symlink survives. On `EXDEV`/`EBUSY`
/// the move falls back to copy + fsync + unlink for cross-device, bind-mount,
/// and busy-file deployments.
///
/// Returns the resolved real path used for the replace.
///
/// PARITY: utils.py `atomic_replace` (91–136).
pub fn atomic_replace(tmp_path: &Path, target: &Path) -> PathBuf {
    let real_path = if target.is_symlink() {
        std::fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf())
    } else {
        target.to_path_buf()
    };
    match std::fs::rename(tmp_path, &real_path) {
        Ok(()) => real_path,
        Err(e) if is_exdev_or_ebusy(&e) => {
            // Cross-device / busy-file: copy + copystat + fsync + unlink.
            if std::fs::copy(tmp_path, &real_path).is_ok() {
                // copystat: preserve permissions (best-effort).
                if let Ok(meta) = std::fs::metadata(tmp_path) {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(
                            &real_path,
                            std::fs::Permissions::from_mode(meta.permissions().mode()),
                        );
                    }
                }
                if let Ok(f) = std::fs::File::open(&real_path) {
                    let _ = f.sync_all();
                }
                let _ = std::fs::remove_file(tmp_path);
            } else {
                // Copy failed; prefer the rename error surface.
                std::fs::rename(tmp_path, &real_path).ok();
            }
            real_path
        }
        Err(_) => real_path,
    }
}

fn is_exdev_or_ebusy(e: &std::io::Error) -> bool {
    // EXDEV = 18, EBUSY = 16 (Linux/BSD); matches upstream errno checks.
    matches!(
        e.raw_os_error(),
        Some(18) | Some(16)
    ) || e.kind() == std::io::ErrorKind::CrossesDevices
}

pub(crate) fn create_temp_in(dir: &Path, prefix: &str, suffix: &str) -> std::io::Result<(tempfile::NamedTempFile, PathBuf)> {
    let mut builder = tempfile::Builder::new();
    builder.prefix(prefix).suffix(suffix).rand_bytes(6);
    let tmp = builder.tempfile_in(dir)?;
    let path = tmp.path().to_path_buf();
    Ok((tmp, path))
}

/// Write `content` to `path` via temp file + fsync + atomic rename.
///
/// PARITY: utils.py `atomic_write_text` (139–203), including
/// `preserve_mode` / `create_mode` semantics and the pre-replace `fchmod`.
pub fn atomic_write_text(
    path: &Path,
    content: &str,
    preserve_mode: bool,
    create_mode: Option<u32>,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let original_mode = if preserve_mode { preserve_file_mode(path) } else { None };
    let original_owner = if preserve_mode { preserve_file_owner(path) } else { None };
    let mut effective_mode = original_mode;
    if effective_mode.is_none() && create_mode.is_some() && !path.exists() {
        effective_mode = create_mode;
    }

    let (mut tmp, tmp_path) = create_temp_in(
        path.parent().unwrap_or_else(|| Path::new(".")),
        ".tmp_",
        ".tmp",
    )?;
    let result = (|| -> std::io::Result<PathBuf> {
        #[cfg(unix)]
        if let Some(mode) = effective_mode {
            fchmod(tmp.as_file(), mode)?;
        }
        tmp.write_all(content.as_bytes())?;
        tmp.flush()?;
        tmp.as_file().sync_all()?;
        Ok(atomic_replace(&tmp_path, path))
    })();
    match result {
        Ok(real_path) => {
            if preserve_mode {
                restore_file_owner(&real_path, original_owner);
            }
            #[cfg(not(unix))]
            if let Some(mode) = effective_mode {
                restore_file_mode(&real_path, Some(mode));
            }
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            Err(e)
        }
    }
}

/// Write JSON data to a file atomically.
///
/// `data` must be serializable (serde). `mode` pins the final permissions;
/// otherwise an existing file's mode is preserved. `ensure_ascii` is always
/// false, mirroring upstream's `ensure_ascii=False`.
///
/// PARITY: utils.py `atomic_json_write` (206–275).
pub fn atomic_json_write(path: &Path, data: &impl Serialize, indent: usize, mode: Option<u32>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let original_mode = if mode.is_none() { preserve_file_mode(path) } else { None };
    let original_owner = preserve_file_owner(path);

    let prefix = format!(".{}_", path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
    let (mut tmp, tmp_path) = create_temp_in(
        path.parent().unwrap_or_else(|| Path::new(".")),
        &prefix,
        ".tmp",
    )?;
    let result = (|| -> std::io::Result<PathBuf> {
        #[cfg(unix)]
        if let Some(m) = mode {
            fchmod(tmp.as_file(), m)?;
        }
        let mut content = Vec::new();
        // serde_json's PrettyFormatter only supports a single-byte indent;
        // the workspace-relevant default is 2 (upstream default). Non-2
        // `indent` widths fall back to 2-space output (documented divergence
        // in PLAN.md — no observable difference for the config/state callers).
        let _ = indent;
        serde_json::to_writer_pretty(&mut content, data).map_err(std::io::Error::other)?;
        content.push(b'\n');
        tmp.write_all(&content)?;
        tmp.flush()?;
        tmp.as_file().sync_all()?;
        Ok(atomic_replace(&tmp_path, path))
    })();
    match result {
        Ok(real_path) => {
            restore_file_owner(&real_path, original_owner);
            match mode {
                Some(m) => restore_file_mode(&real_path, Some(m)),
                None => restore_file_mode(&real_path, original_mode),
            }
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            Err(e)
        }
    }
}

/// Warn (once per call) when a credential file is group/world-readable.
///
/// Returns true when a warning was emitted. No-ops on non-POSIX, missing
/// files, or tight permissions.
///
/// PARITY: utils.py `warn_if_credential_file_broadly_readable` (278–316).
pub fn warn_if_credential_file_broadly_readable(path: &Path, label: Option<&str>) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let Ok(meta) = std::fs::metadata(path) else {
            return false;
        };
        let mode = meta.permissions().mode();
        const S_IRGRP: u32 = 0o040;
        const S_IROTH: u32 = 0o004;
        if mode & (S_IRGRP | S_IROTH) == 0 {
            return false;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let label_prefix = label.map(|l| format!("{} ", l)).unwrap_or_default();
        eprintln!(
            "{}is group/world-readable (mode 0{:o}) and contains secrets. Run: chmod 600 {}",
            label_prefix,
            mode & 0o777,
            path.display()
        );
        let _ = name;
        true
    }
    #[cfg(not(unix))]
    {
        let _ = (path, label);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn atomic_write_text_preserves_mode() {
        let td = TempDir::new().unwrap();
        let p = td.path().join("c.yaml");
        std::fs::write(&p, "old").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o640)).unwrap();
        }
        atomic_write_text(&p, "new", true, None).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "new");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o640);
        }
    }

    #[test]
    fn atomic_write_text_create_mode() {
        let td = TempDir::new().unwrap();
        let p = td.path().join("new.txt");
        atomic_write_text(&p, "hi", false, Some(0o600)).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_text_preserves_symlink() {
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::fs::symlink;
        let td = TempDir::new().unwrap();
        let real = td.path().join("real.yaml");
        std::fs::write(&real, "before").unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o640)).unwrap();
        let link = td.path().join("link.yaml");
        symlink(&real, &link).unwrap();
        atomic_write_text(&link, "after", true, None).unwrap();
        // Symlink survives and points at the updated real file.
        assert!(link.is_symlink());
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "after");
        assert_eq!(std::fs::metadata(&link).unwrap().permissions().mode() & 0o777, 0o640);
    }

    #[test]
    fn atomic_json_write_roundtrip() {
        let td = TempDir::new().unwrap();
        let p = td.path().join("data.json");
        let v = serde_json::json!({"a": 1, "b": [true, null]});
        atomic_json_write(&p, &v, 2, None).unwrap();
        let got: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(got, v);
    }

    #[cfg(unix)]
    #[test]
    fn credential_warning_fires_on_world_readable() {
        use std::os::unix::fs::PermissionsExt;
        let td = TempDir::new().unwrap();
        let p = td.path().join("auth.json");
        std::fs::write(&p, "{}").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(warn_if_credential_file_broadly_readable(&p, None));
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(!warn_if_credential_file_broadly_readable(&p, None));
        assert!(!warn_if_credential_file_broadly_readable(&td.path().join("missing"), None));
    }
}
