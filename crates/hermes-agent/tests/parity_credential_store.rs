use hermes_agent::credential_store::{
    load_auth_store, read_credential_pool_at, save_auth_store, write_credential_pool_at,
    AUTH_STORE_VERSION,
};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn store_path(root: &Path) -> PathBuf {
    root.join("hermes").join("auth.json")
}

fn store_with_pool(pool: Value) -> Value {
    json!({"version": AUTH_STORE_VERSION, "credential_pool": pool})
}

fn write_json(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

// Tier: unit — mirrors hermes_cli.auth._load_auth_store missing-file behavior.
#[test]
fn missing_auth_store_returns_versioned_empty_shape() {
    let temp = tempfile::tempdir().unwrap();
    let loaded = load_auth_store(Some(&store_path(temp.path()))).unwrap();

    assert_eq!(loaded.get("version"), Some(&json!(1)));
    assert_eq!(loaded.get("providers"), Some(&json!({})));
}

// Tier: unit — mirrors _load_auth_store's accepted schema and Nous migration.
#[test]
fn load_accepts_pool_only_stores_and_migrates_stale_nous_portal_url() {
    let temp = tempfile::tempdir().unwrap();
    let path = store_path(temp.path());
    write_json(
        &path,
        &json!({
            "version": 1,
            "credential_pool": {},
            "providers": {
                "nous": {"portal_base_url": "https://api.nousresearch.com"}
            }
        }),
    );

    let loaded = load_auth_store(Some(&path)).unwrap();
    assert_eq!(loaded.get("credential_pool"), Some(&json!({})));
    assert_eq!(
        loaded["providers"]["nous"]["portal_base_url"],
        "https://portal.nousresearch.com"
    );
}

// Tier: unit — mirrors the legacy PR `systems` migration branch.
#[test]
fn load_migrates_legacy_systems_shape_to_nous_provider() {
    let temp = tempfile::tempdir().unwrap();
    let path = store_path(temp.path());
    write_json(
        &path,
        &json!({"systems": {"nous_portal": {"refresh_token": "refresh"}}}),
    );

    let loaded = load_auth_store(Some(&path)).unwrap();
    assert_eq!(loaded["version"], 1);
    assert_eq!(loaded["active_provider"], "nous");
    assert_eq!(loaded["providers"]["nous"]["refresh_token"], "refresh");
}

// Tier: unit — mirrors the corruption quarantine and read-failure split.
#[test]
fn malformed_json_is_quarantined_but_directory_read_errors_propagate() {
    let temp = tempfile::tempdir().unwrap();
    let malformed = store_path(temp.path());
    fs::create_dir_all(malformed.parent().unwrap()).unwrap();
    fs::write(&malformed, b"{ not json").unwrap();

    let loaded = load_auth_store(Some(&malformed)).unwrap();
    assert_eq!(loaded.get("version"), Some(&json!(1)));
    assert_eq!(
        fs::read_to_string(malformed.with_extension("json.corrupt")).unwrap(),
        "{ not json"
    );

    let directory = temp.path().join("directory");
    fs::create_dir(&directory).unwrap();
    let error = load_auth_store(Some(&directory)).unwrap_err();
    assert!(matches!(error.kind(), std::io::ErrorKind::IsADirectory));
    assert!(!directory.with_extension("json.corrupt").exists());
}

// Tier: unit — mirrors _save_auth_store's atomic restricted writer.
#[cfg(unix)]
#[test]
fn save_auth_store_round_trips_with_restricted_file_and_parent_modes() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().unwrap();
    let path = store_path(temp.path());
    let mut store = Map::new();
    store.insert(
        "providers".into(),
        json!({"openrouter": {"api_key": "secret"}}),
    );

    let saved = save_auth_store(&mut store, Some(&path)).unwrap();
    assert_eq!(saved, path);
    assert_eq!(
        path.parent()
            .unwrap()
            .metadata()
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(path.metadata().unwrap().permissions().mode() & 0o777, 0o600);

    let loaded = load_auth_store(Some(&path)).unwrap();
    assert_eq!(loaded["version"], 1);
    assert!(loaded["updated_at"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(loaded["providers"]["openrouter"]["api_key"], "secret");
}

// Tier: mock — mirrors read_credential_pool's per-provider profile fallback.
#[test]
fn read_pool_profile_entries_shadow_global_entries_per_provider() {
    let temp = tempfile::tempdir().unwrap();
    let profile = temp.path().join("profiles/coder/auth.json");
    let global = temp.path().join("auth.json");
    write_json(
        &profile,
        &store_with_pool(json!({
            "openrouter": [{"id": "profile-openrouter"}],
            "empty": []
        })),
    );
    write_json(
        &global,
        &store_with_pool(json!({
            "openrouter": [{"id": "global-openrouter"}],
            "anthropic": [{"id": "global-anthropic"}],
            "empty": [{"id": "global-empty"}]
        })),
    );

    assert_eq!(
        read_credential_pool_at(Some(&profile), Some(&global), Some("openrouter")).unwrap(),
        json!([{"id": "profile-openrouter"}])
    );
    assert_eq!(
        read_credential_pool_at(Some(&profile), Some(&global), Some("anthropic")).unwrap(),
        json!([{ "id": "global-anthropic" }])
    );
    assert_eq!(
        read_credential_pool_at(Some(&profile), Some(&global), Some("empty")).unwrap(),
        json!([{ "id": "global-empty" }])
    );
}

// Tier: mock — mirrors malformed-global fail-open behavior and whole-pool merge.
#[test]
fn read_pool_ignores_malformed_global_and_merges_nonempty_fallbacks() {
    let temp = tempfile::tempdir().unwrap();
    let profile = temp.path().join("profile/auth.json");
    let global = temp.path().join("global/auth.json");
    write_json(
        &profile,
        &store_with_pool(json!({"openrouter": [{"id": "profile"}]})),
    );
    fs::create_dir_all(global.parent().unwrap()).unwrap();
    fs::write(&global, b"{not valid json").unwrap();

    assert_eq!(
        read_credential_pool_at(Some(&profile), Some(&global), None).unwrap(),
        json!({"openrouter": [{"id": "profile"}]})
    );

    write_json(
        &global,
        &store_with_pool(json!({
            "openrouter": [{"id": "global"}],
            "anthropic": [{"id": "global-anthropic"}]
        })),
    );
    assert_eq!(
        read_credential_pool_at(Some(&profile), Some(&global), None).unwrap(),
        json!({
            "openrouter": [{"id": "profile"}],
            "anthropic": [{"id": "global-anthropic"}]
        })
    );
}

// Tier: unit — mirrors write_credential_pool's final borrowed-secret boundary.
#[test]
fn write_pool_sanitizes_borrowed_rows_and_preserves_owned_rows() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("hermes/auth.json");
    let sentinel = "S3NTINEL_DO_NOT_PERSIST";
    let manual_secret = "MANUAL_SECRET_STAYS";
    let entries = vec![
        json!({
            "id": "borrowed",
            "source": "systemd://hermes/openrouter",
            "access_token": sentinel,
            "refresh_token": format!("refresh-{sentinel}"),
            "agent_key": format!("agent-{sentinel}"),
            "api_key": format!("extra-{sentinel}")
        }),
        json!({
            "id": "manual",
            "source": "manual",
            "access_token": manual_secret
        }),
    ];

    write_credential_pool_at(&path, "openrouter", &entries, &[]).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(!text.contains(sentinel));
    assert!(text.contains(manual_secret));
    let loaded = load_auth_store(Some(&path)).unwrap();
    let rows = loaded["credential_pool"]["openrouter"].as_array().unwrap();
    assert_eq!(rows[0]["source"], "systemd://hermes/openrouter");
    assert!(rows[0].get("access_token").is_none());
    assert!(rows[0]["secret_fingerprint"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert_eq!(rows[1]["access_token"], manual_secret);
}

// Tier: unit — mirrors intentional removed_ids handling in write_credential_pool.
#[test]
fn write_pool_does_not_resurrect_intentionally_removed_disk_rows() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("hermes/auth.json");
    let original = vec![json!({"id": "keep"}), json!({"id": "remove"})];
    write_credential_pool_at(&path, "openrouter", &original, &[]).unwrap();
    write_credential_pool_at(
        &path,
        "openrouter",
        &[json!({"id": "keep", "source": "manual"})],
        &["remove".into()],
    )
    .unwrap();

    let loaded = load_auth_store(Some(&path)).unwrap();
    let ids: Vec<_> = loaded["credential_pool"]["openrouter"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["keep"]);
}
