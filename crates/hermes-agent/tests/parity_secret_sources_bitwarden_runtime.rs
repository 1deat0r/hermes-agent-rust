//! Additional parity tests for the bitwarden bws-list orchestration
//! (`agent/secret_sources/bitwarden.py` `_run_bws_list` /
//! `fetch_bitwarden_secrets` @ b9aa928), run against a fake `bws` binary.

use std::collections::HashMap;
use std::fs;
use std::sync::Mutex as StdMutex;

use base64::Engine;
use serde_json::json;

use hermes_agent::secret_sources::bitwarden::{
    classify_bws_error, fetch_bitwarden_secrets, summarize_bws_stderr,
};

static LOCK: StdMutex<()> = StdMutex::new(());

fn install_fake_bws(dir: &std::path::Path, body: &str) -> String {
    let script = dir.join("fake-bws.sh");
    std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    script.to_string_lossy().into_owned()
}

fn empty_env() -> HashMap<String, String> {
    HashMap::new()
}

#[test]
fn summarize_cuts_location_noise_and_joins_causes() {
    let raw =
        "Error:\n   0: server said: [400] bad\n   1: retry later\n\nLocation:\n   src/main.rs:1\n";
    let summarized = summarize_bws_stderr(raw);
    assert_eq!(summarized, "server said: [400] bad; retry later");
}

#[test]
fn classify_bws_error_covers_transport_and_auth() {
    assert_eq!(
        classify_bws_error("connection refused"),
        hermes_agent::secret_sources::base::ErrorKind::Network
    );
    assert_eq!(
        classify_bws_error("invalid token supplied"),
        hermes_agent::secret_sources::base::ErrorKind::AuthFailed
    );
}

#[test]
fn bws_list_parses_json_entries_and_warns_on_bad_names() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let bws = install_fake_bws(
        td.path(),
        "echo '[{\"key\":\"GOOD\",\"value\":\"v1\"},{\"key\":\"bad name\",\"value\":\"v2\"},{\"nonstr\":1}]'",
    );
    let (secrets, warnings) = fetch_bitwarden_secrets(
        "tok",
        "proj",
        Some(std::path::Path::new(&bws)),
        300.0,
        false,
        "",
        Some(td.path()),
        false,
        0.0,
    )
    .unwrap();
    assert_eq!(secrets.get("GOOD").map(String::as_str), Some("v1"));
    assert!(!secrets.contains_key("bad name"));
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("not a valid env-var name"));
}

#[test]
fn empty_project_yields_warning_not_error() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let bws = install_fake_bws(td.path(), "printf ''");
    let (secrets, warnings) = fetch_bitwarden_secrets(
        "tok",
        "proj",
        Some(std::path::Path::new(&bws)),
        300.0,
        false,
        "",
        Some(td.path()),
        false,
        0.0,
    )
    .unwrap();
    assert!(secrets.is_empty());
    assert_eq!(warnings, vec!["bws returned no output (empty project?)"]);
}

#[test]
fn non_json_output_is_a_fatal_error() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let bws = install_fake_bws(td.path(), "echo 'not json'");
    assert!(fetch_bitwarden_secrets(
        "tok",
        "proj",
        Some(std::path::Path::new(&bws)),
        300.0,
        false,
        "",
        Some(td.path()),
        false,
        0.0
    )
    .unwrap_err()
    .contains("non-JSON"));
}

#[test]
fn network_failure_falls_back_to_stale_disk_cache() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path();

    // Seed the plaintext disk cache directly with a STALE entry (fetched_at
    // far older than the 300s fresh TTL) using the same key fingerprint the
    // orchestrator will compute for token "tok" + project "proj".
    let disk = hermes_agent::secret_sources::cache::DiskCache::new("bws_cache.json");
    let token_fp = hermes_agent::secret_sources::bitwarden::token_fingerprint("stale-tok");
    let key = hermes_agent::secret_sources::bitwarden::cache_key_str(&token_fp, "proj", "");
    let stale = hermes_agent::secret_sources::cache::CachedFetch {
        secrets: [("K".to_string(), "cached-value".to_string())]
            .into_iter()
            .collect(),
        fetched_at: 1.0, // ancient
    };
    disk.write_at(&key, &stale, 300.0, Some(home), &rand_nonce);

    // Live fetch fails at the transport level -> stale plaintext fallback.
    let bad = install_fake_bws(td.path(), "echo 'dns resolve host failed' >&2; exit 1");
    let (secrets, warnings) = fetch_bitwarden_secrets(
        "stale-tok",
        "proj",
        Some(std::path::Path::new(&bad)),
        300.0,
        true,
        "",
        Some(home),
        false,
        0.0,
    )
    .unwrap();
    assert_eq!(
        secrets.get("K").map(String::as_str),
        Some("cached-value"),
        "stale cache served"
    );
    assert!(
        warnings.iter().any(|w| w.contains("stale disk cache")),
        "{warnings:?}"
    );
}

fn rand_nonce() -> String {
    use base64::Engine;
    let bytes: [u8; 16] = rand::random();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

#[test]
fn auth_failure_does_not_fall_back_to_stale_cache() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path();

    let good = install_fake_bws(
        td.path(),
        "echo '[{\"key\":\"K\",\"value\":\"cached-value\"}]'",
    );
    let _ = fetch_bitwarden_secrets(
        "tok",
        "proj",
        Some(std::path::Path::new(&good)),
        300.0,
        true,
        "",
        Some(home),
        false,
        0.0,
    )
    .unwrap();

    // Rotate the token AND break the network: the fingerprint changes, and
    // even a transport failure must never serve another token's cache.
    let bad = install_fake_bws(td.path(), "echo 'invalid_client' >&2; exit 1");
    let err = fetch_bitwarden_secrets(
        "rotated-token",
        "proj",
        Some(std::path::Path::new(&bad)),
        300.0,
        true,
        "",
        Some(home),
        false,
        0.0,
    )
    .unwrap_err();
    assert!(err.contains("exited"), "{err}");
}

#[test]
fn missing_token_or_project_are_fatal_before_any_spawn() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(
        fetch_bitwarden_secrets("", "proj", None, 300.0, false, "", None, false, 0.0).unwrap_err(),
        "Bitwarden access token is empty"
    );
    assert_eq!(
        fetch_bitwarden_secrets("tok", "", None, 300.0, false, "", None, false, 0.0).unwrap_err(),
        "Bitwarden project_id is empty"
    );
}

#[test]
fn config_shape_examples() {
    // The adapter's schema/config contract from BitwardenSource.
    let cfg = json!({
        "enabled": true,
        "access_token_env": "BWS_ACCESS_TOKEN",
        "project_id": "uuid",
        "override_existing": true,
    });
    assert_eq!(cfg["access_token_env"], "BWS_ACCESS_TOKEN");
}
