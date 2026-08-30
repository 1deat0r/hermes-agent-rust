// Tier: unit — mirrors `tests/tools/test_browser_camofox_state.py` plus
// golden digests computed from the Python `uuid.uuid5` reference for a fixed
// Hermes home, so the SHA-1 namespace framing, version/variant bits, hex
// casing, and `[:10]`/`[:16]` truncations are all pinned byte-exact.

use hermes_constants::set_hermes_home_override;
use hermes_tools::browser_camofox_state::{
    get_camofox_identity, get_camofox_identity_at, get_camofox_state_dir, get_camofox_state_dir_at,
};
use std::path::Path;

const HOME: &str = "/hermes-home-4d8";

// Oracle: test_paths_are_profile_scoped.
#[test]
fn paths_are_profile_scoped() {
    let _guard = set_hermes_home_override(Some(HOME));
    assert_eq!(
        get_camofox_state_dir(),
        Path::new(HOME).join("browser_auth").join("camofox")
    );
    assert_eq!(
        get_camofox_state_dir_at(Path::new("/elsewhere")),
        Path::new("/elsewhere").join("browser_auth").join("camofox")
    );
}

// Oracle: test_identity_is_deterministic.
#[test]
fn identity_is_deterministic() {
    let _guard = set_hermes_home_override(Some(HOME));
    assert_eq!(
        get_camofox_identity(Some("task-1")),
        get_camofox_identity(Some("task-1"))
    );
}

// Oracle: test_default_task_id, plus the `task_id or "default"` falsiness
// (missing and empty task ids share the default scope).
#[test]
fn default_task_id_and_prefixes() {
    let _guard = set_hermes_home_override(Some(HOME));
    for task_id in [None, Some(""), Some("default")] {
        let identity = get_camofox_identity(task_id);
        assert!(identity.user_id.starts_with("hermes_"), "{identity:?}");
        assert!(identity.session_key.starts_with("task_"), "{identity:?}");
    }
    assert_eq!(
        get_camofox_identity(None),
        get_camofox_identity(Some("default"))
    );
    assert_eq!(get_camofox_identity(Some("")), get_camofox_identity(None));
    // A real task id scopes the session key away from the default.
    assert_ne!(
        get_camofox_identity(Some("task-1")),
        get_camofox_identity(None)
    );
}

// Golden digests: `uuid.uuid5(uuid.NAMESPACE_URL, name).hex[:n]` from the
// Python reference for scope root `/hermes-home-4d8/browser_auth/camofox`.
#[test]
fn identity_matches_the_python_uuid5_reference() {
    let _guard = set_hermes_home_override(Some(HOME));
    let scope_root = get_camofox_state_dir().to_string_lossy().into_owned();
    assert_eq!(scope_root, "/hermes-home-4d8/browser_auth/camofox");

    let identity = get_camofox_identity(Some("task-1"));
    assert_eq!(identity.user_id, "hermes_fcbaba6dc3");
    assert_eq!(identity.session_key, "task_89f1423ee06154e2");

    let default = get_camofox_identity(None);
    assert_eq!(default.user_id, "hermes_fcbaba6dc3");
    assert_eq!(default.session_key, "task_e9cee4d94000529b");

    // The user digest truncates to 10 hex chars, the session digest to 16.
    assert_eq!(identity.user_id.len(), "hermes_".len() + 10);
    assert_eq!(identity.session_key.len(), "task_".len() + 16);
}

// The explicit-path form is the pure core the process form delegates to.
#[test]
fn explicit_state_dir_form_matches_the_process_form() {
    let dir = Path::new(HOME).join("browser_auth").join("camofox");
    let _guard = set_hermes_home_override(Some(HOME));
    assert_eq!(
        get_camofox_identity(Some("task-1")),
        get_camofox_identity_at(&dir, Some("task-1"))
    );
}
