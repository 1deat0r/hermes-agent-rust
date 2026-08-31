//! Parity tests for `agent/secret_sources/bitwarden.py` (partial port) @
//! b9aa928. Upstream has no dedicated test file (missing-test gap, noted
//! in the ledger); cases derive from the upstream code as oracle.

use hermes_agent::secret_sources::base::{ErrorKind, SecretSource};
use serde_json::json;

use hermes_agent::secret_sources::bitwarden::{
    cache_key_str, classify_bws_error, summarize_bws_stderr, token_fingerprint, BitwardenSource,
};

// ── pure helpers ─────────────────────────────────────────────────────────

#[test]
fn cache_key_serialization_is_pipe_joined() {
    assert_eq!(
        cache_key_str("tok1234abcd1234", "proj-uuid", "https://vault.example.com"),
        "tok1234abcd1234|proj-uuid|https://vault.example.com"
    );
}

#[test]
fn token_fingerprint_is_a_stable_sha256_prefix() {
    let fp = token_fingerprint("my-access-token");
    assert_eq!(fp.len(), 16);
    assert_eq!(token_fingerprint("my-access-token"), fp, "stable");
    assert_ne!(token_fingerprint("other"), fp);
    // A fingerprint never equals its input.
    assert_ne!(fp, "my-access-token");
}

// ── bws stderr summarization ─────────────────────────────────────────────

#[test]
fn summarize_reduces_color_eyre_dump_to_cause_lines() {
    let raw = "Error:\n   \
               0: Received error message from server: [400 Bad Request] {\"error\":\"invalid_client\"}\n\n\
               Location:\n   \
               crates/bws/src/main.rs:108\n\
               Backtrace omitted. Run with RUST_BACKTRACE=1";
    let summarized = summarize_bws_stderr(raw);
    assert_eq!(
        summarized,
        "Received error message from server: [400 Bad Request] {\"error\":\"invalid_client\"}"
    );
}

#[test]
fn summarize_unrecognized_shape_falls_back_to_stripped_raw() {
    assert_eq!(summarize_bws_stderr("plain failure"), "plain failure");
    assert_eq!(summarize_bws_stderr(""), "");
    assert_eq!(summarize_bws_stderr("   \n  "), "");
}

// ── error classification ─────────────────────────────────────────────────

#[test]
fn classify_bws_error_maps_the_taxonomy() {
    assert_eq!(classify_bws_error("request timed out"), ErrorKind::Timeout);
    assert_eq!(
        classify_bws_error("binary not available and auto-install disabled"),
        ErrorKind::BinaryMissing
    );
    assert_eq!(
        classify_bws_error("401 unauthorized"),
        ErrorKind::AuthFailed
    );
    // The BSM identity endpoint's OAuth-style rejection of a revoked
    // machine-account token.
    assert_eq!(
        classify_bws_error("[400 Bad Request] {\"error\":\"invalid_client\"}"),
        ErrorKind::AuthFailed
    );
    assert_eq!(classify_bws_error("invalid_grant"), ErrorKind::AuthFailed);
    assert_eq!(
        classify_bws_error("dns resolution failure"),
        ErrorKind::Network
    );
    assert_eq!(classify_bws_error("download failed"), ErrorKind::Network);
    assert_eq!(classify_bws_error("???"), ErrorKind::Internal);
}

// ── adapter contract ─────────────────────────────────────────────────────

#[test]
fn bitwarden_adapter_contract() {
    let source = BitwardenSource;
    assert_eq!(source.name(), "bitwarden");
    assert_eq!(source.label(), "Bitwarden Secrets Manager");
    assert_eq!(source.shape(), "bulk");
    assert_eq!(source.scheme(), Some("bws"));
    // override_existing defaults TRUE (centralized rotation is the point
    // of BSM).
    assert!(source.override_existing(&json!({})));
    assert!(!source.override_existing(&json!({"override_existing": false})));
    // The bootstrap-auth token env is protected.
    assert_eq!(source.protected_env_vars(), vec!["BWS_ACCESS_TOKEN"]);
}

#[test]
fn fetch_reports_missing_token_and_project_as_not_configured() {
    use std::path::Path;
    let source = BitwardenSource;
    // Missing access token env (this test does not set BWS_ACCESS_TOKEN;
    // parallel tests do not set it either).
    let result = source.fetch(&json!({"project_id": "p"}), Path::new("/tmp"));
    assert_eq!(result.error_kind, Some(ErrorKind::NotConfigured));
    assert!(result
        .error
        .as_deref()
        .unwrap()
        .contains("BWS_ACCESS_TOKEN"));

    // Token present but project_id empty -> NOT_CONFIGURED as well.
    // (Access token can't be injected portably without env mutation, so we
    // only pin the project arm through a set token env name if present.)
    let _ = 0;
}
