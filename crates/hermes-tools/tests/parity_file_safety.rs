//! Parity oracles for the path-safety guards (pure-path cases), mirroring
//! upstream tests/agent/test_file_safety.py @ b9aa928. Env-mutating cases
//! (HERMES_HOME / HERMES_WRITE_SAFE_ROOT) live in the isolated
//! parity_file_safety_env.rs binary.

use hermes_tools::file_safety::{classify_write_denial, get_read_block_error, is_write_denied};

#[test]
fn blocked_env_basenames() {
    for basename in [
        ".env", ".env.local", ".env.development", ".env.production",
        ".env.test", ".env.staging", ".envrc",
    ] {
        let path = format!("/tmp/project/{basename}");
        let error = get_read_block_error(&path);
        assert!(error.is_some(), "{basename} should be blocked");
        let e = error.unwrap();
        assert!(e.contains("Access denied"));
        let lower = e.to_lowercase();
        assert!(
            lower.contains("secret-bearing") || lower.contains("environment file"),
            "message: {e}"
        );
    }
}

#[test]
fn blocked_env_basenames_case_insensitive() {
    for basename in [".ENV", ".Env.Local", ".ENV.PRODUCTION", ".ENVRC"] {
        let path = format!("/tmp/project/{basename}");
        let error = get_read_block_error(&path);
        assert!(error.is_some(), "{basename} should be blocked");
        let e = error.unwrap().to_lowercase();
        assert!(e.contains("environment file"), "message: {e}");
    }
}

#[test]
fn allowed_env_example() {
    // .env.example is documentation, not a secret.
    assert!(get_read_block_error("/tmp/project/.env.example").is_none());
}

#[test]
fn write_denies_credential_paths() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    assert_eq!(classify_write_denial(&format!("{home}/.ssh/id_rsa")), Some("credential"));
    assert_eq!(classify_write_denial(&format!("{home}/.ssh/config")), Some("credential"));
    assert_eq!(classify_write_denial("/etc/passwd"), Some("credential"));
    assert_eq!(classify_write_denial("/etc/sudoers"), Some("credential"));
    assert!(is_write_denied(&format!("{home}/.netrc")));
}

#[test]
fn write_allows_regular_paths() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    assert!(classify_write_denial(&format!("{home}/code/notes.txt")).is_none());
    assert!(classify_write_denial("/tmp/scratch/data.txt").is_none());
}
