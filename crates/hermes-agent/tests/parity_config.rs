use hermes_agent::config::{load_config_snapshot_at, ConfigSnapshot};
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_config(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write test config");
}

#[test]
fn missing_file_returns_empty_snapshot_with_requested_path() {
    let td = tempdir().expect("tempdir");
    let path = td.path().join("missing.yaml");

    let snapshot = load_config_snapshot_at(&path);

    assert_eq!(snapshot.path(), path.as_path());
    assert_eq!(snapshot.signature(), None);
    assert!(snapshot.pool_config().is_empty());
    assert_eq!(snapshot.model_config(), None);
    assert!(snapshot.custom_providers().is_empty());
}

#[test]
fn valid_yaml_exposes_root_pool_map_and_model_map() {
    let td = tempdir().expect("tempdir");
    let path = td.path().join("config.yaml");
    write_config(
        &path,
        "model:\n  provider: openrouter\n  default: test/model\ncredentials:\n  strategy: round_robin\n",
    );

    let snapshot = load_config_snapshot_at(&path);

    assert_eq!(snapshot.path(), path.as_path());
    assert!(snapshot.signature().is_some());
    assert_eq!(
        snapshot.pool_config()["credentials"]["strategy"],
        json!("round_robin")
    );
    assert_eq!(
        snapshot.model_config().and_then(|m| m.get("provider")),
        Some(&json!("openrouter"))
    );
    assert!(snapshot.custom_providers().is_empty());
}

#[test]
fn legacy_and_keyed_custom_providers_share_compatibility_normalization() {
    let td = tempdir().expect("tempdir");
    let path = td.path().join("config.yaml");
    write_config(
        &path,
        "custom_providers:\n  - name: Legacy API\n    base_url: https://legacy.example/v1\n    model: legacy-model\nproviders:\n  modern-api:\n    baseUrl: https://modern.example/v1\n    defaultModel: modern-model\n",
    );

    let snapshot = load_config_snapshot_at(&path);
    let providers = snapshot.custom_providers();

    assert_eq!(providers.len(), 2);
    assert_eq!(providers[0]["name"], json!("Legacy API"));
    assert_eq!(providers[0]["base_url"], json!("https://legacy.example/v1"));
    assert_eq!(providers[0]["model"], json!("legacy-model"));
    assert_eq!(providers[1]["name"], json!("modern-api"));
    assert_eq!(providers[1]["provider_key"], json!("modern-api"));
    assert_eq!(providers[1]["base_url"], json!("https://modern.example/v1"));
    assert_eq!(providers[1]["model"], json!("modern-model"));
}

#[test]
fn malformed_and_non_map_yaml_fail_open_to_empty_snapshot() {
    let td = tempdir().expect("tempdir");
    let malformed = td.path().join("malformed.yaml");
    let non_map = td.path().join("non-map.yaml");
    write_config(&malformed, "model: [unterminated\n");
    write_config(&non_map, "- one\n- two\n");

    for path in [&malformed, &non_map] {
        let snapshot = load_config_snapshot_at(path);
        assert!(snapshot.pool_config().is_empty());
        assert_eq!(snapshot.model_config(), None);
        assert!(snapshot.custom_providers().is_empty());
    }
}

#[test]
fn changed_path_signature_reloads_snapshot() {
    let td = tempdir().expect("tempdir");
    let path = td.path().join("config.yaml");
    write_config(&path, "model:\n  default: first\n");
    let first = load_config_snapshot_at(&path);
    assert_eq!(
        first.model_config().and_then(|m| m.get("default")),
        Some(&json!("first"))
    );

    write_config(
        &path,
        "model:\n  default: second-model-with-a-different-size\n",
    );
    let second = load_config_snapshot_at(&path);

    assert_ne!(first.signature(), second.signature());
    assert_eq!(
        second.model_config().and_then(|m| m.get("default")),
        Some(&json!("second-model-with-a-different-size"))
    );
}

#[test]
fn snapshot_is_clone_safe() {
    let td = tempdir().expect("tempdir");
    let path = td.path().join("config.yaml");
    write_config(&path, "model:\n  default: clone-me\n");

    let snapshot = load_config_snapshot_at(&path);
    let clone: ConfigSnapshot = snapshot.clone();
    assert_eq!(snapshot, clone);
}
#[test]
fn malformed_revision_serves_last_known_good_snapshot() {
    let td = tempdir().expect("tempdir");
    let path = td.path().join("config.yaml");
    write_config(&path, "model:\n  default: retained-model\n");
    let first = load_config_snapshot_at(&path);
    assert_eq!(
        first.model_config().and_then(|m| m.get("default")),
        Some(&json!("retained-model"))
    );

    write_config(&path, "model: [unterminated\n");
    let retained = load_config_snapshot_at(&path);

    assert_eq!(retained, first);
}
