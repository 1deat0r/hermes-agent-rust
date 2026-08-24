use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hermes_agent::credential_pool::{
    credential_pool_matches_provider, custom_provider_config, custom_provider_pool_key,
    get_compatible_custom_providers, get_env_prefer_dotenv, label_from_token,
    list_custom_pool_providers, load_env_file, normalize_custom_pool_name,
    normalize_custom_provider_entry, normalize_pool_priorities, pool_strategy_from_config,
    providers_dict_to_custom_providers, prune_stale_seeded_entries, seed_custom_pool,
    seed_from_env, seed_from_singletons, upsert_entry, CredentialPool, CredentialStatus,
    EnvironmentSnapshot, PoolErrorContext, PoolLoadInputs, PoolStrategy, PooledCredential,
    ProviderCredentialConfig,
};
use hermes_agent::credential_store::{read_credential_pool_at, save_auth_store};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;

fn env_snapshot(dotenv: &[(&str, &str)], process: &[(&str, &str)]) -> EnvironmentSnapshot {
    EnvironmentSnapshot {
        dotenv: dotenv
            .iter()
            .map(|(key, value)| ((*key).into(), (*value).into()))
            .collect(),
        process: process
            .iter()
            .map(|(key, value)| ((*key).into(), (*value).into()))
            .collect(),
        ..EnvironmentSnapshot::default()
    }
}

fn entry(id: &str, key: &str, priority: i32) -> PooledCredential {
    PooledCredential::new("openrouter", id, key, priority)
}

// Tier: unit — mirrors tests/agent/test_credential_pool.py::test_load_pool_seeds_env_api_key.
#[test]
fn environment_seed_discovers_openrouter_api_key() {
    let snapshot = env_snapshot(&[], &[("OPENROUTER_API_KEY", "sk-or-seeded")]);
    let mut entries = Vec::new();

    let result = seed_from_env("openrouter", &mut entries, None, &snapshot);

    assert!(result.changed);
    assert_eq!(
        result.active_sources,
        BTreeSet::from([String::from("env:OPENROUTER_API_KEY")])
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "env:OPENROUTER_API_KEY");
    assert_eq!(entries[0].access_token, "sk-or-seeded");
    assert_eq!(
        entries[0].base_url.as_deref(),
        Some(hermes_constants::OPENROUTER_BASE_URL)
    );
}

// Tier: unit — mirrors tests/tools/test_credential_pool_env_fallback.py dotenv precedence.
#[test]
fn environment_seed_prefers_dotenv_and_falls_back_to_process_environment() {
    let config = ProviderCredentialConfig::api_key(
        ["DEEPSEEK_API_KEY"],
        None::<String>,
        "https://api.deepseek.com/v1",
    );
    let fresh = env_snapshot(
        &[("DEEPSEEK_API_KEY", "sk-dotenv-fresh")],
        &[("DEEPSEEK_API_KEY", "sk-shell-stale")],
    );
    let mut entries = Vec::new();
    seed_from_env("deepseek", &mut entries, Some(&config), &fresh);
    assert_eq!(entries[0].access_token, "sk-dotenv-fresh");

    let fallback = env_snapshot(&[], &[("DEEPSEEK_API_KEY", "sk-runtime-env")]);
    let mut entries = Vec::new();
    seed_from_env("deepseek", &mut entries, Some(&config), &fallback);
    assert_eq!(entries[0].access_token, "sk-runtime-env");
}

// Tier: unit — mirrors tests/agent/test_credential_pool.py duplicate env rows.
#[test]
fn environment_seed_collapses_duplicate_source_rows_to_first_identity() {
    let mut first = PooledCredential::from_json(
        "openrouter",
        &json!({
            "id": "current-row",
            "label": "OPENROUTER_API_KEY",
            "source": "env:OPENROUTER_API_KEY",
            "access_token": "old-key",
        }),
    );
    first.priority = 4;
    let duplicate = PooledCredential::from_json(
        "openrouter",
        &json!({
            "id": "stale-duplicate",
            "label": "OPENROUTER_API_KEY",
            "source": "env:OPENROUTER_API_KEY",
            "access_token": "stale-key",
        }),
    );
    let mut entries = vec![first, duplicate];
    let snapshot = env_snapshot(&[], &[("OPENROUTER_API_KEY", "active-key")]);

    let result = seed_from_env("openrouter", &mut entries, None, &snapshot);

    assert!(result.changed);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "current-row");
    assert_eq!(entries[0].priority, 4);
    assert_eq!(entries[0].access_token, "active-key");
}

// Tier: unit — mirrors tests/tools/test_credential_pool_env_fallback.py
// AnthropicEnvAuthTypeClassification.
#[test]
fn anthropic_oat_environment_token_is_oauth() {
    let snapshot = env_snapshot(&[("CLAUDE_CODE_OAUTH_TOKEN", "sk-ant-oat-fake")], &[]);
    let config = ProviderCredentialConfig::api_key(
        [
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_TOKEN",
            "CLAUDE_CODE_OAUTH_TOKEN",
        ],
        None::<String>,
        "https://api.anthropic.com",
    );
    let mut entries = Vec::new();

    seed_from_env("anthropic", &mut entries, Some(&config), &snapshot);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].auth_type, "oauth");
}

// Tier: unit — mirrors the admin-key regression in
// tests/tools/test_credential_pool_env_fallback.py.
#[test]
fn anthropic_non_oat_environment_token_remains_api_key() {
    let snapshot = env_snapshot(&[("ANTHROPIC_API_KEY", "sk-ant-admin-fake")], &[]);
    let config = ProviderCredentialConfig::api_key(
        [
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_TOKEN",
            "CLAUDE_CODE_OAUTH_TOKEN",
        ],
        None::<String>,
        "https://api.anthropic.com",
    );
    let mut entries = Vec::new();

    seed_from_env("anthropic", &mut entries, Some(&config), &snapshot);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].auth_type, "api_key");
}

// Tier: unit — mirrors hermes_cli.config.load_env's quoted/export parser.
#[test]
fn dotenv_parser_preserves_assignment_values_and_supports_export() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(".env");
    fs::write(
        &path,
        "# ignored\nexport API_KEY=plain\nQUOTED=\"a\\\"b\\\\c\"\nSINGLE='opaque value'\nOPAQUE=a=b=c\n",
    )
    .unwrap();

    let parsed = load_env_file(&path).unwrap();

    assert_eq!(parsed.get("API_KEY"), Some(&String::from("plain")));
    assert_eq!(parsed.get("QUOTED"), Some(&String::from("a\"b\\c")));
    assert_eq!(parsed.get("SINGLE"), Some(&String::from("opaque value")));
    assert_eq!(parsed.get("OPAQUE"), Some(&String::from("a=b=c")));

    assert!(load_env_file(&temp.path().join("missing.env"))
        .unwrap()
        .is_empty());
}

// Tier: unit — mirrors get_env_prefer_dotenv's op:// secret-scope exception.
#[test]
fn unresolved_dotenv_secret_uses_active_scope_value() {
    let mut snapshot = env_snapshot(&[("OPENROUTER_API_KEY", "op://Vault/Item/key")], &[]);
    snapshot
        .secret_scope
        .insert("OPENROUTER_API_KEY".into(), "resolved-secret".into());

    assert_eq!(
        get_env_prefer_dotenv("OPENROUTER_API_KEY", &snapshot),
        "resolved-secret"
    );
}

// Tier: unit — mirrors suppression and non-destructive env pruning behavior.
#[test]
fn environment_suppression_blocks_seed_and_missing_env_is_retained_unless_explicitly_pruned() {
    let mut snapshot = env_snapshot(&[], &[("OPENROUTER_API_KEY", "suppressed")]);
    snapshot
        .suppressed_sources
        .insert("env:OPENROUTER_API_KEY".into());
    let mut entries = Vec::new();
    let result = seed_from_env("openrouter", &mut entries, None, &snapshot);
    assert!(!result.changed);
    assert!(entries.is_empty());

    let env_entry = PooledCredential::from_json(
        "openrouter",
        &json!({
            "id": "env-row",
            "source": "env:OPENROUTER_API_KEY",
            "access_token": "runtime-only",
        }),
    );
    let mut entries = vec![env_entry.clone()];
    assert!(!prune_stale_seeded_entries(
        &mut entries,
        &BTreeSet::new(),
        false,
    ));
    assert_eq!(entries, vec![env_entry]);
    assert!(prune_stale_seeded_entries(
        &mut entries,
        &BTreeSet::new(),
        true,
    ));
    assert!(entries.is_empty());
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::test_load_pool_does_not_persist_env_seeded_secret_value.
#[test]
fn environment_pool_loader_persists_metadata_without_borrowed_secret() {
    let temp = tempfile::tempdir().unwrap();
    let auth_path = temp.path().join("hermes/auth.json");
    let mut store = serde_json::Map::new();
    store.insert("providers".into(), json!({}));
    save_auth_store(&mut store, Some(&auth_path)).unwrap();
    let snapshot = env_snapshot(
        &[],
        &[("OPENROUTER_API_KEY", "S3NTINEL_DO_NOT_PERSIST_OPENROUTER")],
    );

    let pool = hermes_agent::credential_pool::load_pool_with_environment_at(
        "openrouter",
        None,
        None,
        &snapshot,
        &auth_path,
        None,
    )
    .unwrap();

    assert_eq!(pool.entries().len(), 1);
    assert_eq!(
        pool.entries()[0].access_token,
        "S3NTINEL_DO_NOT_PERSIST_OPENROUTER"
    );
    let raw = fs::read_to_string(&auth_path).unwrap();
    assert!(!raw.contains("S3NTINEL_DO_NOT_PERSIST_OPENROUTER"));
    let persisted = read_credential_pool_at(Some(&auth_path), None, Some("openrouter")).unwrap();
    let row = persisted
        .as_array()
        .unwrap()
        .first()
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(row["source"], "env:OPENROUTER_API_KEY");
    assert_eq!(row["label"], "OPENROUTER_API_KEY");
    assert_eq!(row["priority"], 0);
    assert!(row.get("access_token").is_none());
    assert!(row["secret_fingerprint"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::test_load_pool_collapses_duplicate_env_rows_to_active_key.
#[test]
fn environment_pool_loader_collapses_duplicate_rows_and_keeps_first_identity() {
    let temp = tempfile::tempdir().unwrap();
    let auth_path = temp.path().join("hermes/auth.json");
    let mut store = serde_json::Map::new();
    store.insert(
        "credential_pool".into(),
        json!({
            "openrouter": [
                {
                    "id": "current-row",
                    "label": "OPENROUTER_API_KEY",
                    "auth_type": "api_key",
                    "priority": 0,
                    "source": "env:OPENROUTER_API_KEY"
                },
                {
                    "id": "stale-duplicate",
                    "label": "OPENROUTER_API_KEY",
                    "auth_type": "api_key",
                    "priority": 1,
                    "source": "env:OPENROUTER_API_KEY"
                }
            ]
        }),
    );
    save_auth_store(&mut store, Some(&auth_path)).unwrap();
    let snapshot = env_snapshot(&[], &[("OPENROUTER_API_KEY", "active-key")]);

    let pool = hermes_agent::credential_pool::load_pool_with_environment_at(
        "openrouter",
        None,
        None,
        &snapshot,
        &auth_path,
        None,
    )
    .unwrap();

    assert_eq!(
        pool.entries()
            .iter()
            .map(|entry| (entry.id.clone(), entry.access_token.clone()))
            .collect::<Vec<_>>(),
        vec![("current-row".into(), "active-key".into())]
    );
    let persisted = read_credential_pool_at(Some(&auth_path), None, Some("openrouter")).unwrap();
    assert_eq!(
        persisted
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["id"].clone())
            .collect::<Vec<_>>(),
        vec![json!("current-row")]
    );
}

// Tier: mock — mirrors agent/credential_pool.py::load_pool's non-custom branch
// and tests/agent/test_credential_pool.py's environment/singleton loader cases.
#[test]
fn composed_pool_loader_runs_singleton_and_env_seeding_and_prunes_stale_rows() {
    let temp = tempfile::tempdir().unwrap();
    let auth_path = temp.path().join("hermes/auth.json");
    let mut store = serde_json::Map::new();
    store.insert(
        "credential_pool".into(),
        json!({
            "anthropic": [
                {
                    "id": "stale-claude-code",
                    "source": "claude_code",
                    "auth_type": "oauth",
                    "access_token": "stale-oauth-token",
                    "priority": 0,
                    "label": "stale"
                }
            ]
        }),
    );
    save_auth_store(&mut store, Some(&auth_path)).unwrap();

    let snapshot = env_snapshot(&[], &[("ANTHROPIC_TOKEN", "sk-ant-api03-env")]);
    let singleton = json!({
        "provider_explicitly_configured": true,
        "api_key_path_explicit": false,
        "hermes_pkce": {
            "accessToken": "sk-ant-oat01-pkce",
            "refreshToken": "pkce-refresh",
            "expiresAt": 1_900_000_000_000i64
        }
    });
    let singleton = singleton.as_object().unwrap().clone();
    let provider_config = ProviderCredentialConfig::api_key(
        ["ANTHROPIC_API_KEY"],
        None::<String>,
        "https://api.anthropic.com/v1",
    );

    let pool = hermes_agent::credential_pool::load_pool_with_inputs_at(
        "anthropic",
        PoolLoadInputs {
            provider_config: Some(&provider_config),
            pool_config: None,
            snapshot: &snapshot,
            singleton_state: Some(&singleton),
            custom_providers: &[],
            model_config: None,
            profile_path: &auth_path,
            global_path: None,
        },
    )
    .unwrap();

    let loaded_entries = pool.entries();
    let sources = loaded_entries
        .iter()
        .map(|entry| entry.source.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        sources,
        BTreeSet::from(["env:ANTHROPIC_TOKEN", "hermes_pkce"])
    );
    assert!(loaded_entries
        .iter()
        .any(|entry| entry.access_token == "sk-ant-oat01-pkce"));
    assert!(!loaded_entries
        .iter()
        .any(|entry| entry.source == "claude_code"));

    let persisted = read_credential_pool_at(Some(&auth_path), None, Some("anthropic"))
        .unwrap()
        .as_array()
        .unwrap()
        .to_vec();
    assert_eq!(persisted.len(), 2);
    assert!(!persisted
        .iter()
        .any(|entry| entry["source"] == "claude_code"));
}

// Tier: mock — mirrors agent/credential_pool.py::load_pool's custom:* branch
// and tests/agent/test_credential_pool.py::test_custom_endpoint_pool_seeds_*.
#[test]
fn composed_custom_pool_uses_custom_sources_without_singleton_or_env_seeding() {
    let temp = tempfile::tempdir().unwrap();
    let auth_path = temp.path().join("hermes/auth.json");
    let mut store = serde_json::Map::new();
    store.insert(
        "credential_pool".into(),
        json!({
            "custom:relay": [
                {
                    "id": "stale-oauth",
                    "source": "hermes_pkce",
                    "auth_type": "oauth",
                    "access_token": "stale-oauth-token",
                    "priority": 0,
                    "label": "stale"
                },
                {
                    "id": "manual-row",
                    "source": "manual",
                    "auth_type": "api_key",
                    "access_token": "manual-key",
                    "priority": 1,
                    "label": "manual"
                }
            ]
        }),
    );
    save_auth_store(&mut store, Some(&auth_path)).unwrap();

    let providers = vec![json!({
        "name": "Relay",
        "base_url": "https://relay.example/v1/",
        "api_key": "config-key"
    })];
    let model = json!({
        "provider": "custom",
        "base_url": "https://relay.example/v1",
        "api_key": "model-key"
    });
    let model = model.as_object().unwrap().clone();
    let snapshot = env_snapshot(&[], &[("OPENROUTER_API_KEY", "must-not-seed")]);
    let singleton = json!({
        "hermes_pkce": {"accessToken": "must-not-seed"}
    });
    let singleton = singleton.as_object().unwrap().clone();

    let pool = hermes_agent::credential_pool::load_pool_with_inputs_at(
        "custom:relay",
        PoolLoadInputs {
            provider_config: None,
            pool_config: None,
            snapshot: &snapshot,
            singleton_state: Some(&singleton),
            custom_providers: &providers,
            model_config: Some(&model),
            profile_path: &auth_path,
            global_path: None,
        },
    )
    .unwrap();

    let loaded_entries = pool.entries();
    let sources = loaded_entries
        .iter()
        .map(|entry| entry.source.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        sources,
        BTreeSet::from(["config:Relay", "manual", "model_config"])
    );
    assert!(!loaded_entries
        .iter()
        .any(|entry| entry.access_token == "must-not-seed"));
    assert_eq!(
        loaded_entries
            .iter()
            .find(|entry| entry.source == "config:Relay")
            .and_then(|entry| entry.base_url.as_deref()),
        Some("https://relay.example/v1")
    );
    let persisted_payload =
        read_credential_pool_at(Some(&auth_path), None, Some("custom:relay")).unwrap();
    let persisted = persisted_payload.as_array().unwrap();
    assert!(!persisted
        .iter()
        .any(|entry| entry["source"] == "hermes_pkce"));
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::test_load_pool_mirrors_nous_invoke_jwt_agent_key_runtime_api_key.
#[test]
fn nous_singleton_seed_materializes_invoke_jwt_agent_key() {
    let token = jwt_with_claims(json!({
        "scope": ["inference:invoke"],
        "exp": 4_000_000_000i64,
    }));
    let state = json!({
        "portal_base_url": "https://portal.example.com",
        "inference_base_url": "https://inference.example.com/v1",
        "client_id": "hermes-cli",
        "token_type": "Bearer",
        "scope": "inference:invoke",
        "access_token": token,
        "refresh_token": "refresh-token",
        "expires_at": "2096-10-02T07:06:40+00:00",
        "agent_key": token,
        "agent_key_expires_at": "2096-10-02T07:06:40+00:00"
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_from_singletons("nous", &mut entries, Some(&state), &BTreeSet::new());

    assert!(result.changed);
    assert_eq!(
        result.active_sources,
        BTreeSet::from([String::from("device_code")])
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "device_code");
    assert_eq!(
        entries[0].agent_key.as_deref(),
        entries[0].access_token.as_str().into()
    );
    assert_eq!(entries[0].runtime_api_key(), entries[0].access_token);
    assert_eq!(
        entries[0].inference_base_url.as_deref(),
        Some("https://inference.example.com/v1")
    );
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::test_nous_seed_from_singletons_preserves_obtained_at_timestamps.
#[test]
fn nous_singleton_seed_preserves_obtained_at_and_refresh_metadata() {
    let state = json!({
        "access_token": "at_XXXXXXXX",
        "refresh_token": "rt_YYYYYYYY",
        "client_id": "hermes-cli",
        "portal_base_url": "https://portal.nousresearch.com",
        "inference_base_url": "https://inference.nousresearch.com/v1",
        "token_type": "Bearer",
        "scope": "openid profile",
        "obtained_at": "2026-04-24T10:00:00+00:00",
        "expires_at": "2026-04-24T11:00:00+00:00",
        "expires_in": 3600,
        "agent_key": "sk-nous-AAAA",
        "agent_key_id": "ak_123",
        "agent_key_expires_at": "2026-04-25T10:00:00+00:00",
        "agent_key_expires_in": 86400,
        "agent_key_reused": false,
        "agent_key_obtained_at": "2026-04-24T10:00:05+00:00",
        "tls": {"insecure": false, "ca_bundle": null}
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    seed_from_singletons("nous", &mut entries, Some(&state), &BTreeSet::new());

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.access_token, "at_XXXXXXXX");
    assert_eq!(entry.refresh_token.as_deref(), Some("rt_YYYYYYYY"));
    assert_eq!(
        entry.expires_at.as_deref(),
        Some("2026-04-24T11:00:00+00:00")
    );
    assert_eq!(entry.agent_key.as_deref(), Some("sk-nous-AAAA"));
    assert_eq!(
        entry.agent_key_expires_at.as_deref(),
        Some("2026-04-25T10:00:00+00:00")
    );
    assert_eq!(
        entry.extra.get("obtained_at"),
        Some(&json!("2026-04-24T10:00:00+00:00"))
    );
    assert_eq!(entry.extra.get("expires_in"), Some(&json!(3600)));
    assert_eq!(entry.extra.get("agent_key_id"), Some(&json!("ak_123")));
    assert_eq!(
        entry.extra.get("agent_key_obtained_at"),
        Some(&json!("2026-04-24T10:00:05+00:00"))
    );
    assert_eq!(
        entry.extra.get("tls"),
        Some(&json!({"insecure": false, "ca_bundle": null}))
    );
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::test_load_pool_seeds_qwen_oauth_via_cli_tokens.
#[test]
fn qwen_singleton_seed_materializes_cli_token_and_metadata() {
    let state = json!({
        "provider": "qwen-oauth",
        "base_url": "https://portal.qwen.ai/v1",
        "api_key": "qwen_fake_token_xyz",
        "source": "qwen-cli",
        "expires_at_ms": 1_900_000_000_000i64,
        "auth_file": "/tmp/.qwen/oauth_creds.json"
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_from_singletons("qwen-oauth", &mut entries, Some(&state), &BTreeSet::new());

    assert!(result.changed);
    assert_eq!(
        result.active_sources,
        BTreeSet::from([String::from("qwen-cli")])
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "qwen-cli");
    assert_eq!(entries[0].auth_type, "oauth");
    assert_eq!(entries[0].access_token, "qwen_fake_token_xyz");
    assert_eq!(entries[0].expires_at_ms, Some(1_900_000_000_000));
    assert_eq!(
        entries[0].base_url.as_deref(),
        Some("https://portal.qwen.ai/v1")
    );
    assert_eq!(entries[0].label, "/tmp/.qwen/oauth_creds.json");
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::test_load_pool_does_not_seed_qwen_oauth_when_no_token.
#[test]
fn qwen_singleton_seed_fails_open_when_cli_token_is_absent() {
    let state = json!({
        "provider": "qwen-oauth",
        "source": "qwen-cli",
        "api_key": ""
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_from_singletons("qwen-oauth", &mut entries, Some(&state), &BTreeSet::new());

    assert!(!result.changed);
    assert!(result.active_sources.is_empty());
    assert!(entries.is_empty());
}

// Tier: mock — mirrors the upstream agent/credential_pool.py MiniMax OAuth
// singleton branch; no dedicated upstream singleton-seeding test exists.
#[test]
fn minimax_oauth_singleton_seed_materializes_token_and_metadata() {
    let state = json!({
        "provider": "minimax-oauth",
        "access_token": "minimax_access_token",
        "refresh_token": "minimax_refresh_token",
        "expires_at": "2026-08-24T12:34:56.789+00:00",
        "inference_base_url": "https://api.minimax.io/v1///",
        "label": "work-account"
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_from_singletons(
        "minimax-oauth",
        &mut entries,
        Some(&state),
        &BTreeSet::new(),
    );

    assert!(result.changed);
    assert_eq!(
        result.active_sources,
        BTreeSet::from([String::from("oauth")])
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "oauth");
    assert_eq!(entries[0].auth_type, "oauth");
    assert_eq!(entries[0].access_token, "minimax_access_token");
    assert_eq!(
        entries[0].refresh_token.as_deref(),
        Some("minimax_refresh_token")
    );
    assert_eq!(entries[0].expires_at_ms, Some(1_787_574_896_789));
    assert_eq!(
        entries[0].base_url.as_deref(),
        Some("https://api.minimax.io/v1")
    );
    assert_eq!(entries[0].label, "work-account");
}

// Tier: mock — mirrors the upstream label_from_token fallback and fail-open
// expiry conversion in the MiniMax OAuth singleton branch.
#[test]
fn minimax_oauth_singleton_seed_uses_token_label_and_ignores_bad_expiry() {
    let token = jwt_with_claims(json!({"email": "minimax@example.test"}));
    let state = json!({
        "access_token": token,
        "expires_at": "not-an-iso-timestamp",
        "inference_base_url": ""
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_from_singletons(
        "minimax-oauth",
        &mut entries,
        Some(&state),
        &BTreeSet::new(),
    );

    assert!(result.changed);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].label, "minimax@example.test");
    assert_eq!(entries[0].expires_at_ms, None);
    assert_eq!(entries[0].base_url.as_deref(), Some(""));
}

// Tier: mock — mirrors the upstream per-source suppression gate.
#[test]
fn minimax_oauth_singleton_seed_respects_oauth_suppression() {
    let state = json!({
        "access_token": "minimax_access_token",
        "inference_base_url": "https://api.minimax.io/v1"
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();
    let suppressed = BTreeSet::from([String::from("oauth")]);

    let result = seed_from_singletons("minimax-oauth", &mut entries, Some(&state), &suppressed);

    assert!(!result.changed);
    assert!(result.active_sources.is_empty());
    assert!(entries.is_empty());
}

// Tier: mock — mirrors tests/hermes_cli/test_auth_codex_provider.py's
// auth-store token shape and the upstream openai-codex singleton branch.
#[test]
fn openai_codex_singleton_seed_materializes_nested_tokens_and_metadata() {
    let state = json!({
        "provider": "openai-codex",
        "tokens": {
            "access_token": "codex_access_token",
            "refresh_token": "codex_refresh_token"
        },
        "last_refresh": "2026-08-24T12:00:00+00:00",
        "label": "work-codex"
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_from_singletons("openai-codex", &mut entries, Some(&state), &BTreeSet::new());

    assert!(result.changed);
    assert_eq!(
        result.active_sources,
        BTreeSet::from([String::from("device_code")])
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "device_code");
    assert_eq!(entries[0].auth_type, "oauth");
    assert_eq!(entries[0].access_token, "codex_access_token");
    assert_eq!(
        entries[0].refresh_token.as_deref(),
        Some("codex_refresh_token")
    );
    assert_eq!(
        entries[0].base_url.as_deref(),
        Some("https://chatgpt.com/backend-api/codex")
    );
    assert_eq!(
        entries[0].last_refresh.as_deref(),
        Some("2026-08-24T12:00:00+00:00")
    );
    assert_eq!(entries[0].label, "work-codex");
}

// Tier: mock — mirrors the upstream label_from_token fallback and the
// fail-open path when the nested Codex token object has no access token.
#[test]
fn openai_codex_singleton_seed_uses_token_label_and_fails_open_without_access() {
    let token = jwt_with_claims(json!({"email": "codex@example.test"}));
    let state = json!({
        "tokens": {"access_token": token},
        "last_refresh": "2026-08-24T12:00:00+00:00"
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_from_singletons("openai-codex", &mut entries, Some(&state), &BTreeSet::new());

    assert!(result.changed);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].label, "codex@example.test");

    let empty_state = json!({"tokens": {"refresh_token": "refresh-only"}});
    let empty_state = empty_state.as_object().unwrap().clone();
    let mut empty_entries = Vec::new();
    let empty_result = seed_from_singletons(
        "openai-codex",
        &mut empty_entries,
        Some(&empty_state),
        &BTreeSet::new(),
    );
    assert!(!empty_result.changed);
    assert!(empty_result.active_sources.is_empty());
    assert!(empty_entries.is_empty());
}

// Tier: mock — mirrors the upstream device_code suppression gate.
#[test]
fn openai_codex_singleton_seed_respects_device_code_suppression() {
    let state = json!({
        "tokens": {"access_token": "codex_access_token"}
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();
    let suppressed = BTreeSet::from([String::from("device_code")]);

    let result = seed_from_singletons("openai-codex", &mut entries, Some(&state), &suppressed);

    assert!(!result.changed);
    assert!(result.active_sources.is_empty());
    assert!(entries.is_empty());
}

// Tier: mock — mirrors tests/hermes_cli/test_auth_xai_oauth_provider.py::
// test_credential_pool_seeds_xai_oauth_from_singleton.
#[test]
fn xai_oauth_singleton_seed_materializes_nested_tokens_and_fixed_base_url() {
    let state = json!({
        "provider": "xai-oauth",
        "tokens": {
            "access_token": "xai_access_token",
            "refresh_token": "xai_refresh_token"
        },
        "last_refresh": "2026-08-24T12:00:00+00:00"
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_from_singletons("xai-oauth", &mut entries, Some(&state), &BTreeSet::new());

    assert!(result.changed);
    assert_eq!(
        result.active_sources,
        BTreeSet::from([String::from("device_code")])
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "device_code");
    assert_eq!(entries[0].auth_type, "oauth");
    assert_eq!(entries[0].access_token, "xai_access_token");
    assert_eq!(
        entries[0].refresh_token.as_deref(),
        Some("xai_refresh_token")
    );
    assert_eq!(entries[0].base_url.as_deref(), Some("https://api.x.ai/v1"));
    assert_eq!(
        entries[0].last_refresh.as_deref(),
        Some("2026-08-24T12:00:00+00:00")
    );
    assert_eq!(entries[0].label, "device_code");
}

// Tier: mock — mirrors the upstream label_from_token fallback for the xAI
// OAuth singleton branch.
#[test]
fn xai_oauth_singleton_seed_uses_token_label() {
    let token = jwt_with_claims(json!({"email": "xai@example.test"}));
    let state = json!({"tokens": {"access_token": token}});
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_from_singletons("xai-oauth", &mut entries, Some(&state), &BTreeSet::new());

    assert!(result.changed);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].label, "xai@example.test");
}

// Tier: mock — mirrors tests/hermes_cli/test_auth_xai_oauth_provider.py::
// test_credential_pool_device_code_seed_respects_suppression and the
// missing-token fail-open path.
#[test]
fn xai_oauth_singleton_seed_respects_suppression_and_missing_tokens() {
    let state = json!({
        "tokens": {"access_token": "xai_access_token"}
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();
    let suppressed = BTreeSet::from([String::from("device_code")]);

    let result = seed_from_singletons("xai-oauth", &mut entries, Some(&state), &suppressed);

    assert!(!result.changed);
    assert!(result.active_sources.is_empty());
    assert!(entries.is_empty());

    let empty_state = json!({"tokens": {"refresh_token": "refresh-only"}});
    let empty_state = empty_state.as_object().unwrap().clone();
    let empty_result = seed_from_singletons(
        "xai-oauth",
        &mut entries,
        Some(&empty_state),
        &BTreeSet::new(),
    );
    assert!(!empty_result.changed);
    assert!(empty_result.active_sources.is_empty());
    assert!(entries.is_empty());
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::
// test_load_pool_oauth_path_still_autodiscovers and the upstream Anthropic
// singleton branch's resolved credential-file inputs.
#[test]
fn anthropic_singleton_seed_materializes_resolved_oauth_sources() {
    let state = json!({
        "provider_explicitly_configured": true,
        "api_key_path_explicit": false,
        "hermes_pkce": {
            "accessToken": "sk-ant-oat01-pkce-token",
            "refreshToken": "pkce-refresh",
            "expiresAt": 1_900_000_000_000i64
        },
        "claude_code": {
            "accessToken": "sk-ant-oat01-claude-code-token",
            "refreshToken": "claude-refresh",
            "expiresAt": 1_900_000_100_000i64
        }
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_from_singletons("anthropic", &mut entries, Some(&state), &BTreeSet::new());

    assert!(result.changed);
    assert_eq!(
        result.active_sources,
        BTreeSet::from([String::from("claude_code"), String::from("hermes_pkce")])
    );
    assert_eq!(entries.len(), 2);
    let pkce = entries
        .iter()
        .find(|entry| entry.source == "hermes_pkce")
        .unwrap();
    assert_eq!(pkce.auth_type, "oauth");
    assert_eq!(pkce.access_token, "sk-ant-oat01-pkce-token");
    assert_eq!(pkce.refresh_token.as_deref(), Some("pkce-refresh"));
    assert_eq!(pkce.expires_at_ms, Some(1_900_000_000_000));
    assert_eq!(pkce.label, "hermes_pkce");
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.source == "claude_code")
            .unwrap()
            .expires_at_ms,
        Some(1_900_000_100_000)
    );
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::
// test_load_pool_api_key_path_prunes_stale_oauth_entries.
#[test]
fn anthropic_singleton_seed_prunes_autodiscovered_sources_on_api_key_path() {
    let state = json!({
        "provider_explicitly_configured": true,
        "api_key_path_explicit": true
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = vec![
        PooledCredential::from_json(
            "anthropic",
            &json!({
                "id": "stale-cc",
                "source": "claude_code",
                "auth_type": "oauth",
                "access_token": "stale-cc-token"
            }),
        ),
        PooledCredential::from_json(
            "anthropic",
            &json!({
                "id": "stale-pkce",
                "source": "hermes_pkce",
                "auth_type": "oauth",
                "access_token": "stale-pkce-token"
            }),
        ),
        PooledCredential::from_json(
            "anthropic",
            &json!({
                "id": "manual",
                "source": "manual",
                "auth_type": "api_key",
                "access_token": "manual-token"
            }),
        ),
    ];

    let result = seed_from_singletons("anthropic", &mut entries, Some(&state), &BTreeSet::new());

    assert!(result.changed);
    assert!(result.active_sources.is_empty());
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "manual");
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::
// test_load_pool_does_not_seed_claude_code_when_anthropic_not_configured.
#[test]
fn anthropic_singleton_seed_requires_explicit_provider_configuration() {
    let state = json!({
        "provider_explicitly_configured": false,
        "api_key_path_explicit": false,
        "claude_code": {"accessToken": "sk-ant-oat01-token"}
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_from_singletons("anthropic", &mut entries, Some(&state), &BTreeSet::new());

    assert!(!result.changed);
    assert!(result.active_sources.is_empty());
    assert!(entries.is_empty());
}

// Tier: mock — mirrors the shared per-source suppression gate.
#[test]
fn anthropic_singleton_seed_respects_source_suppression() {
    let state = json!({
        "provider_explicitly_configured": true,
        "api_key_path_explicit": false,
        "hermes_pkce": {"accessToken": "sk-ant-oat01-pkce-token"},
        "claude_code": {"accessToken": "sk-ant-oat01-claude-token"}
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();
    let suppressed = BTreeSet::from([String::from("claude_code")]);

    let result = seed_from_singletons("anthropic", &mut entries, Some(&state), &suppressed);

    assert!(result.changed);
    assert_eq!(
        result.active_sources,
        BTreeSet::from([String::from("hermes_pkce")])
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "hermes_pkce");
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::
// test_load_pool_seeds_copilot_via_gh_auth_token.
#[test]
fn copilot_singleton_seed_materializes_gh_cli_token_and_default_endpoint() {
    let state = json!({
        "token": "gho_raw_token",
        "source": "gh auth token",
        "api_token": "capi_exchanged_token"
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_from_singletons("copilot", &mut entries, Some(&state), &BTreeSet::new());

    assert!(result.changed);
    assert_eq!(
        result.active_sources,
        BTreeSet::from([String::from("gh_cli")])
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "gh_cli");
    assert_eq!(entries[0].auth_type, "api_key");
    assert_eq!(entries[0].access_token, "capi_exchanged_token");
    assert_eq!(
        entries[0].base_url.as_deref(),
        Some("https://api.githubcopilot.com")
    );
    assert_eq!(entries[0].label, "gh auth token");
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::
// test_load_pool_skips_exchange_for_suppressed_copilot.
#[test]
fn copilot_singleton_seed_respects_gh_cli_suppression() {
    let state = json!({
        "token": "gho_raw_token",
        "source": "gh auth token",
        "api_token": "capi_exchanged_token"
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();
    let suppressed = BTreeSet::from([String::from("gh_cli")]);

    let result = seed_from_singletons("copilot", &mut entries, Some(&state), &suppressed);

    assert!(!result.changed);
    assert!(result.active_sources.is_empty());
    assert!(entries.is_empty());
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::
// test_load_pool_respects_env_var_copilot_suppression.
#[test]
fn copilot_singleton_seed_respects_env_source_suppression() {
    let state = json!({
        "token": "gho_raw_token",
        "source": "GH_TOKEN",
        "api_token": "capi_exchanged_token"
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();
    let suppressed = BTreeSet::from([String::from("env:GH_TOKEN")]);

    let result = seed_from_singletons("copilot", &mut entries, Some(&state), &suppressed);

    assert!(!result.changed);
    assert!(result.active_sources.is_empty());
    assert!(entries.is_empty());
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::
// test_load_pool_gh_cli_suppression_does_not_block_env_tokens.
#[test]
fn copilot_singleton_seed_keeps_env_token_when_gh_cli_is_suppressed() {
    let state = json!({
        "token": "gho_raw_env_token",
        "source": "GH_TOKEN",
        "api_token": "capi_exchanged_token",
        "enterprise_base_url": "https://enterprise.githubcopilot.com"
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();
    let suppressed = BTreeSet::from([String::from("gh_cli")]);

    let result = seed_from_singletons("copilot", &mut entries, Some(&state), &suppressed);

    assert!(result.changed);
    assert_eq!(
        result.active_sources,
        BTreeSet::from([String::from("env:GH_TOKEN")])
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "env:GH_TOKEN");
    assert_eq!(entries[0].access_token, "capi_exchanged_token");
    assert_eq!(
        entries[0].base_url.as_deref(),
        Some("https://enterprise.githubcopilot.com")
    );
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::
// test_load_pool_skips_resolve_when_all_copilot_sources_suppressed.
#[test]
fn copilot_singleton_seed_fails_open_when_all_sources_are_suppressed() {
    let state = json!({
        "token": "gho_raw_token",
        "source": "gh auth token",
        "api_token": "capi_exchanged_token"
    });
    let state = state.as_object().unwrap().clone();
    let mut entries = Vec::new();
    let suppressed = BTreeSet::from([
        String::from("gh_cli"),
        String::from("env:COPILOT_GITHUB_TOKEN"),
        String::from("env:GH_TOKEN"),
        String::from("env:GITHUB_TOKEN"),
    ]);

    let result = seed_from_singletons("copilot", &mut entries, Some(&state), &suppressed);

    assert!(!result.changed);
    assert!(result.active_sources.is_empty());
    assert!(entries.is_empty());
}

// Tier: unit — mirrors agent/credential_pool.py _select_unlocked fill-first path.
#[test]
fn fill_first_selection_follows_priority_and_tracks_current_entry() {
    let mut pool = CredentialPool::new(
        "openrouter",
        vec![entry("second", "key-b", 1), entry("first", "key-a", 0)],
        PoolStrategy::FillFirst,
    );

    let selected = pool.select(1_700_000_000.0).expect("entry should select");
    assert_eq!(selected.id, "first");
    assert_eq!(pool.current().map(|item| item.id), Some("first".into()));
    assert_eq!(
        pool.peek(1_700_000_000.0).map(|item| item.id),
        Some("first".into())
    );
}

// Tier: unit — mirrors tests/agent/test_credential_pool.py least_used cases.
#[test]
fn least_used_selection_picks_lowest_count_and_increments_usage() {
    let mut heavy = entry("heavy", "key-heavy", 0);
    heavy.request_count = 100;
    let mut light = entry("light", "key-light", 1);
    light.request_count = 10;
    let mut pool = CredentialPool::new("openrouter", vec![heavy, light], PoolStrategy::LeastUsed);

    let selected = pool.select(1_700_000_000.0).expect("entry should select");
    assert_eq!(selected.id, "light");
    assert_eq!(selected.request_count, 11);
}

// Tier: unit — mirrors agent/credential_pool.py round-robin priority rotation.
#[test]
fn round_robin_selection_moves_selected_entry_to_the_back() {
    let mut pool = CredentialPool::new(
        "openrouter",
        vec![entry("a", "key-a", 0), entry("b", "key-b", 1)],
        PoolStrategy::RoundRobin,
    );

    assert_eq!(pool.select(1_700_000_000.0).unwrap().id, "a");
    assert_eq!(pool.select(1_700_000_000.0).unwrap().id, "b");
    assert_eq!(pool.select(1_700_000_000.0).unwrap().id, "a");
}

// Tier: unit — mirrors tests/agent/test_credential_pool.py reset_at precedence.
#[test]
fn explicit_reset_timestamp_overrides_default_cooldown() {
    let mut pool = CredentialPool::new(
        "openai-codex",
        vec![entry("a", "key-a", 0)],
        PoolStrategy::FillFirst,
    );
    pool.select(1_700_000_000.0);
    let context = PoolErrorContext {
        reason: Some("rate_limit_exceeded".into()),
        message: None,
        reset_at: Some(1_700_000_900.0),
    };

    assert!(pool
        .mark_exhausted_and_rotate(Some(429), Some(&context), None, None, None, 1_700_000_000.0,)
        .is_none());
    assert!(!pool.has_available(1_700_000_899.0));
    assert!(pool.has_available(1_700_000_900.0));
    assert_eq!(
        pool.select(1_700_000_900.0).unwrap().status,
        Some(CredentialStatus::Ok)
    );
}

// Tier: unit — mirrors tests/agent/test_credential_pool.py token_invalidated.
#[test]
fn terminal_token_invalidated_marks_dead_but_rotates_to_healthy_entry() {
    let mut pool = CredentialPool::new(
        "openai-codex",
        vec![entry("dead", "revoked", 0), entry("healthy", "valid", 1)],
        PoolStrategy::FillFirst,
    );
    pool.select(1_700_000_000.0);
    let context = PoolErrorContext {
        reason: Some("token_invalidated".into()),
        message: Some("token has been invalidated".into()),
        reset_at: None,
    };

    let next = pool
        .mark_exhausted_and_rotate(Some(401), Some(&context), None, None, None, 1_700_000_000.0)
        .expect("healthy entry should rotate in");
    assert_eq!(next.id, "healthy");
    assert_eq!(pool.entries()[0].status, Some(CredentialStatus::Dead));
    assert_eq!(pool.select(1_700_000_000.0).unwrap().id, "healthy");
}

// Tier: unit — mirrors tests/agent/test_credential_pool.py unmatched hint guard.
#[test]
fn unmatched_failed_key_rotates_without_benching_healthy_entries() {
    let mut pool = CredentialPool::new(
        "anthropic",
        vec![entry("a", "key-a", 0), entry("b", "key-b", 1)],
        PoolStrategy::FillFirst,
    );

    let next = pool
        .mark_exhausted_and_rotate(
            Some(429),
            None,
            Some("rotated-away-key"),
            None,
            None,
            1_700_000_000.0,
        )
        .expect("unmatched identity should rotate to a candidate");
    assert_eq!(next.id, "a");
    assert!(pool
        .entries()
        .iter()
        .all(|item| item.status.is_none() || item.status == Some(CredentialStatus::Ok)));
}

// Tier: unit — mirrors tests/agent/test_credential_pool.py shared-key billing case.
#[test]
fn failed_key_marks_all_duplicate_entries_before_rotation() {
    let mut pool = CredentialPool::new(
        "custom",
        vec![
            entry("explicit", "shared-key", 0),
            entry("model", "shared-key", 1),
        ],
        PoolStrategy::FillFirst,
    );
    let next = pool.mark_exhausted_and_rotate(
        Some(402),
        None,
        Some("shared-key"),
        None,
        Some("billing"),
        1_700_000_000.0,
    );

    assert!(next.is_none());
    assert!(pool
        .entries()
        .iter()
        .all(|item| item.status == Some(CredentialStatus::Exhausted)));
}

// Tier: unit — mirrors tests/agent/test_credential_pool_sole_cooldown.py.
#[test]
fn sole_transient_credential_uses_short_cooldown_but_billing_keeps_long_ttl() {
    let mut transient = CredentialPool::new(
        "openrouter",
        vec![entry("transient", "key-a", 0)],
        PoolStrategy::FillFirst,
    );
    transient.mark_exhausted_and_rotate(Some(429), None, None, None, None, 1_700_000_000.0);
    assert!(transient.has_available(1_700_000_060.0));

    let mut billing = CredentialPool::new(
        "openrouter",
        vec![entry("billing", "key-b", 0)],
        PoolStrategy::FillFirst,
    );
    billing.mark_exhausted_and_rotate(
        Some(402),
        None,
        None,
        None,
        Some("billing"),
        1_700_000_000.0,
    );
    assert!(!billing.has_available(1_700_000_060.0));
}

fn jwt_with_claims(claims: Value) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
    format!("{header}.{payload}.signature")
}

// Tier: unit — mirrors PooledCredential.from_dict/to_dict field and extra
// handling in agent/credential_pool.py.
#[test]
fn pooled_credential_json_round_trip_preserves_runtime_metadata() {
    let payload = json!({
        "id": "oauth-1",
        "label": "primary",
        "auth_type": "oauth",
        "priority": 3,
        "source": "manual:device_code",
        "access_token": "access",
        "refresh_token": "refresh",
        "last_status": "exhausted",
        "last_status_at": "2026-08-24T00:00:00+00:00",
        "last_error_code": 429,
        "last_error_reason": "rate_limit",
        "last_error_message": "slow down",
        "last_error_reset_at": 1_700_000_900.0,
        "base_url": "https://api.example.test/v1",
        "expires_at": "2026-08-25T00:00:00+00:00",
        "expires_at_ms": 1_750_000_000_000i64,
        "last_refresh": "2026-08-23T00:00:00+00:00",
        "inference_base_url": "https://inference.example.test/v1",
        "agent_key": "agent-key",
        "agent_key_expires_at": "2026-08-25T00:00:00+00:00",
        "request_count": 7,
        "token_type": "Bearer",
        "scope": "inference:invoke",
        "obtained_at": "2026-08-23T00:00:00+00:00",
        "unrelated": "dropped by _EXTRA_KEYS",
    });

    let entry = PooledCredential::from_json("nous", &payload);
    assert_eq!(entry.id, "oauth-1");
    assert_eq!(entry.last_status.as_deref(), Some("exhausted"));
    assert_eq!(entry.status, Some(CredentialStatus::Exhausted));
    assert_eq!(entry.last_status_at, Some(1_787_529_600.0));
    assert_eq!(entry.expires_at_ms, Some(1_750_000_000_000));
    assert_eq!(entry.extra.get("scope"), Some(&json!("inference:invoke")));
    assert!(!entry.extra.contains_key("unrelated"));

    let serialized = entry.to_json();
    assert_eq!(serialized["last_status"], "exhausted");
    assert_eq!(serialized["last_status_at"], 1_787_529_600.0);
    assert_eq!(serialized["token_type"], "Bearer");
    assert_eq!(serialized["scope"], "inference:invoke");
    assert_eq!(serialized["request_count"], 7);
}

// Tier: unit — mirrors test_credential_pool_oat_authtype.py.
#[test]
fn anthropic_oat_tokens_are_oauth_but_real_api_keys_remain_api_keys() {
    let oauth = PooledCredential::from_json(
        "anthropic",
        &json!({"auth_type": "api_key", "access_token": "sk-ant-oat-legacy"}),
    );
    assert_eq!(oauth.auth_type, "oauth");

    let api_key = PooledCredential::from_json(
        "anthropic",
        &json!({"auth_type": "api_key", "access_token": "sk-ant-api-example"}),
    );
    assert_eq!(api_key.auth_type, "api_key");
}

// Tier: unit — mirrors credential_persistence.sanitize_borrowed_credential_payload.
#[test]
fn borrowed_credentials_strip_secret_fields_and_keep_metadata_fingerprint() {
    let sentinel = "S3NTINEL_DO_NOT_PERSIST";
    let mut credential = PooledCredential::new("openrouter", "borrowed-1", sentinel, 3);
    credential.label = "vault-ref".into();
    credential.source = "vault:openrouter/api-key".into();
    credential.refresh_token = Some(format!("refresh-{sentinel}"));
    credential.agent_key = Some(format!("agent-{sentinel}"));
    credential.request_count = 7;
    credential.last_status = Some("ok".into());
    credential
        .extra
        .insert("api_key".into(), json!(format!("extra-{sentinel}")));
    credential
        .extra
        .insert("token_type".into(), json!("Bearer"));
    credential.extra.insert("scope".into(), json!("inference"));

    let payload = credential.to_json();
    let serialized = serde_json::to_string(&payload).unwrap();
    assert!(!serialized.contains(sentinel));
    assert!(payload.get("access_token").is_none());
    assert!(payload.get("refresh_token").is_none());
    assert!(payload.get("agent_key").is_none());
    assert!(payload.get("api_key").is_none());
    assert_eq!(payload["source"], "vault:openrouter/api-key");
    assert_eq!(payload["label"], "vault-ref");
    assert_eq!(payload["request_count"], 7);
    assert_eq!(payload["token_type"], "Bearer");
    assert_eq!(payload["scope"], "inference");
    assert!(payload["secret_fingerprint"]
        .as_str()
        .is_some_and(|value| value.starts_with("sha256:")));
}

// Tier: unit — mirrors owned-source exceptions in credential_persistence.py.
#[test]
fn manual_and_provider_owned_device_code_credentials_keep_secret_fields() {
    let manual = PooledCredential::new("openrouter", "manual", "manual-secret", 0);
    assert_eq!(manual.to_json()["access_token"], "manual-secret");

    let device = PooledCredential::from_json(
        "nous",
        &json!({
            "id": "nous-device",
            "source": "device_code",
            "access_token": "access-secret",
            "refresh_token": "refresh-secret",
            "agent_key": "agent-secret",
        }),
    );
    let serialized = device.to_json();
    assert_eq!(serialized["access_token"], "access-secret");
    assert_eq!(serialized["refresh_token"], "refresh-secret");
    assert_eq!(serialized["agent_key"], "agent-secret");
}

// Tier: unit — mirrors PooledCredential.runtime_api_key and
// test_nous_runtime_api_key_rejects_opaque_agent_key.
#[test]
fn nous_runtime_api_key_requires_inference_jwt_and_prefers_agent_key() {
    let valid = jwt_with_claims(json!({
        "scope": ["inference:invoke"],
        "exp": 4_000_000_000i64,
    }));
    let entry = PooledCredential::from_json(
        "nous",
        &json!({
            "source": "device_code",
            "auth_type": "oauth",
            "access_token": "opaque-access",
            "agent_key": valid,
            "extra": {"scope": "inference:invoke"},
            "scope": "inference:invoke",
        }),
    );
    assert_eq!(entry.runtime_api_key(), entry.agent_key.as_deref().unwrap());

    let fallback_token = jwt_with_claims(json!({
        "scope": "inference:invoke",
        "exp": 4_000_000_000i64,
    }));
    let fallback = PooledCredential::from_json(
        "nous",
        &json!({
            "access_token": fallback_token,
            "agent_key": "opaque-agent-key",
            "scope": "inference:invoke",
        }),
    );
    assert_eq!(fallback.runtime_api_key(), fallback.access_token);

    let opaque = PooledCredential::from_json(
        "nous",
        &json!({
            "access_token": "opaque-access-token",
            "agent_key": "opaque-agent-key",
            "scope": "inference:invoke",
        }),
    );
    assert_eq!(opaque.runtime_api_key(), "");
}

// Tier: unit — mirrors label_from_token and runtime_base_url helpers.
#[test]
fn token_labels_and_provider_runtime_base_urls_follow_source_precedence() {
    let token = jwt_with_claims(json!({
        "email": "  user@example.test ",
        "preferred_username": "preferred",
    }));
    assert_eq!(label_from_token(&token, "fallback"), "user@example.test");
    assert_eq!(label_from_token("not-a-jwt", "fallback"), "fallback");

    let nous = PooledCredential::from_json(
        "nous",
        &json!({
            "base_url": "https://portal.example.test",
            "inference_base_url": "https://inference.example.test/v1",
        }),
    );
    assert_eq!(
        nous.runtime_base_url(),
        Some("https://inference.example.test/v1")
    );

    let other = PooledCredential::from_json(
        "openrouter",
        &json!({"base_url": "https://openrouter.example.test/v1"}),
    );
    assert_eq!(
        other.runtime_base_url(),
        Some("https://openrouter.example.test/v1")
    );
}

// Tier: unit — mirrors tests/agent/test_credential_pool_key_rotation.py.
#[test]
fn upsert_key_rotation_clears_stale_exhaustion_state() {
    let existing = PooledCredential::from_json(
        "openrouter",
        &json!({
            "id": "cred-1",
            "label": "OPENROUTER_API_KEY",
            "auth_type": "api_key",
            "priority": 0,
            "source": "env:OPENROUTER_API_KEY",
            "access_token": "old-key",
            "last_status": "exhausted",
            "last_status_at": 1000.0,
            "last_error_code": 429,
            "last_error_reason": "rate_limit",
            "last_error_message": "Too many requests",
            "last_error_reset_at": 2000.0,
        }),
    );
    let mut entries = vec![existing];
    let payload = json!({
        "source": "env:OPENROUTER_API_KEY",
        "auth_type": "api_key",
        "access_token": "new-rotated-key",
    });

    assert!(upsert_entry(
        &mut entries,
        "openrouter",
        "env:OPENROUTER_API_KEY",
        payload.as_object().unwrap(),
    ));
    assert_eq!(entries[0].access_token, "new-rotated-key");
    assert_eq!(entries[0].last_status, None);
    assert_eq!(entries[0].status, None);
    assert_eq!(entries[0].last_status_at, None);
    assert_eq!(entries[0].last_error_code, None);
    assert_eq!(entries[0].last_error_reason, None);
    assert_eq!(entries[0].last_error_message, None);
    assert_eq!(entries[0].last_error_reset_at, None);
}

// Tier: unit — mirrors tests/agent/test_credential_pool_key_rotation.py.
#[test]
fn upsert_same_key_preserves_exhaustion_state() {
    let existing = PooledCredential::from_json(
        "openrouter",
        &json!({
            "id": "cred-1",
            "label": "OPENROUTER_API_KEY",
            "auth_type": "api_key",
            "priority": 0,
            "source": "env:OPENROUTER_API_KEY",
            "access_token": "same-key",
            "last_status": "exhausted",
            "last_status_at": 1000.0,
            "last_error_code": 429,
            "last_error_reason": "rate_limit",
            "last_error_message": "Too many requests",
            "last_error_reset_at": 2000.0,
        }),
    );
    let mut entries = vec![existing];
    let payload = json!({
        "source": "env:OPENROUTER_API_KEY",
        "auth_type": "api_key",
        "access_token": "same-key",
    });

    assert!(!upsert_entry(
        &mut entries,
        "openrouter",
        "env:OPENROUTER_API_KEY",
        payload.as_object().unwrap(),
    ));
    assert_eq!(entries[0].last_status.as_deref(), Some("exhausted"));
    assert_eq!(entries[0].status, Some(CredentialStatus::Exhausted));
    assert_eq!(entries[0].last_error_reset_at, Some(2000.0));
}

// Tier: unit — mirrors agent/credential_pool.py _upsert_entry duplicate/source rules.
#[test]
fn upsert_collapses_duplicate_source_rows_and_keeps_first_identity() {
    let first = PooledCredential::from_json(
        "openrouter",
        &json!({
            "id": "first",
            "label": "OPENROUTER_API_KEY",
            "priority": 4,
            "source": "env:OPENROUTER_API_KEY",
            "access_token": "active",
        }),
    );
    let duplicate = PooledCredential::from_json(
        "openrouter",
        &json!({
            "id": "duplicate",
            "label": "stale",
            "priority": 9,
            "source": "env:OPENROUTER_API_KEY",
            "access_token": "stale",
        }),
    );
    let mut entries = vec![first, duplicate];
    let payload = json!({
        "source": "env:OPENROUTER_API_KEY",
        "access_token": "active",
    });

    assert!(upsert_entry(
        &mut entries,
        "openrouter",
        "env:OPENROUTER_API_KEY",
        payload.as_object().unwrap(),
    ));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "first");
    assert_eq!(entries[0].priority, 4);
}

// Tier: unit — mirrors agent/credential_pool.py _normalize_pool_priorities.
#[test]
fn anthropic_priority_normalization_keeps_manual_rows_before_seeded_order() {
    let make = |id: &str, source: &str, priority: i32, label: &str| {
        PooledCredential::from_json(
            "anthropic",
            &json!({
                "id": id,
                "source": source,
                "priority": priority,
                "label": label,
                "access_token": id,
            }),
        )
    };
    let mut entries = vec![
        make("claude", "claude_code", 0, "claude"),
        make("manual-b", "manual:second", 8, "manual-b"),
        make("api", "env:ANTHROPIC_API_KEY", 1, "api"),
        make("manual-a", "manual", 2, "manual-a"),
        make("token", "env:ANTHROPIC_TOKEN", 9, "token"),
    ];

    assert!(normalize_pool_priorities("anthropic", &mut entries));
    assert_eq!(
        entries
            .iter()
            .map(|entry| (entry.id.as_str(), entry.priority))
            .collect::<Vec<_>>(),
        vec![
            ("claude", 3),
            ("manual-b", 1),
            ("api", 4),
            ("manual-a", 0),
            ("token", 2),
        ]
    );
    assert!(!normalize_pool_priorities("openrouter", &mut entries));
}

// Tier: unit — mirrors agent/credential_pool.py get_pool_strategy.
#[test]
fn configured_pool_strategy_accepts_supported_values_and_fails_open() {
    let config = json!({
        "credential_pool_strategies": {
            "openrouter": "least_used",
            "anthropic": "ROUND_ROBIN",
            "bad": "not-a-strategy",
        }
    });
    let object = config.as_object().unwrap();
    assert_eq!(
        pool_strategy_from_config("openrouter", Some(object)),
        PoolStrategy::LeastUsed
    );
    assert_eq!(
        pool_strategy_from_config("anthropic", Some(object)),
        PoolStrategy::RoundRobin
    );
    let random_config = json!({
        "credential_pool_strategies": {"openrouter": "random"}
    });
    assert_eq!(
        pool_strategy_from_config("openrouter", random_config.as_object()),
        PoolStrategy::Random
    );
    assert_eq!(
        pool_strategy_from_config("bad", Some(object)),
        PoolStrategy::FillFirst
    );
    assert_eq!(
        pool_strategy_from_config("openrouter", None),
        PoolStrategy::FillFirst
    );
}

// Tier: unit — mirrors tests/agent/test_credential_pool_provider_boundary.py.
#[test]
fn custom_provider_pool_matching_is_scoped_by_normalized_endpoint() {
    let providers = vec![json!({
        "name": "Lab Provider",
        "base_url": "https://lab.example/v1/",
    })];
    assert_eq!(
        normalize_custom_pool_name("  Lab Provider "),
        "lab-provider"
    );
    assert!(credential_pool_matches_provider(
        Some("deepseek"),
        Some("deepseek"),
        None,
        &[],
    ));
    assert!(!credential_pool_matches_provider(
        Some("openai-codex"),
        Some("deepseek"),
        None,
        &[],
    ));
    assert!(!credential_pool_matches_provider(
        Some(""),
        Some("deepseek"),
        None,
        &[],
    ));
    assert!(credential_pool_matches_provider(
        Some("custom:lab-provider"),
        Some("custom"),
        Some(" https://lab.example/v1 "),
        &providers,
    ));
    assert!(!credential_pool_matches_provider(
        Some("custom:other"),
        Some("custom"),
        Some("https://lab.example/v1"),
        &providers,
    ));
}

// Tier: unit — mirrors agent/credential_pool.py custom-provider lookup helpers.
#[test]
fn custom_provider_lookup_prefers_name_and_lists_only_nonempty_pools() {
    let providers = vec![
        json!({
            "name": "First",
            "base_url": "https://shared.example/v1",
            "api_key": "first-key",
        }),
        json!({
            "name": "Second Provider",
            "base_url": "https://shared.example/v1/",
            "api_key": "second-key",
        }),
    ];
    assert_eq!(
        custom_provider_pool_key(
            Some("https://shared.example/v1"),
            Some("Second Provider"),
            &providers,
        ),
        Some("custom:second-provider".into())
    );
    assert_eq!(
        custom_provider_pool_key(Some("https://shared.example/v1/"), None, &providers),
        Some("custom:first".into())
    );
    assert_eq!(
        custom_provider_config("custom:second-provider", &providers).and_then(|entry| entry
            .get("api_key")
            .and_then(Value::as_str)
            .map(str::to_owned)),
        Some("second-key".into())
    );

    let pool_data = json!({
        "custom:first": [],
        "custom:second-provider": [{"id": "row"}],
        "openrouter": [{"id": "row"}],
    });
    assert_eq!(
        list_custom_pool_providers(pool_data.as_object().unwrap()),
        vec!["custom:second-provider"]
    );
}

// Tier: unit — mirrors hermes_cli.config.get_compatible_custom_providers and
// tests/hermes_cli/test_config.py::TestCustomProviderCompatibility::test_providers_dict_resolves_at_runtime.
#[test]
fn compatible_custom_providers_reads_keyed_provider_entries() {
    let config = json!({
        "providers": {
            "openai-direct": {
                "api": "https://api.openai.com/v1",
                "api_key": "test-key",
                "default_model": "gpt-5-mini",
                "name": "OpenAI Direct",
                "transport": "codex_responses"
            }
        }
    });

    let providers = get_compatible_custom_providers(config.as_object().unwrap());

    assert_eq!(providers.len(), 1);
    assert_eq!(
        providers[0],
        json!({
            "name": "OpenAI Direct",
            "base_url": "https://api.openai.com/v1",
            "provider_key": "openai-direct",
            "api_key": "test-key",
            "api_mode": "codex_responses",
            "model": "gpt-5-mini"
        })
    );
}

// Tier: unit — mirrors tests/hermes_cli/test_config.py::
// test_compatible_custom_providers_prefers_base_url_then_url_then_api.
#[test]
fn compatible_custom_providers_prefers_base_url_then_url_then_api() {
    let config = json!({
        "providers": {
            "my-provider": {
                "name": "My Provider",
                "api": "https://api.example.com/v1",
                "url": "https://url.example.com/v1",
                "base_url": "https://base.example.com/v1"
            }
        }
    });

    assert_eq!(
        get_compatible_custom_providers(config.as_object().unwrap()),
        vec![json!({
            "name": "My Provider",
            "base_url": "https://base.example.com/v1",
            "provider_key": "my-provider"
        })]
    );
}

// Tier: unit — mirrors tests/hermes_cli/test_custom_provider_normalize_no_mutate.py.
#[test]
fn custom_provider_compatibility_normalizes_without_mutating_input() {
    let config = json!({
        "custom_providers": [{
            "name": "Kimi Coding Plan",
            "base_url": "https://kimi.example/v1",
            "api_key_env": "KIMI_CODING_API_KEY",
            "api_mode": "anthropic_messages",
            "model": "kimi-k2.6",
            "models": {
                "kimi-k2.6": {"context_length": 262144}
            },
            "context_length": 262144,
            "rate_limit_delay": 0.25,
            "discover_models": false,
            "extra_body": {"chat_template_kwargs": {"enable_thinking": false}},
            "extra_headers": {"X-Int": 7, "X-None": null}
        }],
        "providers": {
            "disabled": {
                "base_url": "https://disabled.example/v1",
                "enabled": false
            }
        }
    });
    let snapshot = config.clone();

    let providers = get_compatible_custom_providers(config.as_object().unwrap());

    assert_eq!(config, snapshot);
    assert_eq!(providers.len(), 1);
    assert_eq!(
        providers[0],
        json!({
            "name": "Kimi Coding Plan",
            "base_url": "https://kimi.example/v1",
            "key_env": "KIMI_CODING_API_KEY",
            "api_mode": "anthropic_messages",
            "model": "kimi-k2.6",
            "models": {"kimi-k2.6": {"context_length": 262144}},
            "context_length": 262144,
            "rate_limit_delay": 0.25,
            "discover_models": false,
            "extra_body": {"chat_template_kwargs": {"enable_thinking": false}},
            "extra_headers": {"X-Int": "7"}
        })
    );

    let mut normalized = providers[0].as_object().unwrap().clone();
    normalized
        .get_mut("models")
        .and_then(Value::as_object_mut)
        .unwrap()
        .insert("injected".into(), json!({}));
    assert!(!config["custom_providers"][0]["models"]
        .as_object()
        .unwrap()
        .contains_key("injected"));
}

// Tier: unit — mirrors tests/hermes_cli/test_custom_provider_normalize_no_mutate.py
// and the config compatibility fail-open branch.
#[test]
fn custom_provider_compatibility_handles_aliases_invalid_entries_and_non_list_legacy_config() {
    let entry = json!({
        "baseUrl": "https://alias.example/v1",
        "apiKey": "sk-alias",
        "apiKeyEnv": "ALIAS_KEY",
        "defaultModel": "alias-model",
        "models": ["alpha", {"id": "beta", "context_length": 8192}, {"id": "", "name": "gamma"}],
        "enabled": true
    });
    let snapshot = entry.clone();
    let normalized =
        normalize_custom_provider_entry(entry.as_object().unwrap(), Some("alias")).unwrap();
    assert_eq!(entry, snapshot);
    assert_eq!(
        normalized,
        json!({
            "name": "alias",
            "base_url": "https://alias.example/v1",
            "provider_key": "alias",
            "api_key": "sk-alias",
            "key_env": "ALIAS_KEY",
            "model": "alias-model",
            "models": {
                "alpha": {},
                "beta": {"context_length": 8192},
                "gamma": {}
            }
        })
        .as_object()
        .unwrap()
        .clone()
    );

    let keyed = json!({
        "bad": "not-an-entry",
        "invalid-url": {"name": "Invalid", "api": "not-a-url"},
        "disabled": {"base_url": "https://disabled.example/v1", "enabled": false},
        "valid": {"base_url": "https://valid.example/v1"}
    });
    let converted = providers_dict_to_custom_providers(Some(keyed.as_object().unwrap()));
    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0]["provider_key"], "valid");

    let malformed = json!({"custom_providers": {"name": "wrong-shape"}});
    assert!(get_compatible_custom_providers(malformed.as_object().unwrap()).is_empty());
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::
// test_custom_endpoint_pool_seeds_from_config.
#[test]
fn custom_pool_seed_materializes_configured_api_key() {
    let providers = vec![json!({
        "name": "Together.ai",
        "base_url": "https://api.together.ai/v1/",
        "api_key": "sk-config-seeded"
    })];
    let mut entries = Vec::new();

    let result = seed_custom_pool(
        "custom:together.ai",
        &mut entries,
        &providers,
        None,
        &BTreeSet::new(),
    );

    assert!(result.changed);
    assert_eq!(
        result.active_sources,
        BTreeSet::from([String::from("config:Together.ai")])
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "config:Together.ai");
    assert_eq!(entries[0].access_token, "sk-config-seeded");
    assert_eq!(
        entries[0].base_url.as_deref(),
        Some("https://api.together.ai/v1")
    );
    assert_eq!(entries[0].label, "Together.ai");
}

// Tier: mock — mirrors tests/agent/test_credential_pool.py::
// test_custom_endpoint_pool_seeds_from_model_config.
#[test]
fn custom_pool_seed_materializes_matching_model_config_key() {
    let providers = vec![json!({
        "name": "Together.ai",
        "base_url": "https://api.together.ai/v1"
    })];
    let model = json!({
        "provider": "custom",
        "base_url": "https://api.together.ai/v1/",
        "api_key": "sk-model-key"
    });
    let model = model.as_object().unwrap().clone();
    let mut entries = Vec::new();

    let result = seed_custom_pool(
        "custom:together.ai",
        &mut entries,
        &providers,
        Some(&model),
        &BTreeSet::new(),
    );

    assert!(result.changed);
    assert_eq!(
        result.active_sources,
        BTreeSet::from([String::from("model_config")])
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "model_config");
    assert_eq!(entries[0].access_token, "sk-model-key");
    assert_eq!(
        entries[0].base_url.as_deref(),
        Some("https://api.together.ai/v1")
    );
    assert_eq!(entries[0].label, "model_config");
}
