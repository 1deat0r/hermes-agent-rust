use hermes_agent::config::{
    load_config_snapshot_at, load_merged_config_snapshot_at, ConfigSnapshot,
};
use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_config(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write test config");
}

fn map(value: Value) -> Map<String, Value> {
    value.as_object().expect("object defaults").clone()
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

#[test]
fn merged_loader_deep_merges_defaults_and_ignores_null_map_sections() {
    let td = tempdir().expect("tempdir");
    let path = td.path().join("config.yaml");
    write_config(
        &path,
        "terminal:\n  shell: zsh\nmodel:\n  context_length: 8192\nscalar: null\n",
    );
    let defaults = map(json!({
        "terminal": {"shell": "bash", "env": {"PATH": "default"}},
        "model": {"provider": "openrouter", "context_length": 4096},
        "scalar": "fallback",
    }));

    let snapshot = load_merged_config_snapshot_at(&path, &defaults);

    assert_eq!(snapshot.pool_config()["terminal"]["shell"], json!("zsh"));
    assert_eq!(
        snapshot.pool_config()["terminal"]["env"]["PATH"],
        json!("default")
    );
    assert_eq!(
        snapshot.pool_config()["model"]["provider"],
        json!("openrouter")
    );
    assert_eq!(
        snapshot.pool_config()["model"]["context_length"],
        json!(8192)
    );
    assert_eq!(snapshot.pool_config()["scalar"], Value::Null);
}

#[test]
fn merged_loader_expands_env_refs_and_invalidates_on_value_change() {
    let td = tempdir().expect("tempdir");
    let path = td.path().join("config.yaml");
    write_config(
        &path,
        "key: ${PARITY_CONFIG_KEY}\nprefixed: ${env:PARITY_CONFIG_KEY}\nunknown: ${file:SECRET}\nunknown_spaced: ${ file:SECRET }\n",
    );
    let defaults = Map::new();
    env::set_var("PARITY_CONFIG_KEY", "first");
    let first = load_merged_config_snapshot_at(&path, &defaults);
    assert_eq!(first.pool_config()["key"], json!("first"));
    assert_eq!(first.pool_config()["prefixed"], json!("first"));
    assert_eq!(first.pool_config()["unknown"], json!("${file:SECRET}"));
    assert_eq!(
        first.pool_config()["unknown_spaced"],
        json!("${ file:SECRET }")
    );

    env::set_var("PARITY_CONFIG_KEY", "second");
    let second = load_merged_config_snapshot_at(&path, &defaults);
    assert_eq!(second.pool_config()["key"], json!("second"));
    assert_eq!(second.pool_config()["prefixed"], json!("second"));
    env::remove_var("PARITY_CONFIG_KEY");
}

#[test]
fn merged_loader_normalizes_model_aliases_and_root_max_turns_precedence() {
    let td = tempdir().expect("tempdir");
    let path = td.path().join("config.yaml");
    write_config(
        &path,
        "provider: user-provider\napi_base: https://user.example/v1\ncontext_length: 16384\nmax_turns: 77\nmodel:\n  model: user-model\n  base_url: https://model.example/v1\n",
    );
    let defaults = map(json!({
        "provider": "default-provider",
        "base_url": "https://default.example/v1",
        "context_length": 4096,
        "model": {
            "default": "default-model",
            "provider": "model-provider",
            "base_url": "https://model-default.example/v1",
            "context_length": 8192
        },
        "agent": {"max_turns": 12}
    }));

    let snapshot = load_merged_config_snapshot_at(&path, &defaults);
    let model = snapshot.model_config().expect("normalized model");
    assert_eq!(model.get("default"), Some(&json!("default-model")));
    assert_eq!(model.get("provider"), Some(&json!("model-provider")));
    assert_eq!(
        model.get("base_url"),
        Some(&json!("https://model.example/v1"))
    );
    assert_eq!(model.get("context_length"), Some(&json!(8192)));
    assert!(model.get("model").is_none());
    assert!(model.get("api_base").is_none());
    assert_eq!(snapshot.pool_config()["agent"]["max_turns"], json!(77));
    assert!(snapshot.pool_config().get("max_turns").is_none());
}

#[test]
fn merged_loader_does_not_invent_max_turns_without_a_source_value() {
    let td = tempdir().expect("tempdir");
    let path = td.path().join("config.yaml");
    write_config(&path, "model:\n  default: model\n");
    let snapshot = load_merged_config_snapshot_at(&path, &Map::new());
    assert!(snapshot.pool_config()["agent"].get("max_turns").is_none());
}

#[test]
fn merged_loader_retains_last_known_good_on_malformed_revision() {
    let td = tempdir().expect("tempdir");
    let path = td.path().join("config.yaml");
    let defaults = map(json!({"model": {"provider": "default"}}));
    write_config(&path, "model:\n  default: retained\n");
    let first = load_merged_config_snapshot_at(&path, &defaults);

    write_config(&path, "model: [unterminated\n");
    let retained = load_merged_config_snapshot_at(&path, &defaults);

    assert_eq!(retained, first);
}
