use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hermes_agent::credential_pool::{
    label_from_token, CredentialPool, CredentialStatus, PoolErrorContext, PoolStrategy,
    PooledCredential,
};
use serde_json::{json, Value};

fn entry(id: &str, key: &str, priority: i32) -> PooledCredential {
    PooledCredential::new("openrouter", id, key, priority)
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
