//! Hermes-managed Camofox state helpers.
//!
//! PARITY: `tools/browser_camofox_state.py` @ b9aa928 (whole module, lines
//! 1-48). Provides profile-scoped identity and state directory paths for
//! Camofox persistent browser profiles. When managed persistence is enabled,
//! Hermes sends a deterministic userId derived from the active profile so
//! Camofox can map it to the same persistent browser profile directory
//! across restarts.
//!
//! Upstream resolves the home through `hermes_constants.get_hermes_home()`;
//! this crate calls the ported [`hermes_constants::get_hermes_home`] for the
//! process form and layers an explicit-home form
//! ([`get_camofox_state_dir_at`], [`get_camofox_identity_at`]) as the pure
//! core, matching the crate's layered-form convention.

use hermes_constants::get_hermes_home;
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};

/// PARITY: `CAMOFOX_STATE_DIR_NAME` (upstream line 18).
pub const CAMOFOX_STATE_DIR_NAME: &str = "browser_auth";

/// PARITY: `CAMOFOX_STATE_SUBDIR` (upstream line 19).
pub const CAMOFOX_STATE_SUBDIR: &str = "camofox";

/// `uuid.NAMESPACE_URL` — `6ba7b811-9dad-11d1-80b4-00c04fd430c8`.
const NAMESPACE_URL: [u8; 16] = [
    0x6b, 0xa7, 0xb8, 0x11, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
];

/// The stable Hermes-managed Camofox identity for this profile.
///
/// The user identity is profile-scoped (same Hermes profile = same userId).
/// The session key is scoped to the logical browser task so newly created
/// tabs within the same profile reuse the same identity contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CamofoxIdentity {
    /// `hermes_<10 hex chars>`.
    pub user_id: String,
    /// `task_<16 hex chars>`.
    pub session_key: String,
}

/// Return the profile-scoped root directory for Camofox persistence.
///
/// PARITY: `get_camofox_state_dir` (upstream lines 22-24).
pub fn get_camofox_state_dir() -> PathBuf {
    get_camofox_state_dir_at(&get_hermes_home())
}

/// Explicit-home form of [`get_camofox_state_dir`].
pub fn get_camofox_state_dir_at(home: &Path) -> PathBuf {
    home.join(CAMOFOX_STATE_DIR_NAME).join(CAMOFOX_STATE_SUBDIR)
}

/// Return the stable Hermes-managed Camofox identity for this profile.
///
/// PARITY: `get_camofox_identity` (upstream lines 27-47). A missing, empty,
/// or `"default"` task id shares the default logical scope
/// (`task_id or "default"`).
pub fn get_camofox_identity(task_id: Option<&str>) -> CamofoxIdentity {
    get_camofox_identity_at(&get_camofox_state_dir(), task_id)
}

/// Explicit-scope form of [`get_camofox_identity`]: derives the identity
/// from an explicit state directory instead of the resolved Hermes home.
pub fn get_camofox_identity_at(state_dir: &Path, task_id: Option<&str>) -> CamofoxIdentity {
    let scope_root = state_dir.to_string_lossy();
    let logical_scope = task_id.filter(|task| !task.is_empty()).unwrap_or("default");
    let user_digest = uuid5_hex(&format!("camofox-user:{scope_root}"), 10);
    let session_digest = uuid5_hex(&format!("camofox-session:{scope_root}:{logical_scope}"), 16);
    CamofoxIdentity {
        user_id: format!("hermes_{user_digest}"),
        session_key: format!("task_{session_digest}"),
    }
}

/// PARITY: `uuid.uuid5(uuid.NAMESPACE_URL, name).hex[:take]` — SHA-1 over
/// the namespace bytes + the UTF-8 name, first 16 bytes with version 5 and
/// RFC 4122 variant bits set, lowercase hex.
fn uuid5_hex(name: &str, take: usize) -> String {
    let mut hasher = Sha1::new();
    hasher.update(NAMESPACE_URL);
    hasher.update(name.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    hex[..take].to_string()
}
