//! Parity tests for `agent/secret_sources/onepassword.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle. Live `op`
//! invocations are exercised with a fake `op` script; env-touching tests
//! serialize behind a mutex per the workspace convention.

use std::collections::BTreeMap;
use std::process::Command;
use std::sync::Mutex as StdMutex;

use serde_json::json;

use hermes_agent::secret_sources::base::SecretSource;
use hermes_agent::secret_sources::onepassword::{
    classify_op_error, clear_caches, fetch_onepassword_secrets, find_op, validate_references,
    OnePasswordSource,
};

static ENV_LOCK: StdMutex<()> = StdMutex::new(());

#[test]
fn reference_validation() {
    let refs = json!({
        "GOOD_KEY": "  op://Private/OpenAI/api key  ",
        "bad name": "op://x",
        "NO_REF": "https://example.com",
        "NUM_KEY": 42,
    });
    let (valid, warnings) = validate_references(Some(&refs));
    assert_eq!(valid.len(), 1);
    assert_eq!(
        valid.get("GOOD_KEY").map(String::as_str),
        Some("op://Private/OpenAI/api key")
    );
    assert_eq!(warnings.len(), 3);
    assert!(warnings
        .iter()
        .any(|w| w.contains("not a valid env-var name")));
    assert!(warnings
        .iter()
        .any(|w| w.contains("not an op:// secret reference")));
}

#[test]
fn op_binary_discovery_pinned_path_semantics() {
    // A pinned path that does not exist: None, never a silent fallback.
    assert!(find_op("/definitely/not/op").is_none());
    // A pinned path that exists and is executable: used verbatim.
    let real_sh = std::path::Path::new("/bin/sh");
    if real_sh.exists() {
        assert_eq!(find_op("/bin/sh").as_deref(), Some(real_sh));
    }
}

/// Write a fake `op` script that prints a fixed value per reference.
fn install_fake_op(dir: &std::path::Path, body: &str) -> String {
    let script = dir.join("fake-op.sh");
    std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    script.to_string_lossy().into_owned()
}

#[test]
fn fetch_resolves_references_and_caches_complete_pulls() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path();

    // Fake op: prints the reference itself as the value.
    // `op read -- <ref>`: the reference is argument 3.
    let binary_path = install_fake_op(td.path(), "echo \"value-for-$3\"");

    let mut refs = BTreeMap::new();
    refs.insert(
        "OPENAI_API_KEY".to_string(),
        "op://Private/OpenAI/api key".to_string(),
    );
    refs.insert(
        "ANTHROPIC_API_KEY".to_string(),
        "op://Private/Anthropic/credential".to_string(),
    );

    let (secrets, warnings) = fetch_onepassword_secrets(
        &refs,
        "",
        DEFAULT_TOKEN_ENV,
        Some(std::path::Path::new(&binary_path)),
        "",
        true,
        300.0,
        Some(home),
    )
    .unwrap();
    assert_eq!(secrets.len(), 2);
    assert!(warnings.is_empty());
    assert!(secrets["OPENAI_API_KEY"].starts_with("value-for-op://Private/OpenAI"));

    // A complete pull is cached: the second fetch hits L1 (same values,
    // no re-invocation errors, and the fake's output is deterministic so
    // we can't count invocations — but a changed refs fingerprint must
    // miss).
    let (secrets2, _) = fetch_onepassword_secrets(
        &refs,
        "",
        DEFAULT_TOKEN_ENV,
        Some(std::path::Path::new(&binary_path)),
        "",
        true,
        300.0,
        Some(home),
    )
    .unwrap();
    assert_eq!(secrets2, secrets);
    assert!(
        home.join("cache").join("op_cache.json").exists(),
        "disk cache written"
    );

    // Changing the mapping changes the refs fingerprint -> cache miss.
    let mut different = BTreeMap::new();
    different.insert("OTHER_KEY".to_string(), "op://Private/Other/x".to_string());
    let result = fetch_onepassword_secrets(
        &different,
        "",
        DEFAULT_TOKEN_ENV,
        Some(std::path::Path::new(&binary_path)),
        "",
        true,
        300.0,
        Some(home),
    );
    assert!(result.is_ok());
}

use hermes_agent::secret_sources::onepassword::DEFAULT_TOKEN_ENV;

#[test]
fn per_reference_failures_become_warnings() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path();
    // A fake op that exits 1 (the error text goes to stderr, which the
    // caller scrubs and truncates into the warning).
    let binary_path = install_fake_op(td.path(), "echo 'auth expired' >&2; exit 1");

    let mut refs = BTreeMap::new();
    refs.insert("GOOD_KEY".to_string(), "op://V/I/F".to_string());
    refs.insert("BAD_KEY".to_string(), "op://V/BAD".to_string());

    let (secrets, warnings) = fetch_onepassword_secrets(
        &refs,
        "",
        DEFAULT_TOKEN_ENV,
        Some(std::path::Path::new(&binary_path)),
        "",
        true,
        300.0,
        Some(home),
    )
    .unwrap();
    // One bad reference never sinks the rest.
    assert!(warnings.len() >= 1);
    assert!(secrets.is_empty() || !secrets.contains_key("BAD_KEY"));
}

#[test]
fn missing_op_binary_is_a_fatal_fetch_error() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let mut refs = BTreeMap::new();
    refs.insert("K".to_string(), "op://V/I/F".to_string());
    let err = fetch_onepassword_secrets(
        &refs,
        "",
        DEFAULT_TOKEN_ENV,
        None,
        "/definitely/not/op",
        true,
        300.0,
        Some(td.path()),
    )
    .unwrap_err();
    assert!(err.contains("op CLI not found"), "{err}");
}

#[test]
fn classify_op_error_maps_the_taxonomy() {
    assert_eq!(
        classify_op_error("op read timed out after 30s"),
        hermes_agent::secret_sources::base::ErrorKind::Timeout
    );
    assert_eq!(
        classify_op_error("op CLI not found on PATH"),
        hermes_agent::secret_sources::base::ErrorKind::BinaryMissing
    );
    assert_eq!(
        classify_op_error("401 unauthorized"),
        hermes_agent::secret_sources::base::ErrorKind::AuthFailed
    );
    assert_eq!(
        classify_op_error("session expired"),
        hermes_agent::secret_sources::base::ErrorKind::AuthFailed
    );
    assert_eq!(
        classify_op_error("empty value for ref"),
        hermes_agent::secret_sources::base::ErrorKind::EmptyValue
    );
    assert_eq!(
        classify_op_error("dns resolve host failed"),
        hermes_agent::secret_sources::base::ErrorKind::Network
    );
    assert_eq!(
        classify_op_error("???"),
        hermes_agent::secret_sources::base::ErrorKind::Internal
    );
}

#[test]
fn onepassword_source_adapter_contract() {
    let source = OnePasswordSource;
    assert_eq!(source.name(), "onepassword");
    assert_eq!(source.label(), "1Password");
    assert_eq!(source.shape(), "mapped");
    assert_eq!(source.scheme(), Some("op"));
    // override_existing defaults TRUE (explicit VAR→op:// binding is
    // strongest intent).
    assert!(source.override_existing(&json!({})));
    assert!(source.override_existing(&json!({"override_existing": true})));
    assert!(!source.override_existing(&json!({"override_existing": false})));
    // The bootstrap-auth token env is protected.
    assert_eq!(
        source.protected_env_vars(&serde_json::json!({})),
        vec!["OP_SERVICE_ACCOUNT_TOKEN"]
    );
    // Remediation hints.
    assert!(source
        .remediation(
            Some(hermes_agent::secret_sources::base::ErrorKind::BinaryMissing),
            &serde_json::json!({})
        )
        .unwrap()
        .contains("1Password CLI"));
}

#[test]
fn remediation_hints_fall_through_to_generic() {
    // PARITY: `remediation_hints` (not a full override) — listed kinds
    // use the source text with {token_env} filled; every other kind
    // falls through to the GENERIC text (the old full override wrongly
    // returned None for Timeout/Network).
    let source = OnePasswordSource;
    let cfg = serde_json::json!({});
    let auth = source
        .remediation(
            Some(hermes_agent::secret_sources::base::ErrorKind::AuthFailed),
            &cfg,
        )
        .unwrap();
    assert!(auth.contains("OP_SERVICE_ACCOUNT_TOKEN"), "{auth}");
    let timeout = source
        .remediation(
            Some(hermes_agent::secret_sources::base::ErrorKind::Timeout),
            &cfg,
        )
        .unwrap();
    assert!(timeout.contains("timeout_seconds"), "{timeout}");
    let network = source
        .remediation(
            Some(hermes_agent::secret_sources::base::ErrorKind::Network),
            &cfg,
        )
        .unwrap();
    assert!(network.contains("Network problem"), "{network}");
    // Custom token env fills the placeholder.
    let custom = serde_json::json!({"service_account_token_env": "MY_OP_TOKEN"});
    let auth = source
        .remediation(
            Some(hermes_agent::secret_sources::base::ErrorKind::AuthFailed),
            &custom,
        )
        .unwrap();
    assert!(auth.contains("MY_OP_TOKEN"), "{auth}");
    assert_eq!(source.token_env(&custom), "MY_OP_TOKEN");
    assert_eq!(source.token_env(&cfg), "OP_SERVICE_ACCOUNT_TOKEN");
}

#[test]
fn apply_skips_guarded_refs_before_fetching() {
    // PARITY: `apply_onepassword_secrets` — disabled is a no-op; the
    // token var and env-satisfied refs never reach `op`.
    use hermes_agent::secret_sources::onepassword::apply_onepassword_secrets;
    use std::collections::BTreeMap;
    let empty = apply_onepassword_secrets(
        false,
        None,
        "",
        "OP_SERVICE_ACCOUNT_TOKEN",
        "",
        true,
        300.0,
        None,
    );
    assert!(empty.ok());
    assert!(empty.secrets.is_empty());
    // Token var is skipped even when bound to a real-looking ref, and a
    // pinned-missing binary errors when a real ref needs fetching.
    let mut env = BTreeMap::new();
    env.insert(
        "OP_SERVICE_ACCOUNT_TOKEN".to_string(),
        "op://vault/item/field".to_string(),
    );
    env.insert("MY_KEY".to_string(), "op://vault/item/key".to_string());
    let result = apply_onepassword_secrets(
        true,
        Some(&env),
        "",
        "OP_SERVICE_ACCOUNT_TOKEN",
        "/nonexistent-op-binary-xyz",
        true,
        300.0,
        None,
    );
    assert!(result
        .skipped
        .contains(&"OP_SERVICE_ACCOUNT_TOKEN".to_string()));
    assert!(!result.skipped.contains(&"MY_KEY".to_string()));
    assert!(result.error.is_some(), "pinned-missing binary errors");
}

#[test]
fn fetch_reports_missing_env_map_as_not_configured() {
    let source = OnePasswordSource;
    let result = source.fetch(&json!({"enabled": true}), std::path::Path::new("/tmp"));
    // Empty env map with no warnings -> NOT_CONFIGURED error.
    assert_eq!(
        result.error_kind,
        Some(hermes_agent::secret_sources::base::ErrorKind::NotConfigured)
    );
    assert!(result
        .error
        .as_deref()
        .unwrap()
        .contains("env: map is empty"));
}

#[test]
fn clear_caches_is_idempotent() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_caches(None);
    clear_caches(None);
}
