// Tier: unit/mock — mirrors tests/hermes_cli/test_managed_scope.py,
// test_managed_scope_config.py, and test_managed_scope_env.py for
// `hermes_cli/managed_scope.py`.
//
// The managed resolver reads process state (`HERMES_MANAGED_DIR`) and the
// loaders share a process-wide cache, so every test here takes the same mutex
// and restores the environment it touched.

use hermes_agent::managed_scope::{
    apply_managed_overlay, get_managed_dir, invalidate_managed_cache, is_env_managed,
    is_key_managed, load_managed_config, load_managed_config_from, load_managed_env,
    load_managed_env_from, managed_config_keys,
};
use parking_lot::Mutex;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

static MANAGED_ENV_MUTEX: Mutex<()> = Mutex::new(());

fn write(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write managed file");
}

/// Create `<tmp>/managed` with optional `config.yaml` and `.env` payloads and
/// publish it as `HERMES_MANAGED_DIR`, mirroring the upstream `_write_managed`
/// helper (which also drops the module caches).
struct ManagedScope {
    dir: PathBuf,
    previous: Option<std::ffi::OsString>,
    _td: TempDir,
}

impl ManagedScope {
    fn new(config: Option<&str>, env: Option<&str>) -> Self {
        let td = tempdir().expect("tempdir");
        let dir = td.path().join("managed");
        fs::create_dir(&dir).expect("managed dir");
        if let Some(body) = config {
            write(&dir.join("config.yaml"), body);
        }
        if let Some(body) = env {
            write(&dir.join(".env"), body);
        }
        let previous = std::env::var_os("HERMES_MANAGED_DIR");
        unsafe { std::env::set_var("HERMES_MANAGED_DIR", &dir) };
        // Drop the per-path caches so a fresh tempdir cannot be shadowed by a
        // stale entry, exactly like the upstream fixture.
        invalidate_managed_cache();
        Self {
            dir,
            previous,
            _td: td,
        }
    }
}

impl Drop for ManagedScope {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var("HERMES_MANAGED_DIR", value) },
            None => unsafe { std::env::remove_var("HERMES_MANAGED_DIR") },
        }
        invalidate_managed_cache();
    }
}

#[test]
fn resolver_honours_an_existing_override_directory() {
    let _lock = MANAGED_ENV_MUTEX.lock();
    let scope = ManagedScope::new(None, None);
    assert_eq!(get_managed_dir().as_deref(), Some(scope.dir.as_path()));
}

#[test]
fn resolver_returns_none_for_a_nonexistent_override() {
    let _lock = MANAGED_ENV_MUTEX.lock();
    let td = tempdir().expect("tempdir");
    let previous = std::env::var_os("HERMES_MANAGED_DIR");
    unsafe { std::env::set_var("HERMES_MANAGED_DIR", td.path().join("absent")) };

    let resolved = get_managed_dir();

    match previous {
        Some(value) => unsafe { std::env::set_var("HERMES_MANAGED_DIR", value) },
        None => unsafe { std::env::remove_var("HERMES_MANAGED_DIR") },
    }
    assert_eq!(resolved, None);
    // No scope means every loader fails open to an empty document.
    assert!(load_managed_config().is_empty());
    assert!(load_managed_env().is_empty());
}

#[test]
fn resolver_treats_blank_and_whitespace_overrides_as_unset() {
    let _lock = MANAGED_ENV_MUTEX.lock();
    let previous = std::env::var_os("HERMES_MANAGED_DIR");
    unsafe { std::env::set_var("HERMES_MANAGED_DIR", "   ") };

    let resolved = get_managed_dir();

    match previous {
        Some(value) => unsafe { std::env::set_var("HERMES_MANAGED_DIR", value) },
        None => unsafe { std::env::remove_var("HERMES_MANAGED_DIR") },
    }
    // Falls through to the POSIX default tier, which is absent on this host;
    // the assertion documents that a whitespace-only override is NOT a scope.
    assert!(resolved.is_none_or(|path| path == Path::new("/etc/hermes")));
}

#[test]
fn managed_config_loads_the_single_managed_file() {
    let _lock = MANAGED_ENV_MUTEX.lock();
    let _scope = ManagedScope::new(Some("model:\n  default: managed/model\n"), None);

    let config = load_managed_config();

    assert_eq!(config["model"]["default"], json!("managed/model"));
    assert!(is_key_managed("model.default"));
    assert!(!is_key_managed("model.base_url"));
    assert_eq!(
        managed_config_keys(),
        BTreeSet::from(["model.default".to_string()])
    );
}

#[test]
fn managed_config_flattens_nested_and_empty_sections_as_leaves() {
    let _lock = MANAGED_ENV_MUTEX.lock();
    let _scope = ManagedScope::new(
        Some("model:\n  default: m\nagent: {}\nauxiliary:\n  vision:\n    model: v\n"),
        None,
    );

    assert_eq!(
        managed_config_keys(),
        BTreeSet::from([
            "agent".to_string(),
            "auxiliary.vision.model".to_string(),
            "model.default".to_string(),
        ])
    );
}

#[test]
fn managed_config_fails_open_when_absent_or_malformed() {
    let _lock = MANAGED_ENV_MUTEX.lock();
    let absent = ManagedScope::new(None, None);
    assert!(load_managed_config_from(&absent.dir).is_empty());

    let malformed = ManagedScope::new(Some("model: [unclosed\n"), None);
    assert!(load_managed_config_from(&malformed.dir).is_empty());

    // A non-mapping document is not a config: upstream returns {} for it.
    let scalar = ManagedScope::new(Some("- just\n- a list\n"), None);
    assert!(load_managed_config_from(&scalar.dir).is_empty());

    // An empty document parses to {} rather than failing.
    let empty = ManagedScope::new(Some("\n"), None);
    assert!(load_managed_config_from(&empty.dir).is_empty());
}

#[test]
fn managed_cache_revalidates_on_file_change_and_clears_on_invalidate() {
    let _lock = MANAGED_ENV_MUTEX.lock();
    let scope = ManagedScope::new(Some("model:\n  default: first\n"), None);
    assert_eq!(load_managed_config()["model"]["default"], json!("first"));

    // Same signature, same cached value.
    assert_eq!(load_managed_config()["model"]["default"], json!("first"));

    // A rewrite with a different byte length changes the `(mtime_ns, size)`
    // signature even when the filesystem clock is coarse.
    write(
        &scope.dir.join("config.yaml"),
        "model:\n  default: second\n",
    );
    assert_eq!(load_managed_config()["model"]["default"], json!("second"));

    invalidate_managed_cache();
    assert_eq!(load_managed_config()["model"]["default"], json!("second"));
}

#[test]
fn managed_env_parses_the_documented_subset() {
    let _lock = MANAGED_ENV_MUTEX.lock();
    let _scope = ManagedScope::new(
        None,
        Some(
            "# comment\n\
             \n\
             OPENAI_API_BASE=https://org.example/v1\n\
             QUOTED=\"double quoted\"\n\
             SINGLE='single quoted'\n\
             PADDED  =  spaced  \n\
             URLISH=a=b=c\n\
             noequals\n",
        ),
    );

    let env = load_managed_env();

    assert_eq!(
        env.get("OPENAI_API_BASE").map(String::as_str),
        Some("https://org.example/v1")
    );
    assert_eq!(env.get("QUOTED").map(String::as_str), Some("double quoted"));
    assert_eq!(env.get("SINGLE").map(String::as_str), Some("single quoted"));
    assert_eq!(env.get("PADDED").map(String::as_str), Some("spaced"));
    // `partition("=")`: only the first `=` splits key from value.
    assert_eq!(env.get("URLISH").map(String::as_str), Some("a=b=c"));
    assert!(!env.contains_key("noequals"));
    assert!(!env.contains_key("# comment"));

    assert!(is_env_managed("OPENAI_API_BASE"));
    assert!(!is_env_managed("OTHER"));
}

#[test]
fn managed_env_absent_is_empty() {
    let _lock = MANAGED_ENV_MUTEX.lock();
    let scope = ManagedScope::new(None, None);
    assert!(load_managed_env_from(&scope.dir).is_empty());
}

#[test]
fn apply_managed_overlay_wins_per_leaf_and_keeps_user_siblings() {
    let _lock = MANAGED_ENV_MUTEX.lock();
    let _scope = ManagedScope::new(Some("model:\n  default: managed/model\n"), None);
    let mut user = Map::new();
    let mut model = Map::new();
    model.insert("default".into(), json!("user/model"));
    model.insert("base_url".into(), json!("user/url"));
    user.insert("model".into(), Value::Object(model));

    let merged = apply_managed_overlay(user);

    assert_eq!(merged["model"]["default"], json!("managed/model"));
    assert_eq!(merged["model"]["base_url"], json!("user/url"));
}

#[test]
fn apply_managed_overlay_promotes_a_bare_string_model() {
    let _lock = MANAGED_ENV_MUTEX.lock();
    // Upstream `_normalize_root_model_keys` only promotes a bare string when
    // root provider/base_url keys exist, so `apply_managed_overlay` patches
    // the remaining case itself.
    let _scope = ManagedScope::new(Some("model: managed/model\n"), None);
    let mut user = Map::new();
    let mut model = Map::new();
    model.insert("default".into(), json!("user/model"));
    user.insert("model".into(), Value::Object(model));

    let merged = apply_managed_overlay(user);

    assert_eq!(merged["model"]["default"], json!("managed/model"));
}

#[test]
fn apply_managed_overlay_without_scope_is_identity() {
    let _lock = MANAGED_ENV_MUTEX.lock();
    let td = tempdir().expect("tempdir");
    let previous = std::env::var_os("HERMES_MANAGED_DIR");
    unsafe { std::env::set_var("HERMES_MANAGED_DIR", td.path().join("absent")) };

    let mut user = Map::new();
    user.insert("model".into(), json!({"default": "user/model"}));
    let merged = apply_managed_overlay(user.clone());

    match previous {
        Some(value) => unsafe { std::env::set_var("HERMES_MANAGED_DIR", value) },
        None => unsafe { std::env::remove_var("HERMES_MANAGED_DIR") },
    }
    assert_eq!(merged, user);
}
