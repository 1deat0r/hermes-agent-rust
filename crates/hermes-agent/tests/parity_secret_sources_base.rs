//! Parity tests for `agent/secret_sources/base.py` @ b9aa928.
//!
//! Upstream has no dedicated test file for the base contract (missing-test
//! gap, noted in the ledger); cases derive from the upstream code as
//! oracle. Env tests serialize behind a mutex per the workspace convention.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::json;

use hermes_agent::secret_sources::base::{
    get_source_env_var, is_valid_env_name, reset_source_environment, run_secret_cli, scrub_ansi,
    set_source_environment, ErrorKind, FetchResult, SecretSource, DEFAULT_CLI_TIMEOUT_SECONDS,
    DEFAULT_FETCH_TIMEOUT_SECONDS, SECRET_SOURCE_API_VERSION,
};

// ── env-name validation ──────────────────────────────────────────────────

#[test]
fn env_name_validation() {
    assert!(is_valid_env_name("OPENROUTER_API_KEY"));
    assert!(is_valid_env_name("_private"));
    assert!(is_valid_env_name("a1_b2"));
    assert!(!is_valid_env_name(""));
    assert!(!is_valid_env_name("1bad"));
    assert!(!is_valid_env_name("has-dash"));
    assert!(!is_valid_env_name("has space"));
}

// ── ANSI scrubbing ───────────────────────────────────────────────────────

#[test]
fn scrub_ansi_strips_whole_sequences_including_unterminated_osc() {
    // Whole CSI sequence.
    assert_eq!(scrub_ansi(Some("\u{1b}[31mred\u{1b}[0m")), "red");
    // Terminated OSC.
    assert_eq!(scrub_ansi(Some("\u{1b}]0;title\u{7}x")), "x");
    // Unterminated OSC (CLI killed mid-write) — this regex's specialty.
    assert_eq!(scrub_ansi(Some("\u{1b}]0;title")), "");
    // Plain text untouched.
    assert_eq!(scrub_ansi(Some("plain")), "plain");
    assert_eq!(scrub_ansi(None), "");
}

// ── per-fetch environment view ───────────────────────────────────────────

#[test]
fn source_environment_view_overrides_per_lookup() {
    let mut view = HashMap::new();
    view.insert("BW_SESSION".to_string(), "from-view".to_string());
    let token = set_source_environment(view);
    assert_eq!(
        get_source_env_var("BW_SESSION").as_deref(),
        Some("from-view")
    );
    reset_source_environment(token);
    // After reset the process environment answers again.
    assert_ne!(
        get_source_env_var("BW_SESSION").as_deref(),
        Some("from-view")
    );
}

// ── fetch timeout coercion ───────────────────────────────────────────────

#[test]
fn fetch_timeout_coercion_matches_python_try_float() {
    struct Source;
    impl SecretSource for Source {
        fn name(&self) -> &str {
            "test"
        }
        fn label(&self) -> &str {
            "Test"
        }
        fn fetch(&self, _cfg: &serde_json::Value, _home: &std::path::Path) -> FetchResult {
            FetchResult::default()
        }
    }
    let source = Source;
    // Missing -> default.
    assert_eq!(
        source.fetch_timeout_seconds(&json!({})),
        DEFAULT_FETCH_TIMEOUT_SECONDS
    );
    // Numeric values honored.
    assert_eq!(
        source.fetch_timeout_seconds(&json!({"timeout_seconds": 5})),
        5.0
    );
    // Non-numeric -> default.
    assert_eq!(
        source.fetch_timeout_seconds(&json!({"timeout_seconds": "soon"})),
        DEFAULT_FETCH_TIMEOUT_SECONDS
    );
    // Non-positive -> default.
    assert_eq!(
        source.fetch_timeout_seconds(&json!({"timeout_seconds": -1})),
        DEFAULT_FETCH_TIMEOUT_SECONDS
    );
    assert_eq!(DEFAULT_FETCH_TIMEOUT_SECONDS, 120.0);
    assert_eq!(DEFAULT_CLI_TIMEOUT_SECONDS, 30.0);
    assert_eq!(SECRET_SOURCE_API_VERSION, 1);
}

// ── remediation mapping ──────────────────────────────────────────────────

#[test]
fn remediation_maps_kinds_to_actionable_hints() {
    struct Source;
    impl SecretSource for Source {
        fn name(&self) -> &str {
            "bitwarden"
        }
        fn label(&self) -> &str {
            "Bitwarden"
        }
        fn fetch(&self, _: &serde_json::Value, _: &std::path::Path) -> FetchResult {
            FetchResult::default()
        }
    }
    let source = Source;
    assert_eq!(
        source
            .remediation(Some(ErrorKind::NotConfigured))
            .as_deref(),
        Some("Run `hermes secrets bitwarden setup` to finish configuration.")
    );
    assert_eq!(
        source
            .remediation(Some(ErrorKind::BinaryMissing))
            .as_deref(),
        Some("Run `hermes secrets bitwarden setup` to install the helper CLI.")
    );
    assert_eq!(
        source.remediation(Some(ErrorKind::Timeout)).as_deref(),
        Some("Backend was slow — raise secrets.bitwarden.timeout_seconds if this recurs.")
    );
    // No kind -> no hint.
    assert_eq!(source.remediation(None), None);
}

// ── run_secret_cli ───────────────────────────────────────────────────────

#[test]
fn run_secret_cli_allowlists_env_and_scrubs_stderr() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        std::env::set_var("SECRET_TEST_TOKEN", "tok-1");
        std::env::set_var("SECRET_TEST_SECRET", "sekret");
    }
    let allow = vec!["SECRET_TEST_TOKEN".to_string()];
    let argv = vec![
        "sh".to_string(),
        "-c".to_string(),
        "printf '%s' \"$SECRET_TEST_TOKEN\"; printf 'x' \"$SECRET_TEST_SECRET\" 1>&2; printf '\\033[31mred\\033[0m' 1>&2".to_string(),
    ];
    let result = run_secret_cli(&argv, &allow, None, DEFAULT_CLI_TIMEOUT_SECONDS).unwrap();
    // Allowed env var reached the child.
    assert!(result.stdout.contains("tok-1"), "{result:?}");
    // Non-allowlisted env var did not.
    assert!(!result.stdout.contains("sekret"));
    // stderr ANSI sequences are scrubbed.
    assert!(!result.stderr.contains('\u{1b}'), "{result:?}");
    unsafe {
        std::env::remove_var("SECRET_TEST_TOKEN");
        std::env::remove_var("SECRET_TEST_SECRET");
    }
}

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn run_secret_cli_timeout_yields_actionable_error() {
    let argv = vec!["sleep".to_string(), "30".to_string()];
    let err = run_secret_cli(&argv, &[], None, 1.0).unwrap_err();
    assert!(err.contains("timed out after 1s"), "{err}");
}

#[test]
fn run_secret_cli_missing_binary_error() {
    let argv = vec!["definitely-not-a-binary-xyz".to_string()];
    let err = run_secret_cli(&argv, &[], None, 1.0).unwrap_err();
    assert!(err.contains("failed to invoke"), "{err}");
}

#[test]
fn fetch_result_ok_semantics() {
    let mut result = FetchResult::default();
    assert!(result.ok());
    result.error = Some("bad".to_string());
    assert!(!result.ok());
}
