//! Parity tests for `agent/secret_sources/command.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle. POSIX-only
//! helper tests run real `/bin/sh` children.

use serde_json::json;

use std::sync::Mutex;

use hermes_agent::secret_sources::base::{ErrorKind, SecretSource};
use hermes_agent::secret_sources::command::{
    get_command_secret, list_command_secrets, parse_secret_output, unquote_dotenv_value,
    CommandSource, COMMAND_TIMEOUT_SECONDS, MAX_OUTPUT_BYTES,
};

// Process-env mutations serialize (apply_* writes real env vars).
static ENV_GUARD: Mutex<()> = Mutex::new(());

// ── unquote_dotenv_value ─────────────────────────────────────────────────

#[test]
fn unquote_strips_one_layer_of_matching_quotes() {
    assert_eq!(unquote_dotenv_value("\"quoted\""), "quoted");
    assert_eq!(unquote_dotenv_value("'quoted'"), "quoted");
    // Requires length >= 2 so a lone quote is left intact.
    assert_eq!(unquote_dotenv_value("\""), "\"");
    // Empty quoted pair collapses to empty.
    assert_eq!(unquote_dotenv_value("\"\""), "");
    assert_eq!(unquote_dotenv_value("plain"), "plain");
    assert_eq!(unquote_dotenv_value("  padded  "), "padded");
}

// ── parse_secret_output ──────────────────────────────────────────────────

#[test]
fn exact_dotenv_match_wins() {
    let out = "OTHER=1\nAPI_KEY=real\n";
    assert_eq!(parse_secret_output(out, "API_KEY").as_deref(), Some("real"));
}

#[test]
fn multi_key_dump_without_wanted_key_is_none() {
    let out = "OTHER=1\nANOTHER=2\n";
    assert_eq!(parse_secret_output(out, "API_KEY"), None);
}

#[test]
fn bare_value_passes_through() {
    assert_eq!(
        parse_secret_output("just-a-secret", "API_KEY").as_deref(),
        Some("just-a-secret")
    );
}

#[test]
fn base64_padding_is_not_misrouted() {
    // A bare base64 secret with '=' padding matches the KEY=VALUE shape
    // but its "value" part is all '=' — treated as a bare value.
    assert_eq!(
        parse_secret_output("dGVzdA==", "API_KEY").as_deref(),
        Some("dGVzdA==")
    );
}

#[test]
fn cross_key_misroute_is_blocked() {
    // A single env-shaped line for a DIFFERENT key is never returned as
    // the wanted secret (cross-provider credential leakage, not just 401).
    assert_eq!(parse_secret_output("OTHER_KEY=realvalue", "API_KEY"), None);
}

#[test]
fn whitespace_placeholder_is_no_value() {
    // A quoted `K="  "` placeholder resolves to None.
    assert_eq!(parse_secret_output("API_KEY=\"  \"", "API_KEY"), None);
}

#[test]
fn comments_and_blank_lines_are_skipped() {
    let out = "# comment\n\nAPI_KEY=real\n";
    assert_eq!(parse_secret_output(out, "API_KEY").as_deref(), Some("real"));
}

// ── dotenv map parsing ───────────────────────────────────────────────────

#[test]
fn dotenv_map_parsing() {
    let blob = "# comment\nA=1\nB=spaced value\nBAD LINE\nC=\"quoted\"\n";
    let map = hermes_agent::secret_sources::command::parse_dotenv_map_public(blob);
    assert_eq!(
        map,
        vec![
            ("A".to_string(), "1".to_string()),
            ("B".to_string(), "spaced value".to_string()),
            ("C".to_string(), "quoted".to_string()),
        ]
    );
    // A bare-value helper yields {} — per-key resolution still works.
    assert!(hermes_agent::secret_sources::command::parse_dotenv_map_public("bare").is_empty());
}

// ── live helper runs (POSIX /bin/sh) ─────────────────────────────────────

#[test]
fn get_command_secret_resolves_via_env_key() {
    // The key travels ONLY via HERMES_SECRET_KEY — the command never
    // interpolates it.
    let got = get_command_secret(
        "printf '%s' \"bot-$HERMES_SECRET_KEY\" 2>/dev/null",
        "MY_KEY",
        COMMAND_TIMEOUT_SECONDS,
        MAX_OUTPUT_BYTES,
    );
    assert_eq!(got.as_deref(), Some("bot-MY_KEY"));
}

#[test]
fn list_command_secrets_enumerates_dotenv_blob() {
    let script = "printf 'A=1\\nB=two\\n'";
    let map = list_command_secrets(script, COMMAND_TIMEOUT_SECONDS, MAX_OUTPUT_BYTES);
    assert_eq!(
        map,
        vec![
            ("A".to_string(), "1".to_string()),
            ("B".to_string(), "two".to_string())
        ]
    );
}

#[test]
fn timeouts_and_empty_commands_degrade_to_none() {
    // A stalling helper hits the 3s hard timeout.
    let started = std::time::Instant::now();
    assert_eq!(
        get_command_secret("sleep 30", "K", 1.0, MAX_OUTPUT_BYTES),
        None
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(25));
    // Empty command degrades to None.
    assert_eq!(get_command_secret("", "K", 1.0, MAX_OUTPUT_BYTES), None);
    // Non-zero exit degrades to None.
    assert_eq!(
        get_command_secret("exit 3", "K", COMMAND_TIMEOUT_SECONDS, MAX_OUTPUT_BYTES),
        None
    );
}

#[test]
fn output_cap_degrades_to_none() {
    let script = "yes A_LOT_OF_OUTPUT | head -c 4096";
    assert_eq!(
        get_command_secret(script, "K", COMMAND_TIMEOUT_SECONDS, 16),
        None,
        "over-cap output resolves to no value"
    );
}

// ── CommandSource adapter ────────────────────────────────────────────────

#[test]
fn command_source_fetch_contract() {
    let source = CommandSource;
    assert_eq!(source.name(), "command");
    assert_eq!(source.shape(), "bulk");

    // Empty command -> NOT_CONFIGURED with the actionable message.
    let result = source.fetch(&json!({}), std::path::Path::new("/tmp"));
    assert_eq!(result.error_kind, Some(ErrorKind::NotConfigured));
    assert!(result
        .error
        .as_deref()
        .unwrap()
        .contains("secrets.command.command is empty"));

    // A live KEY=VALUE helper populates secrets.
    let cfg = json!({
        "command": "printf 'A=1\\nB=two\\n'",
        "helper_timeout_seconds": 5,
    });
    let result = source.fetch(&cfg, std::path::Path::new("/tmp"));
    assert!(result.error.is_none(), "{result:?}");
    assert!(result.secrets.contains_key("A"));
    assert_eq!(result.secrets.get("B").map(String::as_str), Some("two"));

    // Remediation hints.
    assert!(source
        .remediation(Some(ErrorKind::NotConfigured), &serde_json::json!({}))
        .unwrap()
        .contains("Set secrets.command.command"));
    assert!(source
        .remediation(Some(ErrorKind::Internal), &serde_json::json!({}))
        .unwrap()
        .contains("Run the helper manually"));
}

// ── 5d59366 fixes ──────────────────────────────────────────────────────

#[test]
fn stderr_does_not_pollute_parsed_secrets() {
    // A helper whose diagnostics go to stderr must not leak them into
    // the parsed secrets (separate buffers; stderr discarded).
    let cfg = serde_json::json!({
        "command": "printf 'A=1\n' >&2; printf 'B=two\n'",
        "helper_timeout_seconds": 5,
    });
    let result = CommandSource.fetch(&cfg, std::path::Path::new("/tmp"));
    assert!(result.error.is_none(), "{result:?}");
    assert_eq!(result.secrets.get("B").map(String::as_str), Some("two"));
    assert!(!result.secrets.contains_key("A"), "{result:?}");
}

#[test]
fn apply_command_secrets_applies_and_skips() {
    // PARITY: `apply_command_secrets` — empty command errors, guarded
    // keys skip, live values apply.
    use hermes_agent::secret_sources::command::apply_command_secrets;
    let empty = apply_command_secrets("", false, 3.0, 1024 * 1024);
    assert!(empty.error.is_some());
    let _guard = ENV_GUARD.lock();
    unsafe { std::env::set_var("APPLY_PROBE_A", "preexisting") };
    let result = apply_command_secrets(
        "printf 'APPLY_PROBE_A=new\nAPPLY_PROBE_B=  \nAPPLY_PROBE_C=v\n'",
        false,
        5.0,
        1024 * 1024,
    );
    // A keeps its env value (no override), whitespace-only B skips,
    // C applies.
    assert!(result.skipped.contains(&"APPLY_PROBE_A".to_string()));
    assert!(result.skipped.contains(&"APPLY_PROBE_B".to_string()));
    assert!(result.applied.contains(&"APPLY_PROBE_C".to_string()));
    assert_eq!(
        std::env::var("APPLY_PROBE_C").as_deref(),
        Ok("v"),
        "applied to the process env"
    );
    unsafe {
        std::env::remove_var("APPLY_PROBE_A");
        std::env::remove_var("APPLY_PROBE_C");
    }
}
