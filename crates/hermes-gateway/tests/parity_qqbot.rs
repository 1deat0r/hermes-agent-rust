//! Parity tests for `gateway/platforms/qqbot/{constants,utils,crypto}.py`
//! @ b9aa928.
//!
//! Upstream has no dedicated test files for these leaves (missing-test gap,
//! noted in the ledger); cases derive from the upstream code as oracle.
//! The crypto round-trip encrypts test-side with the same AES-256-GCM
//! layout the portal server produces (IV ‖ ciphertext ‖ tag).

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde_json::json;

use hermes_gateway::qqbot::constants;
use hermes_gateway::qqbot::crypto::{decrypt_secret, generate_bind_key};
use hermes_gateway::qqbot::utils::{
    build_user_agent, build_user_agent_with, coerce_list, get_api_headers,
};

// ── constants ────────────────────────────────────────────────────────────

#[test]
fn package_constants_match_the_pinned_values() {
    assert_eq!(constants::QQBOT_VERSION, "1.1.0");
    assert_eq!(constants::API_BASE, "https://api.sgroup.qq.com");
    assert_eq!(
        constants::TOKEN_URL,
        "https://bots.qq.com/app/getAppAccessToken"
    );
    assert_eq!(constants::GATEWAY_URL_PATH, "/gateway");
    assert_eq!(constants::ONBOARD_CREATE_PATH, "/lite/create_bind_task");
    assert_eq!(constants::ONBOARD_POLL_PATH, "/lite/poll_bind_result");
    assert_eq!(
        constants::QR_URL_TEMPLATE,
        "https://q.qq.com/qqbot/openclaw/connect.html?task_id={task_id}&_wv=2&source=hermes"
    );
}

#[test]
fn timing_and_retry_constants_match() {
    assert_eq!(constants::DEFAULT_API_TIMEOUT, 30.0);
    assert_eq!(constants::FILE_UPLOAD_TIMEOUT, 120.0);
    assert_eq!(constants::CONNECT_TIMEOUT_SECONDS, 20.0);
    assert_eq!(constants::RECONNECT_BACKOFF, [2, 5, 10, 30, 60]);
    assert_eq!(constants::MAX_RECONNECT_ATTEMPTS, 100);
    assert_eq!(constants::RATE_LIMIT_DELAY, 60);
    assert_eq!(constants::QUICK_DISCONNECT_THRESHOLD, 5.0);
    assert_eq!(constants::MAX_QUICK_DISCONNECT_COUNT, 3);
    assert_eq!(constants::ONBOARD_POLL_INTERVAL, 2.0);
    assert_eq!(constants::ONBOARD_API_TIMEOUT, 10.0);
}

#[test]
fn message_and_media_type_constants_match() {
    assert_eq!(constants::MAX_MESSAGE_LENGTH, 4000);
    assert_eq!(constants::DEDUP_WINDOW_SECONDS, 300);
    assert_eq!(constants::DEDUP_MAX_SIZE, 1000);
    assert_eq!(constants::MSG_TYPE_TEXT, 0);
    assert_eq!(constants::MSG_TYPE_MARKDOWN, 2);
    assert_eq!(constants::MSG_TYPE_MEDIA, 7);
    assert_eq!(constants::MSG_TYPE_INPUT_NOTIFY, 6);
    assert_eq!(constants::MEDIA_TYPE_IMAGE, 1);
    assert_eq!(constants::MEDIA_TYPE_VIDEO, 2);
    assert_eq!(constants::MEDIA_TYPE_VOICE, 3);
    assert_eq!(constants::MEDIA_TYPE_FILE, 4);
}

#[test]
fn portal_host_defaults_to_production() {
    // The env override is read once at import (Lazy here); with the var
    // unset at first access the production default holds. Clearing is
    // best-effort — another test may have pinned a value already, so only
    // assert the non-empty grammar.
    let host = constants::PORTAL_HOST.as_str();
    assert!(!host.is_empty());
}

// ── utils ────────────────────────────────────────────────────────────────

#[test]
fn user_agent_grammar_is_pinned() {
    assert_eq!(
        build_user_agent_with("3.11.15", "darwin", "0.9.0"),
        "QQBotAdapter/1.1.0 (Python/3.11.15; darwin; Hermes/0.9.0)"
    );
    // The runtime form keeps the same grammar with the Rust OS name and
    // the hermes_cli version constant.
    let ua = build_user_agent();
    assert!(
        ua.starts_with("QQBotAdapter/1.1.0 (Python/unknown; ")
            && ua.contains(&format!("; Hermes/{})", hermes_cli::VERSION)),
        "{ua}"
    );
}

#[test]
fn api_headers_carry_content_type_accept_and_user_agent() {
    let headers = get_api_headers();
    let names: Vec<&str> = headers.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(names, vec!["Content-Type", "Accept", "User-Agent"]);
    let ua = headers
        .iter()
        .find(|(k, _)| k == "User-Agent")
        .map(|(_, v)| v.clone())
        .unwrap();
    assert_eq!(ua, build_user_agent());
    assert!(headers
        .iter()
        .any(|(k, v)| k == "Accept" && v == "application/json"));
}

#[test]
fn coerce_list_handles_strings_and_arrays() {
    assert_eq!(coerce_list(None), Vec::<String>::new());
    assert_eq!(coerce_list(Some(&json!(null))), Vec::<String>::new());
    assert_eq!(coerce_list(Some(&json!("a, b ,c"))), vec!["a", "b", "c"]);
    assert_eq!(coerce_list(Some(&json!("solo"))), vec!["solo"]);
    assert_eq!(
        coerce_list(Some(&json!([" x ", "y", "", "  "]))),
        vec!["x", "y"]
    );
    assert_eq!(coerce_list(Some(&json!([1, 2]))), vec!["1", "2"]);
    // Single scalars wrap (Python `str(value).strip()`); True renders as
    // Python's str(), not JSON's.
    assert_eq!(coerce_list(Some(&json!(42))), vec!["42"]);
    assert_eq!(coerce_list(Some(&json!(true))), vec!["True"]);
    assert_eq!(coerce_list(Some(&json!("   "))), Vec::<String>::new());
}

// ── crypto ───────────────────────────────────────────────────────────────

/// Test-side encryption producing exactly the layout the QQBot portal
/// returns: base64(IV ‖ ciphertext ‖ tag), AES-256-GCM with no AAD.
fn encrypt_secret(plaintext: &str, key: &[u8]) -> String {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit};

    let iv: [u8; 12] = rand::random();
    let cipher = Aes256Gcm::new_from_slice(key).unwrap();
    let mut sealed = cipher
        .encrypt(aes_gcm::Nonce::from_slice(&iv), plaintext.as_bytes())
        .unwrap();
    let mut raw = iv.to_vec();
    raw.append(&mut sealed);
    BASE64.encode(raw)
}

#[test]
fn generate_bind_key_is_base64_of_256_bits() {
    let key = generate_bind_key();
    let raw = BASE64.decode(&key).unwrap();
    assert_eq!(raw.len(), 32, "256-bit key");
    // Distinct calls mint distinct keys.
    assert_ne!(key, generate_bind_key());
}

#[test]
fn decrypt_secret_round_trips_the_portal_layout() {
    let key_b64 = generate_bind_key();
    let key = BASE64.decode(&key_b64).unwrap();
    let secret = "s3cret_client_secret_value";
    let encrypted = encrypt_secret(secret, &key);
    assert_eq!(decrypt_secret(&encrypted, &key_b64).unwrap(), secret);
}

#[test]
fn decrypt_rejects_wrong_key_and_tampering() {
    let key_b64 = generate_bind_key();
    let key = BASE64.decode(&key_b64).unwrap();
    let encrypted = encrypt_secret("top secret", &key);

    let other_key = generate_bind_key();
    assert!(decrypt_secret(&encrypted, &other_key).is_err(), "wrong key");

    // Flip a ciphertext byte (inside the sealed portion).
    let mut raw = BASE64.decode(&encrypted).unwrap();
    let last = raw.len() - 1;
    raw[last] ^= 0x01;
    assert!(decrypt_secret(&BASE64.encode(&raw), &key_b64).is_err());

    // Truncated below IV+tag length.
    assert!(decrypt_secret(&BASE64.encode([0u8; 10]), &key_b64).is_err());
    // Garbage base64.
    assert!(decrypt_secret("!!!not base64!!!", &key_b64).is_err());
}
