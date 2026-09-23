//! Atomic file writes: symlink preservation, mode/owner carry-over,
//! cross-device/busy fallbacks, and the Windows contended-rename state
//! machine.
//!
//! PARITY: utils.py @ 5d59366 — `_preserve_file_mode` (50–56),
//! `_preserve_file_owner` (58–64), `_restore_file_metadata` (67–80),
//! `default_new_file_mode` (83–99), `_restore_file_owner` /
//! `_restore_file_mode` (102–107), the Windows contended-replace table +
//! retry budget (110–131), `_rewrite_in_place` (134–153),
//! `_copy_fallback` (156–163), `atomic_replace` (166–205),
//! `fsync_directory` (208–225), `_atomic_write` (228–261),
//! `_mode_for_write` (264–267), `atomic_write_text` (270–283),
//! `atomic_write_bytes` (286–291), `atomic_json_write` (311–325),
//! `read_json_or_empty` lives in [`crate::json`],
//! `warn_if_credential_file_broadly_readable` (340–357),
//! `file_signature` (38–47).
//!
//! PORT SEAMS (documented divergences):
//! - The contended-retry jitter mirrors `agent.retry_utils.jittered_backoff`
//!   (base · 2^(n-1) capped at max, + uniform jitter up to 50% of the
//!   delay). Upstream lazy-imports it to keep utils free of an agent
//!   dependency; in Rust the formula is inlined (no crate edge).
//! - Test seams replace the upstream monkeypatches: the replace hook
//!   (`os.replace`), `force_windows_contended_for_test` (`_IS_WINDOWS`),
//!   retry-delay setters (`_REPLACE_RETRY_*`), and the owner seams
//!   (`_preserve_file_owner` / `os.chown`). Documented test-support only.
//! - `atomic_write_text`'s `encoding` / `tmp_prefix` knobs have no caller
//!   in this workspace and no oracle pin: the port writes UTF-8 (Rust
//!   `String`) through a `.tmp_` prefix. `fsync_dir` and `mode` ARE ported
//!   (oracle-pinned).
//! - `_dump_json`'s lone-surrogate `UnicodeEncodeError` retry cannot occur:
//!   Rust `String` cannot hold lone surrogates (see PLAN.md).

use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// Retry budget for the atomic rename (upstream `_REPLACE_RETRY_ATTEMPTS`).
/// A rename that wins here keeps the write fully atomic.
pub const REPLACE_RETRY_ATTEMPTS: u32 = 4;

static REPLACE_RETRY_BASE_MICROS: AtomicU64 = AtomicU64::new(20_000); // 0.02 s
static REPLACE_RETRY_MAX_MICROS: AtomicU64 = AtomicU64::new(100_000); // 0.1 s
static FORCE_WINDOWS_CONTENDED: AtomicBool = AtomicBool::new(false);

type ReplaceHook = Box<dyn Fn(&Path, &Path) -> std::io::Result<PathBuf> + Send + Sync>;
static REPLACE_HOOK: Mutex<Option<ReplaceHook>> = Mutex::new(None);

type PreserveOwnerFn = fn(&Path) -> Option<(u32, u32)>;
type ChownFn = Box<dyn Fn(&Path, u32, u32) + Send + Sync>;
static PRESERVE_OWNER_OVERRIDE: Mutex<Option<PreserveOwnerFn>> = Mutex::new(None);
static CHOWN_SPY: Mutex<Option<ChownFn>> = Mutex::new(None);

/// Test seam: replace `std::fs::rename` inside [`atomic_replace`] (the
/// upstream oracle monkeypatches `utils.os.replace`). `None` restores the
/// default rename (same shape as [`reset_replace_hook_for_test`]).
pub fn set_replace_hook_for_test(hook: Option<ReplaceHook>) {
    *REPLACE_HOOK.lock().expect("replace hook") = hook;
}

/// Test seam: clear the replace hook (default = real rename).
pub fn reset_replace_hook_for_test() {
    *REPLACE_HOOK.lock().expect("replace hook") = None;
}

/// Test seam: pretend to be Windows for the contended-error classifier
/// (upstream monkeypatches `_IS_WINDOWS`).
pub fn force_windows_contended_for_test(on: bool) {
    FORCE_WINDOWS_CONTENDED.store(on, Ordering::SeqCst);
}

/// Test seam: collapse the jittered retry delays (upstream monkeypatches
/// `_REPLACE_RETRY_BASE_DELAY_S` / `_MAX`).
pub fn set_replace_retry_delays_for_test(base_s: f64, max_s: f64) {
    REPLACE_RETRY_BASE_MICROS.store((base_s.max(0.0) * 1e6) as u64, Ordering::SeqCst);
    REPLACE_RETRY_MAX_MICROS.store((max_s.max(0.0) * 1e6) as u64, Ordering::SeqCst);
}

/// Test seam: restore the default retry delays.
pub fn reset_replace_retry_delays_for_test() {
    REPLACE_RETRY_BASE_MICROS.store(20_000, Ordering::SeqCst);
    REPLACE_RETRY_MAX_MICROS.store(100_000, Ordering::SeqCst);
}

type PreserveOwnerOverride = Option<fn(&Path) -> Option<(u32, u32)>>;
type ChownSpy = Option<Box<dyn Fn(&Path, u32, u32) + Send + Sync>>;

/// Test seam: force the preserved uid/gid (upstream monkeypatches
/// `_preserve_file_owner`) and/or record chown calls instead of performing
/// them (upstream monkeypatches `os.chown`).
pub fn set_owner_seams_for_test(preserve: PreserveOwnerOverride, chown: ChownSpy) {
    *PRESERVE_OWNER_OVERRIDE.lock().expect("owner seam") = preserve;
    *CHOWN_SPY.lock().expect("owner seam") = chown;
}

/// Test seam: clear the owner seams (real stat/chown resume).
pub fn reset_owner_seams_for_test() {
    *PRESERVE_OWNER_OVERRIDE.lock().expect("owner seam") = None;
    *CHOWN_SPY.lock().expect("owner seam") = None;
}

// ---------------------------------------------------------------------------
// Metadata helpers (upstream lines 50–107)
// ---------------------------------------------------------------------------

/// Capture the permission bits of `path` if it exists, else `None`.
///
/// PARITY: `_preserve_file_mode` (50–56).
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

/// Capture the owning uid/gid of `path` on POSIX, else `None`.
///
/// PARITY: `_preserve_file_owner` (58–64).
#[cfg(unix)]
pub fn preserve_file_owner(path: &Path) -> Option<(u32, u32)> {
    if let Some(forced) = *PRESERVE_OWNER_OVERRIDE.lock().expect("owner seam") {
        return forced(path);
    }
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|st| (st.uid(), st.gid()))
}

#[cfg(not(unix))]
pub fn preserve_file_owner(_path: &Path) -> Option<(u32, u32)> {
    None
}

/// Best-effort re-apply of uid/gid after an atomic replace.
///
/// PARITY: the `owner` half of `_restore_file_metadata` (67–80).
#[cfg(unix)]
pub fn restore_file_owner(path: &Path, owner: Option<(u32, u32)>) {
    let Some((uid, gid)) = owner else { return };
    if let Some(spy) = CHOWN_SPY.lock().expect("owner seam").as_ref() {
        spy(path, uid, gid);
        return;
    }
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let Ok(cpath) = CString::new(path.as_os_str().as_bytes()) else {
        return;
    };
    // EPERM is ignored — best-effort for unprivileged callers (upstream
    // `with suppress(OSError)`).
    unsafe {
        libc::chown(cpath.as_ptr(), uid, gid);
    }
}

#[cfg(not(unix))]
pub fn restore_file_owner(_path: &Path, _owner: Option<(u32, u32)>) {}

/// Best-effort re-apply of permission bits after an atomic replace.
///
/// PARITY: the `mode` half of `_restore_file_metadata` (67–80).
pub fn restore_file_mode(path: &Path, mode: Option<u32>) {
    let Some(mode) = mode else { return };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
    }
}

/// Best-effort re-apply of uid/gid AND permission bits after an atomic
/// replace — the combined upstream helper.
///
/// PARITY: `_restore_file_metadata` (67–80): privileged callers chown back
/// (Docker/NAS volumes); mkstemp's 0600 would otherwise break mounts that
/// rely on broader permissions.
pub fn restore_file_metadata(path: &Path, owner: Option<(u32, u32)>, mode: Option<u32>) {
    restore_file_owner(path, owner);
    restore_file_mode(path, mode);
}

/// The mode `open(path, "w")` gives a file it has to create
/// (`0o666 & ~umask`); `None` when umask cannot be read or on non-POSIX.
///
/// The transient mask is 0o077 (same race window upstream documents): a
/// thread that opens a file in the read window gets a tighter file, never
/// a looser one.
///
/// PARITY: `default_new_file_mode` (83–99).
pub fn default_new_file_mode() -> Option<u32> {
    #[cfg(unix)]
    {
        unsafe {
            let current = libc::umask(0o077);
            libc::umask(current);
            Some(0o666 & !(current as u32))
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Existing permission bits of `path` (when `preserve`), else `create_mode`
/// for a new file.
///
/// PARITY: `_mode_for_write` (264–267).
fn mode_for_write(path: &Path, create_mode: Option<u32>, preserve: bool) -> Option<u32> {
    let mode = if preserve {
        preserve_file_mode(path)
    } else {
        None
    };
    if mode.is_some() || path.exists() {
        mode
    } else {
        create_mode
    }
}

/// Apply `mode` to an open fd (mkstemp-equivalent pre-replace chmod; the
/// upstream `os.fchmod` — Unix-only).
#[cfg(unix)]
pub fn fchmod(fd: &impl std::os::unix::io::AsRawFd, mode: u32) -> std::io::Result<()> {
    let rc = unsafe { libc::fchmod(fd.as_raw_fd(), mode) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

// ---------------------------------------------------------------------------
// Contended-replace machinery (upstream lines 110–163)
// ---------------------------------------------------------------------------

/// Windows rename failures possibly caused by another handle on the target:
/// 5 ERROR_ACCESS_DENIED (measured shape for a held target), 32
/// ERROR_SHARING_VIOLATION, 33 ERROR_LOCK_VIOLATION.
///
/// PARITY: `_WINDOWS_CONTENDED_REPLACE_ERRORS` (line 117) +
/// `_is_contended_windows_replace_error` (129–131).
fn is_contended_windows_replace_error(e: &std::io::Error) -> bool {
    FORCE_WINDOWS_CONTENDED.load(Ordering::SeqCst)
        && matches!(e.raw_os_error(), Some(5) | Some(32) | Some(33))
}

fn is_cross_device_err(e: &std::io::Error) -> bool {
    // EXDEV = 18, EBUSY = 16 (Linux/BSD); matches upstream errno checks.
    matches!(e.raw_os_error(), Some(18) | Some(16))
        || e.kind() == std::io::ErrorKind::CrossesDevices
}

/// `agent.retry_utils.jittered_backoff` inlined (upstream lazy-imports it
/// to keep the utils ↔ agent edge out of the package graph): delay =
/// `min(base · 2^(n-1), max)` + uniform jitter in `[0, 0.5 · delay]`,
/// attempt is 1-based.
fn jittered_backoff(attempt: u32) -> f64 {
    let base = REPLACE_RETRY_BASE_MICROS.load(Ordering::SeqCst) as f64 / 1e6;
    let max = REPLACE_RETRY_MAX_MICROS.load(Ordering::SeqCst) as f64 / 1e6;
    let exponent = attempt.saturating_sub(1).min(63) as i32;
    let delay = if exponent >= 63 || base <= 0.0 {
        max
    } else {
        (base * 2f64.powi(exponent)).min(max)
    };
    // Seed from time + an atomic tick so coarse clocks still decorrelate
    // concurrent writers (upstream: time_ns ^ tick·0x9E3779B9).
    static TICK: AtomicU64 = AtomicU64::new(0);
    let tick = TICK.fetch_add(1, Ordering::SeqCst);
    let seed = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0))
        ^ tick.wrapping_mul(0x9E37_79B9);
    let mut x = seed | 1;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    let unit = (x as f64) / (u64::MAX as f64); // uniform [0,1)
    delay + unit * (0.5 * delay)
}

/// Overwrite `real_path` through the existing file — last resort for a
/// still-held target (upstream lines 134–153). Not atomic (a smaller
/// window than a copy, not none): writes through the target so it keeps
/// its ACL, never truncates before filling (a concurrent reader must not
/// observe a 0-byte credential store), and `set_len` drops the tail on a
/// shrinking rewrite. Unlinks the temp on success.
///
/// PARITY: `_rewrite_in_place`.
pub fn rewrite_in_place(tmp_path: &Path, real_path: &Path) -> std::io::Result<()> {
    let data = std::fs::read(tmp_path)?;
    let mut file = std::fs::OpenOptions::new().write(true).open(real_path)?;
    file.write_all(&data)?;
    file.set_len(data.len() as u64)?;
    let _ = file.sync_all();
    drop(file);
    std::fs::remove_file(tmp_path)?;
    Ok(())
}

/// Copy/fsync/unlink fallback for cross-device and bind-mount renames
/// (upstream `_copy_fallback`, 156–163): errors PROPAGATE (upstream
/// `shutil.copyfile` raises — the old port swallowed them here, silently
/// reporting success while the target kept stale bytes).
fn copy_fallback(tmp_path: &Path, real_path: &Path) -> std::io::Result<()> {
    std::fs::copy(tmp_path, real_path)?;
    if let Ok(f) = std::fs::File::open(real_path) {
        let _ = f.sync_all();
    }
    std::fs::remove_file(tmp_path)?;
    Ok(())
}

/// Resolve the FINAL path component through its symlink chain (upstream
/// `os.path.realpath` when `islink`): a symlinked target is overwritten
/// in place so the link survives (#16743), including the broken-link case
/// where the real path does not exist yet (Python realpath is non-strict).
fn resolve_replace_target(target: &Path) -> PathBuf {
    let mut cur = target.to_path_buf();
    // 40 hops mirrors ELOOP's typical cap; a self-referential loop stops
    // here instead of spinning.
    for _ in 0..40 {
        match std::fs::symlink_metadata(&cur) {
            Ok(md) if md.file_type().is_symlink() => match std::fs::read_link(&cur) {
                Ok(link) => {
                    cur = if link.is_absolute() {
                        link
                    } else {
                        let parent = cur.parent().unwrap_or_else(|| Path::new(""));
                        parent.join(link)
                    };
                }
                Err(_) => break,
            },
            _ => break, // regular file or missing target — done
        }
    }
    cur
}

fn replace_once(tmp_path: &Path, real_path: &Path) -> std::io::Result<PathBuf> {
    let hook = REPLACE_HOOK.lock().expect("replace hook");
    match hook.as_ref() {
        Some(hook) => hook(tmp_path, real_path),
        None => {
            std::fs::rename(tmp_path, real_path)?;
            Ok(real_path.to_path_buf())
        }
    }
}

/// Atomically move `tmp_path` onto `target`, preserving symlinks.
///
/// Resolves a symlink first so the rename writes the real file in place and
/// the symlink survives. On `EXDEV`/`EBUSY` falls back to copy + fsync +
/// unlink immediately (these never clear on retry). A Windows rename
/// contended by another open handle (winerror 5/32/33) retries with
/// jittered backoff for the bounded budget, then rewrites in place.
/// Every other error PROPAGATES (the old port returned the target path as
/// if the write had succeeded — silent data loss).
///
/// Returns the resolved real path used for the replace.
///
/// PARITY: `atomic_replace` (166–205).
pub fn atomic_replace(tmp_path: &Path, target: &Path) -> std::io::Result<PathBuf> {
    let real_path = resolve_replace_target(target);
    match replace_once(tmp_path, &real_path) {
        Ok(path) => Ok(path),
        Err(e) => {
            let mut contended = is_contended_windows_replace_error(&e);
            if !is_cross_device_err(&e) && !contended {
                return Err(e);
            }
            let mut last = e;
            if contended {
                for attempt in 1..=REPLACE_RETRY_ATTEMPTS {
                    let delay = jittered_backoff(attempt);
                    std::thread::sleep(std::time::Duration::from_secs_f64(delay));
                    match replace_once(tmp_path, &real_path) {
                        Ok(path) => return Ok(path),
                        Err(retry) => {
                            if is_cross_device_err(&retry) {
                                contended = false; // not contention after all
                                last = retry;
                                break;
                            }
                            if !is_contended_windows_replace_error(&retry) {
                                return Err(retry);
                            }
                            last = retry;
                        }
                    }
                }
            }
            log::debug!(
                "atomic_replace: {} -> {} failed with {}; falling back to {}",
                tmp_path.display(),
                real_path.display(),
                last,
                if contended {
                    "in-place rewrite"
                } else {
                    "copy"
                }
            );
            if contended {
                // The rewrite re-raises its own error, so an ACL denial is
                // reported as such, not as contention (upstream line 203).
                rewrite_in_place(tmp_path, &real_path)?;
            } else {
                copy_fallback(tmp_path, &real_path)?;
            }
            Ok(real_path)
        }
    }
}

// ---------------------------------------------------------------------------
// fsync + the shared atomic-write core (upstream lines 208–261)
// ---------------------------------------------------------------------------

/// Best-effort fsync of a directory entry so a just-renamed file survives
/// power loss. No-op on non-POSIX and on any OSError (durability of the
/// directory entry is never worth failing a write that already replaced).
///
/// PARITY: `fsync_directory` (208–225).
pub fn fsync_directory(path: &Path) {
    #[cfg(unix)]
    {
        let Ok(file) = std::fs::File::open(path) else {
            return;
        };
        let _ = file.sync_all();
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

pub(crate) fn create_temp_in(
    dir: &Path,
    prefix: &str,
    suffix: &str,
) -> std::io::Result<(tempfile::NamedTempFile, PathBuf)> {
    let mut builder = tempfile::Builder::new();
    builder.prefix(prefix).suffix(suffix).rand_bytes(6);
    let tmp = builder.tempfile_in(dir)?;
    let path = tmp.path().to_path_buf();
    Ok((tmp, path))
}

/// Per-write policy — the parameters upstream threads through
/// `_atomic_write`.
pub(crate) struct AtomicSpec {
    pub prefix: String,
    pub mode: Option<u32>,
    pub preserve_owner: bool,
    pub fsync_dir: bool,
}

/// Temp file + fsync + [`atomic_replace`], then re-apply owner/mode
/// (upstream `_atomic_write`, 228–261).
///
/// The temp is created O_CREAT|O_EXCL at 0600 (tempfile) so a secret is
/// never readable at process umask, not even between create and chmod.
/// `mode` is fchmod'd onto the temp fd BEFORE the replace so the target
/// never transits through 0600. With no mode a NEW target gets what
/// `open(path, "w")` would have given it (process umask) — the callers
/// this replaced wrote at umask; an existing target with no mode keeps
/// mkstemp's bits (upstream "existing default" semantics). The temp file
/// is removed on any failure — `NamedTempFile`'s drop is the
/// `BaseException` cleanup (upstream lines 258–261).
pub(crate) fn atomic_write_with(
    path: &Path,
    spec: &AtomicSpec,
    write: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> std::io::Result<PathBuf> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut mode = spec.mode;
    if mode.is_none() && !path.exists() {
        mode = default_new_file_mode();
    }
    let owner = if spec.preserve_owner {
        preserve_file_owner(path)
    } else {
        None
    };
    let (mut tmp, tmp_path) = create_temp_in(
        path.parent().unwrap_or_else(|| Path::new(".")),
        &spec.prefix,
        ".tmp",
    )?;
    #[cfg(unix)]
    if let Some(m) = mode {
        fchmod(tmp.as_file(), m)?;
    }
    write(tmp.as_file_mut())?;
    tmp.flush()?;
    tmp.as_file().sync_all()?;
    let real = atomic_replace(&tmp_path, path)?;
    // NamedTempFile drop: unlink(tmp_path) — already renamed/copied away →
    // no-op; on an Err above it performs the cleanup (see atomic_write_with).
    drop(tmp);
    restore_file_metadata(&real, owner, mode);
    if spec.fsync_dir {
        fsync_directory(path.parent().unwrap_or_else(|| Path::new(".")));
    }
    Ok(real)
}

// ---------------------------------------------------------------------------
// Public writers (upstream lines 270–325)
// ---------------------------------------------------------------------------

/// Write `content` to `path` via temp file + fsync + atomic rename.
///
/// `mode` forces the final permission bits regardless of what exists;
/// otherwise `_mode_for_write` semantics apply (existing bits when
/// `preserve_mode`, `create_mode` for a new file, else the umask default
/// for new files / mkstemp 0600 for rewrites). Owner is carried only when
/// `preserve_mode` (upstream `preserve_owner=preserve_mode`).
///
/// PARITY: `atomic_write_text` (270–283). The `encoding` / `tmp_prefix`
/// knobs are unported (see PORT SEAMS).
pub fn atomic_write_text(
    path: &Path,
    content: &str,
    preserve_mode: bool,
    create_mode: Option<u32>,
    mode: Option<u32>,
    fsync_dir: bool,
) -> std::io::Result<()> {
    let eff = mode.or_else(|| mode_for_write(path, create_mode, preserve_mode));
    atomic_write_with(
        path,
        &AtomicSpec {
            prefix: ".tmp_".to_string(),
            mode: eff,
            preserve_owner: preserve_mode,
            fsync_dir,
        },
        |f| {
            f.write_all(content.as_bytes())?;
            Ok(())
        },
    )?;
    Ok(())
}

/// Bytes variant of [`atomic_write_text`] (encrypted blobs, key material).
///
/// PARITY: `atomic_write_bytes` (286–291): owner is never carried;
/// `mode` falls back to the existing file's bits.
pub fn atomic_write_bytes(
    path: &Path,
    content: &[u8],
    mode: Option<u32>,
    fsync_dir: bool,
) -> std::io::Result<()> {
    let eff = mode.or_else(|| preserve_file_mode(path));
    atomic_write_with(
        path,
        &AtomicSpec {
            prefix: ".tmp_".to_string(),
            mode: eff,
            preserve_owner: false,
            fsync_dir,
        },
        |f| {
            f.write_all(content)?;
            Ok(())
        },
    )?;
    Ok(())
}

/// Write JSON to `path` atomically (temp file + fsync + replace).
///
/// `indent` is honored exactly (`json.dumps(indent=N)` — N spaces per
/// level; 0 keeps newlines only). `mode` pins the final permissions;
/// without it an existing file keeps its bits and a new file follows the
/// process umask. Owner is always carried (upstream default).
///
/// PARITY: `atomic_json_write` (311–325). The `_dump_json` surrogate
/// retry has no Rust analog (see PORT SEAMS).
pub fn atomic_json_write(
    path: &Path,
    data: &impl Serialize,
    indent: usize,
    mode: Option<u32>,
    fsync_dir: bool,
) -> std::io::Result<()> {
    let eff = mode.or_else(|| preserve_file_mode(path));
    let prefix = format!(
        ".{}_",
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    );
    atomic_write_with(
        path,
        &AtomicSpec {
            prefix,
            mode: eff,
            preserve_owner: true,
            fsync_dir,
        },
        |f| {
            let indent_bytes = vec![b' '; indent];
            {
                let mut ser = serde_json::Serializer::with_formatter(
                    &mut *f,
                    serde_json::ser::PrettyFormatter::with_indent(&indent_bytes),
                );
                data.serialize(&mut ser).map_err(std::io::Error::other)?;
            }
            f.write_all(b"\n")?;
            Ok(())
        },
    )?;
    Ok(())
}

/// Warn when a credential file is group/world-readable; true when a
/// warning was emitted. No-op on non-POSIX, missing files, or tight
/// permissions.
///
/// PARITY: `warn_if_credential_file_broadly_readable` (340–357) — message
/// carries label + basename + mode + `chmod 600 <full path>`. Emitted
/// through the `log` crate at WARN (upstream `logger.warning`); the
/// upstream `log=` sink parameter maps to the log facade's global logger.
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
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let label_prefix = label.map(|l| format!("{l} ")).unwrap_or_default();
        log::warn!(
            "{label_prefix}{name} is group/world-readable (mode 0{:o}) and contains secrets. Run: chmod 600 {}",
            mode & 0o777,
            path.display()
        );
        true
    }
    #[cfg(not(unix))]
    {
        let _ = (path, label);
        false
    }
}

/// Change-detection key for a stat result:
/// `(st_mtime_ns, st_size, st_ino, st_ctime_ns)`.
///
/// mtime + size alone miss a replacement that preserves both (`cp -p`,
/// `rsync -t`, a script pinning the timestamp); the inode changes on an
/// atomic replace and ctime cannot be backdated from user space, so the
/// quad catches those writers.
///
/// PARITY: `file_signature` (38–47).
pub fn file_signature(path: &Path) -> Option<(u128, u64, u64, u128)> {
    let metadata = std::fs::metadata(path).ok()?;
    let mtime_ns = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let ino = metadata.ino();
        let ctime_ns = ctime_ns(path)?;
        Some((mtime_ns, metadata.len(), ino, ctime_ns))
    }
    #[cfg(not(unix))]
    {
        // Windows: st_ino may be 0 and st_ctime_ns is the creation time —
        // both stable across an in-place rewrite (upstream docstring).
        let ctime = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(mtime_ns);
        Some((mtime_ns, metadata.len(), 0, ctime))
    }
}

#[cfg(unix)]
fn ctime_ns(path: &Path) -> Option<u128> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let cpath = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::stat(cpath.as_ptr(), &mut st) } != 0 {
        return None;
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        Some(st.st_ctimespec.tv_sec as u128 * 1_000_000_000 + st.st_ctimespec.tv_nsec as u128)
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    {
        // libc linux gnu/musl expose (st_ctime, st_ctime_nsec) — the
        // (sec, nsec) pair Python's st_ctime_ns reports.
        Some(st.st_ctime as u128 * 1_000_000_000 + st.st_ctime_nsec as u128)
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
        atomic_write_text(&p, "new", true, None, None, false).unwrap();
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
        atomic_write_text(&p, "hi", false, Some(0o600), None, false).unwrap();
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
        use std::os::unix::fs::symlink;
        use std::os::unix::fs::PermissionsExt;
        let td = TempDir::new().unwrap();
        let real = td.path().join("real.yaml");
        std::fs::write(&real, "before").unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o640)).unwrap();
        let link = td.path().join("link.yaml");
        symlink(&real, &link).unwrap();
        atomic_write_text(&link, "after", true, None, None, false).unwrap();
        assert!(link.is_symlink());
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "after");
        assert_eq!(
            std::fs::metadata(&link).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[test]
    fn atomic_json_write_roundtrip() {
        let td = TempDir::new().unwrap();
        let p = td.path().join("data.json");
        let v = serde_json::json!({"a": 1, "b": [true, null]});
        atomic_json_write(&p, &v, 2, None, false).unwrap();
        let got: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
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
        assert!(!warn_if_credential_file_broadly_readable(
            &td.path().join("missing"),
            None
        ));
    }
}
