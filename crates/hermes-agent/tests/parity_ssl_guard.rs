//! Parity tests for `agent/ssl_guard.py` @ b9aa928, mirroring
//! `tests/agent/test_ssl_ca_guard.py` (the certifi leg is
//! parameterised — no certifi in Rust; the caller supplies the platform
//! bundle path). Env tests serialize behind a mutex per the workspace
//! convention.

use std::fs;
use std::sync::Mutex;

use hermes_agent::ssl_guard::{verify_ca_bundle, CA_BUNDLE_ENV_VARS};

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// A plausible PEM bundle: > 1024 bytes, one certificate block.
const GOOD_PEM: &str = "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n";

fn clear_env() {
    for key in CA_BUNDLE_ENV_VARS {
        unsafe { std::env::remove_var(key) };
    }
    unsafe { std::env::remove_var("HERMES_SKIP_SSL_GUARD") };
}

#[test]
fn healthy_bundle_passes() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_env();
    let td = tempfile::TempDir::new().unwrap();
    let bundle = td.path().join("cacert.pem");
    // > 1024 bytes with a certificate block — the upstream healthy case.
    let filler = "x".repeat(1100);
    fs::write(&bundle, format!("{GOOD_PEM}{filler}")).unwrap();
    verify_ca_bundle(Some(&bundle))
        .map_err(|e| e.message.clone())
        .unwrap();
}

#[test]
fn empty_bundle_raises_too_small() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_env();
    let td = tempfile::TempDir::new().unwrap();
    let bundle = td.path().join("empty.pem");
    fs::write(&bundle, b"").unwrap();
    let err = verify_ca_bundle(Some(&bundle)).unwrap_err();
    assert!(err.message.to_lowercase().contains("too small"), "{err}");
}

#[test]
fn missing_explicit_ca_bundle_env_raises_before_client_init() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_env();
    let td = tempfile::TempDir::new().unwrap();
    for env_var in CA_BUNDLE_ENV_VARS {
        let fake = td.path().join("missing.pem");
        unsafe { std::env::set_var(env_var, &fake) };
        let err = verify_ca_bundle(None).unwrap_err();
        let message = err.message;
        assert!(message.contains(env_var), "{env_var}: {message}");
        assert!(message.contains("missing.pem"), "{message}");
        assert!(message.contains("force-reinstall"), "{message}");
        unsafe { std::env::remove_var(env_var) };
    }
}

#[test]
fn directory_and_certificate_free_bundle_raise() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_env();
    let td = tempfile::TempDir::new().unwrap();
    // A directory: exists but is not a file.
    let dir = td.path().join("bundle-dir");
    fs::create_dir_all(&dir).unwrap();
    unsafe { std::env::set_var("SSL_CERT_FILE", &dir) };
    let err = verify_ca_bundle(None).unwrap_err();
    assert!(
        err.message.contains("does not point to a CA bundle file"),
        "{err}"
    );
    unsafe { std::env::remove_var("SSL_CERT_FILE") };

    // A big-enough file with zero certificate blocks: "did not load any
    // certificates".
    let no_certs = td.path().join("no-certs.pem");
    fs::write(&no_certs, "x".repeat(1100)).unwrap();
    unsafe { std::env::set_var("SSL_CERT_FILE", &no_certs) };
    let err = verify_ca_bundle(None).unwrap_err();
    assert!(
        err.message.contains("did not load any certificates"),
        "{err}"
    );
    unsafe { std::env::remove_var("SSL_CERT_FILE") };
}

#[test]
fn skip_guard_env_short_circuits_all_checks() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_env();
    unsafe { std::env::set_var("HERMES_SKIP_SSL_GUARD", "1") };
    // Even a missing platform bundle is tolerated when the guard is skipped.
    assert!(verify_ca_bundle(None).is_ok());
    unsafe { std::env::remove_var("HERMES_SKIP_SSL_GUARD") };
}

#[test]
fn error_message_carries_the_repair_hint() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_env();
    let td = tempfile::TempDir::new().unwrap();
    let missing = td.path().join("gone.pem");
    unsafe { std::env::set_var("HERMES_CA_BUNDLE", &missing) };
    let err = verify_ca_bundle(None).unwrap_err();
    assert!(err.message.contains("hermes doctor --fix"), "{err}");
    unsafe { std::env::remove_var("HERMES_CA_BUNDLE") };
}

#[test]
fn expanduser_home_prefix_is_expanded() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_env();
    let td = tempfile::TempDir::new().unwrap();
    let bundle = td.path().join("ca.pem");
    // The platform leg requires a substantial bundle (> 1024 bytes).
    fs::write(&bundle, format!("{GOOD_PEM}{}", "x".repeat(1100))).unwrap();
    unsafe { std::env::set_var("HOME", td.path()) };
    unsafe { std::env::set_var("SSL_CERT_FILE", "~/ca.pem") };
    // ~/ca.pem resolves inside HOME, so validation passes (the platform
    // bundle leg is satisfied by the same file).
    verify_ca_bundle(Some(&bundle))
        .map_err(|e| e.message.clone())
        .unwrap();
    unsafe { std::env::remove_var("SSL_CERT_FILE") };
    unsafe { std::env::remove_var("HOME") };
}
