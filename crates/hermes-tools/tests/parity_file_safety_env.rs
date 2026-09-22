//! Isolated (own-process) oracles for env-dependent file-safety guards.
//! The env vars are process-global, so these must not share a test binary
//! with the parallel pure-path tests. Each test owns its env window
//! sequentially (workspace runs `--test-threads=1`).
//!
//! Mirrors upstream fixtures that monkeypatch `_hermes_home_path` /
//! `_hermes_root_path`: the Rust analog is HERMES_HOME env + the
//! `get_default_hermes_root` profiles-parent derivation, which is exactly
//! what upstream's own `fake_homes` fixture documents
//! (test_file_safety_session_state.py lines 24–33).
//! Tier: `unit`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use hermes_tools::file_safety::{
    classify_write_denial, get_read_block_error, get_safe_write_roots, get_write_denied_error,
    is_write_denied, resolve_active_profile_name,
};

/// Serializes the env-mutating tests in this binary: HERMES_HOME is
/// process-global, so parallel threads would cross-contaminate each
/// other's layouts even though the workspace protocol runs
/// `--test-threads=1` (a bare `cargo test -p hermes-tools` must not flake).
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn tmp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "hfs_env_{label}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct EnvGuard {
    key: &'static str,
    old: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &Path) -> Self {
        let old = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

fn touch(base: &Path, rel: &str) -> PathBuf {
    let p = base.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&p, "dummy").unwrap();
    p
}

#[test]
fn hermes_home_guard_cases() {
    let _guard = ENV_LOCK.lock().unwrap();
    // Upstream TestCacheFileReadBlocking + TestCombinedGuards +
    // state.db write denial, under a HERMES_HOME-scoped tmp dir.
    let dir = tmp_dir("hub");
    let _env = EnvGuard::set("HERMES_HOME", &dir);
    let cache = dir.join("skills/.hub/index-cache/data.json");
    std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
    std::fs::write(&cache, "{}").unwrap();
    let hub = dir.join("skills/.hub/metadata.json");
    std::fs::write(&hub, "{}").unwrap();
    let state_db = dir.join("state.db");
    std::fs::write(&state_db, "x").unwrap();

    let error = get_read_block_error(&cache.to_string_lossy());
    assert!(error.is_some());
    assert!(error.unwrap().contains("internal Hermes cache"));

    let error = get_read_block_error(&hub.to_string_lossy());
    assert!(error.unwrap_or_default().contains("internal Hermes cache"));
    // Probe-pinned: the hub-cache reason is the ONE read denial without the
    // defense-in-depth suffix (upstream line 341 vs 347+).
    assert!(
        !get_read_block_error(&hub.to_string_lossy())
            .unwrap_or_default()
            .contains("Defense-in-depth"),
        "hub message carries no DID suffix"
    );

    // Probe-pinned precedence: a `.env` INSIDE vault/ takes the vault
    // directory message (the dir deny sits in the elif chain above the
    // env-basename `or_else`).
    let vault_env = dir.join("vault/.env");
    std::fs::create_dir_all(vault_env.parent().unwrap()).unwrap();
    std::fs::write(&vault_env, "x").unwrap();
    let err = get_read_block_error(&vault_env.to_string_lossy()).expect("blocked");
    assert!(err.contains("credential vault"), "dir reason wins: {err}");

    // Regular project .env still blocked; .env.example allowed.
    assert!(get_read_block_error("/workspace/.env").is_some());
    assert!(get_read_block_error("/workspace/.env.example").is_none());

    // state.db write denial.
    assert_eq!(
        classify_write_denial(&state_db.to_string_lossy()),
        Some("credential")
    );

    // Arbitrary non-credential home file stays readable (oracle:
    // test_arbitrary_hermes_home_file_not_blocked).
    let safe = touch(&dir, "session_log.txt");
    assert!(get_read_block_error(&safe.to_string_lossy()).is_none());
    // Nested same-name file is not the top-level credential store
    // (oracle: test_subdirectory_named_auth_json_not_blocked).
    let nested = touch(&dir, "skills/my-skill/auth.json");
    assert!(get_read_block_error(&nested.to_string_lossy()).is_none());
    // Only the known google path is blocked, not all auth/*
    // (oracle: test_non_secret_auth_subtree_file_not_blocked).
    let note = touch(&dir, "auth/notes.json");
    assert!(get_read_block_error(&note.to_string_lossy()).is_none());
    // config.yaml stays readable (oracle: test_config_yaml_not_blocked).
    let cfg = touch(&dir, "config.yaml");
    assert!(get_read_block_error(&cfg.to_string_lossy()).is_none());
    // webhook_subscriptions.json holds HMAC secrets (oracle:
    // test_webhook_subscriptions_blocked).
    let subs = touch(&dir, "webhook_subscriptions.json");
    let err = get_read_block_error(&subs.to_string_lossy()).expect("blocked");
    assert!(err.contains("credential store"), "{err}");

    drop(_env);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_denies_hermes_credential_stores() {
    let _guard = ENV_LOCK.lock().unwrap();
    // Upstream test_file_safety_credentials: exact stores + the
    // credential-store message, on a single tmp HERMES_HOME.
    let home = tmp_dir("creds");
    let _env = EnvGuard::set("HERMES_HOME", &home);

    for rel in [
        "auth.json",
        "auth.lock",
        ".anthropic_oauth.json",
        ".env",
        "webhook_subscriptions.json",
        "auth/google_oauth.json",
        "cache/bws_cache.json",
    ] {
        let p = touch(&home, rel);
        let err = get_read_block_error(&p.to_string_lossy())
            .unwrap_or_else(|| panic!("not blocked: {rel}"));
        assert!(err.contains("credential store"), "{rel}: {err}");
    }

    drop(_env);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn read_denies_dirs_and_env_but_not_outside_home_lookalikes() {
    let _guard = ENV_LOCK.lock().unwrap();
    // Upstream test_identically_named_hermes_files_outside_home_not_blocked
    // + mcp-tokens dir/file messages + .env basename blocked anywhere.
    let home = tmp_dir("dirs");
    let _env = EnvGuard::set("HERMES_HOME", &home);

    let tok = touch(&home, "mcp-tokens/provider.json");
    let err = get_read_block_error(&tok.to_string_lossy()).expect("blocked");
    assert!(err.contains("MCP token"), "{err}");
    let tok_dir = home.join("mcp-tokens");
    let err = get_read_block_error(&tok_dir.to_string_lossy()).expect("blocked");
    assert!(err.contains("MCP token directory"), "{err}");

    // Lookalikes OUTSIDE HERMES_HOME stay readable (per-location gate).
    let outside = tmp_dir("outside");
    let auth = touch(&outside, "auth.json");
    assert!(get_read_block_error(&auth.to_string_lossy()).is_none());
    let oauth = touch(&outside, "auth/google_oauth.json");
    assert!(get_read_block_error(&oauth.to_string_lossy()).is_none());
    let outside_tok = touch(&outside, "mcp-tokens/token.json");
    assert!(get_read_block_error(&outside_tok.to_string_lossy()).is_none());
    // …while .env is blocked anywhere on disk.
    assert!(get_read_block_error(&outside.join(".env").to_string_lossy()).is_some());
    let _ = std::fs::remove_dir_all(&outside);

    drop(_env);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn profile_layout_write_denies_secret_stores_both_bases() {
    let _guard = ENV_LOCK.lock().unwrap();
    // Upstream test_file_safety_write_credentials with the
    // `hermes_layout` fixture: HERMES_HOME=<root>/profiles/coder makes
    // get_default_hermes_root derive <root> (profiles-parent rule).
    let root = tmp_dir("wroot");
    let profile = root.join("profiles/coder");
    std::fs::create_dir_all(&profile).unwrap();
    let _env = EnvGuard::set("HERMES_HOME", &profile);

    const SECRET_STORES: &[&str] = &[
        "auth/google_oauth.json",
        "cache/bws_cache.json",
        "vault/vault.key",
        "browser-profile/Default/Cookies",
    ];
    for base in [&profile, &root] {
        for rel in SECRET_STORES {
            let path = touch(base, rel);
            let path_s = path.to_string_lossy();
            // Fixture-drift guard: every secret store is read-denied…
            assert!(
                get_read_block_error(&path_s).is_some(),
                "fixture drift: not read-denied: {path_s}"
            );
            // …and write-denied on BOTH the profile and the global root.
            assert!(is_write_denied(&path_s), "write allowed: {path_s}");
        }
    }

    // Control files stay writable (#45947) on both bases.
    for base in [&profile, &root] {
        for rel in ["auth.json", "config.yaml", "webhook_subscriptions.json"] {
            let path = touch(base, rel);
            assert!(
                !is_write_denied(&path.to_string_lossy()),
                "#45947 regression: {rel}"
            );
        }
    }
    // Lookalikes OUTSIDE hermes homes stay writable.
    let project = tmp_dir("wproj");
    assert!(!is_write_denied(
        &touch(&project, "cache/bws_cache.json").to_string_lossy()
    ));
    assert!(!is_write_denied(
        &touch(&project, "vault/vault.key").to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&project);

    drop(_env);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn session_state_paths_are_write_denied() {
    let _guard = ENV_LOCK.lock().unwrap();
    // Upstream test_file_safety_session_state::test_session_state_paths —
    // fake_homes fixture uses exactly this env chain.
    let root = tmp_dir("sroot");
    let profile = root.join("profiles/work");
    std::fs::create_dir_all(&profile).unwrap();
    let _env = EnvGuard::set("HERMES_HOME", &profile);

    for rel in ["state.db", "sessions/session_abc.json"] {
        let target = touch(&profile, rel);
        assert!(
            is_write_denied(&target.to_string_lossy()),
            "session state must be write-denied: {rel}"
        );
    }

    drop(_env);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn profile_mode_blocks_root_credentials_on_read() {
    let _guard = ENV_LOCK.lock().unwrap();
    // Upstream test_profile_mode_blocks_root_credentials — profile +
    // distinct root, both credential-scoped reads blocked.
    let root = tmp_dir("proot");
    let profile = root.join("profiles/coder");
    std::fs::create_dir_all(&profile).unwrap();
    let _env = EnvGuard::set("HERMES_HOME", &profile);

    let profile_auth = touch(&profile, "auth.json");
    assert!(get_read_block_error(&profile_auth.to_string_lossy())
        .unwrap_or_default()
        .contains("credential store"));
    let root_auth = touch(&root, "auth.json");
    assert!(get_read_block_error(&root_auth.to_string_lossy())
        .unwrap_or_default()
        .contains("credential store"));
    let root_env = touch(&root, ".env");
    assert!(
        get_read_block_error(&root_env.to_string_lossy())
            .unwrap_or_default()
            .contains("credential store"),
        ".env under root hits the exact-credential arm first"
    );
    let root_oauth = touch(&root, "auth/google_oauth.json");
    assert!(get_read_block_error(&root_oauth.to_string_lossy())
        .unwrap_or_default()
        .contains("credential store"));
    let root_tok = touch(&root, "mcp-tokens/gh.json");
    assert!(get_read_block_error(&root_tok.to_string_lossy())
        .unwrap_or_default()
        .contains("MCP token"));

    drop(_env);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn resolve_active_profile_name_cases() {
    let _guard = ENV_LOCK.lock().unwrap();
    // Upstream TestResolveActiveProfileName — home==root → "default";
    // home=<root>/profiles/X → "X"; profile-scoped env chain.
    let root = tmp_dir("aprofile");
    let profile = root.join("profiles/coder");
    std::fs::create_dir_all(&profile).unwrap();

    {
        // HERMES_HOME=<root>/profiles/coder → active profile "coder".
        let _env = EnvGuard::set("HERMES_HOME", &profile);
        assert_eq!(resolve_active_profile_name(), "coder");
    }
    {
        // HERMES_HOME=<root> itself (no profiles parent) → "default"
        // (upstream: home is the root; relative_to(root/profiles) fails).
        let _env = EnvGuard::set("HERMES_HOME", &root);
        assert_eq!(resolve_active_profile_name(), "default");
    }
    {
        // Unset HERMES_HOME → platform default home → "default".
        let old = std::env::var("HERMES_HOME").ok();
        std::env::remove_var("HERMES_HOME");
        assert_eq!(resolve_active_profile_name(), "default");
        if let Some(v) = old {
            std::env::set_var("HERMES_HOME", v);
        }
    }
    {
        // HERMES_HOME=<root>/profiles (bare) → root resolves to HERMES_HOME
        // itself (rule 4) → relative_to(root/profiles) fails → "default".
        let bare = root.join("profiles");
        let _env = EnvGuard::set("HERMES_HOME", &bare);
        assert_eq!(resolve_active_profile_name(), "default");
    }

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn safe_root_gate_cases() {
    let _guard = ENV_LOCK.lock().unwrap();
    // HERMES_WRITE_SAFE_ROOT gate, run in one test.
    let old = std::env::var("HERMES_WRITE_SAFE_ROOT").ok();
    let dir = tmp_dir("safe");
    std::env::set_var("HERMES_WRITE_SAFE_ROOT", &dir);
    {
        assert_eq!(
            classify_write_denial("/tmp/other/file.txt"),
            Some("safe_root")
        );
        let err = get_write_denied_error("/tmp/other/file.txt", "Write");
        assert!(err.is_some() && err.unwrap().contains("HERMES_WRITE_SAFE_ROOT"));
        assert!(classify_write_denial(&dir.join("file.txt").to_string_lossy()).is_none());
        assert!(get_safe_write_roots().contains(&dir.to_string_lossy().into_owned()));
    }
    match old {
        Some(v) => std::env::set_var("HERMES_WRITE_SAFE_ROOT", v),
        None => std::env::remove_var("HERMES_WRITE_SAFE_ROOT"),
    }
    assert!(classify_write_denial("/tmp/other/file.txt").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
