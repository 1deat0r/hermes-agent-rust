use hermes_agent::credential_pool::{
    CredentialPool, CredentialStatus, PoolErrorContext, PoolStrategy, PooledCredential,
};

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
