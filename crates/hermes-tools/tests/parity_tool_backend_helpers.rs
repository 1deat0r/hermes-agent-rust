//! Parity oracles for tool backend selection helpers, mirroring upstream
//! tests/tools/test_tool_backend_helpers.py @ b9aa928.
//!
//! Evidence tiers:
//! - unit: pure coercion / selection helpers (normalize_browser_cloud_provider,
//!   coerce_modal_mode, normalize_modal_mode, resolve_modal_backend_state matrix).
//! - mock: subsystem seams stand in for the upstream monkeypatches of
//!   hermes_cli.nous_account / hermes_cli.config / agent.secret_scope
//!   (managed_nous_tools_enabled override, prefers_gateway config override,
//!   env-getter injection).
//!
//! Deferred seams (unported subsystems — see the module docs):
//!   full Nous-portal entitlement message formatting, profile secret-scope
//!   multiplexing, and the credential pool fallback.

use std::sync::{Mutex, MutexGuard};

use hermes_tools::tool_backend_helpers::{
    coerce_modal_mode, fal_key_is_configured, has_direct_modal_credentials,
    managed_nous_tools_enabled, normalize_browser_cloud_provider, normalize_modal_mode,
    nous_tool_gateway_unavailable_message, prefers_gateway, resolve_modal_backend_state,
    resolve_openai_audio_api_key, resolve_provider_secret, set_load_config_for_test,
    set_nous_entitlement_for_test,
};
use serde_json::{json, Value};

fn tmp_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "hbh_{label}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// Env vars are process-global: serialize the env-mutating tests.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Holds the env lock for its lifetime and restores every touched variable
/// on drop.
struct EnvGuard {
    _guard: MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn lock() -> Self {
        EnvGuard {
            _guard: ENV_LOCK.lock().unwrap(),
            saved: Vec::new(),
        }
    }
    fn set(mut self, key: &'static str, value: &str) -> Self {
        let old = std::env::var(key).ok();
        std::env::set_var(key, value);
        self.saved.push((key, old));
        self
    }
    fn unset(mut self, key: &'static str) -> Self {
        let old = std::env::var(key).ok();
        std::env::remove_var(key);
        self.saved.push((key, old));
        self
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}

// ── managed_nous_tools_enabled ────────────────────────────────────────────

#[test]
fn managed_nous_tools_disabled_when_subsystem_unavailable() {
    // The hermes_cli.nous_account subsystem is unported, so the ruff path
    // fails closed exactly like upstream's `except Exception: return False`
    // (mirrors test_disabled_when_not_logged_in / test_returns_false_on_exception).
    set_nous_entitlement_for_test(None);
    assert!(!managed_nous_tools_enabled(false));
    assert!(!managed_nous_tools_enabled(true)); // force_fresh ignored when no seam
}

#[test]
fn managed_nous_tools_entitlement_override() {
    // Mock tier: the override stands in for monkeypatching
    // hermes_cli.nous_account.get_nous_portal_account_info.
    set_nous_entitlement_for_test(Some(true));
    assert!(managed_nous_tools_enabled(false));
    assert!(managed_nous_tools_enabled(true));
    set_nous_entitlement_for_test(Some(false));
    assert!(!managed_nous_tools_enabled(false));
    set_nous_entitlement_for_test(None);
}

// ── nous_tool_gateway_unavailable_message ─────────────────────────────────

#[test]
fn unavailable_message_falls_back_to_plain_guidance() {
    // Mock tier: the Nous-portal entitlement formatter is a deferred seam;
    // the fallback matches upstream's return when the formatter yields no
    // account-aware message.
    set_nous_entitlement_for_test(None);
    let message = nous_tool_gateway_unavailable_message("managed image generation", false);
    assert!(message.contains("managed image generation"), "{message}");
    assert!(message.contains("`hermes model`"), "{message}");

    let message = nous_tool_gateway_unavailable_message("the Nous Tool Gateway", true);
    assert!(message.contains("the Nous Tool Gateway"), "{message}");
}

// ── normalize_browser_cloud_provider ──────────────────────────────────────

#[test]
fn browser_provider_none_returns_default() {
    assert_eq!(normalize_browser_cloud_provider(None), "local");
}

#[test]
fn browser_provider_integer_coerced() {
    // Upstream passes int 42 into `str(value or "local")`; Rust callers
    // stringify first (Python's Any→str coercion is caller-side).
    let result = normalize_browser_cloud_provider(Some("42"));
    assert_eq!(result, "42");
}

#[test]
fn browser_provider_trims_and_lowercases() {
    assert_eq!(
        normalize_browser_cloud_provider(Some("  Chrome-Cloud  ")),
        "chrome-cloud"
    );
    assert_eq!(normalize_browser_cloud_provider(Some("")), "local");
}

// ── coerce_modal_mode / normalize_modal_mode ──────────────────────────────

#[test]
fn modal_mode_valid_modes_passthrough() {
    for mode in ["auto", "direct", "managed"] {
        assert_eq!(coerce_modal_mode(Some(mode)), mode);
    }
}

#[test]
fn modal_mode_none_returns_auto() {
    assert_eq!(coerce_modal_mode(None), "auto");
}

#[test]
fn modal_mode_strips_whitespace() {
    assert_eq!(coerce_modal_mode(Some("  managed  ")), "managed");
    assert_eq!(coerce_modal_mode(Some(" MANAGED ")), "managed");
}

#[test]
fn modal_mode_bogus_falls_back_to_auto() {
    assert_eq!(coerce_modal_mode(Some("bogus")), "auto");
}

#[test]
fn normalize_modal_mode_delegates_to_coerce() {
    assert_eq!(
        normalize_modal_mode(Some("direct")),
        coerce_modal_mode(Some("direct"))
    );
    assert_eq!(normalize_modal_mode(None), coerce_modal_mode(None));
    assert_eq!(
        normalize_modal_mode(Some("bogus")),
        coerce_modal_mode(Some("bogus"))
    );
}

// ── has_direct_modal_credentials ──────────────────────────────────────────

#[test]
fn direct_modal_credentials_no_env_no_file() {
    let _guard = EnvGuard::lock()
        .unset("MODAL_TOKEN_ID")
        .unset("MODAL_TOKEN_SECRET");
    let dir = tmp_dir("noenv");
    std::fs::create_dir_all(&dir).unwrap();
    let set_home = set_home_env(dir.to_str().unwrap());
    let result = has_direct_modal_credentials();
    set_home.restore();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(!result);
}

// Helper: point $HOME at a temp dir (hermes-constants get_real_home reads
// $HOME fresh on each call).
fn set_home_env(path: &str) -> HomeRestore {
    let old = std::env::var("HOME").ok();
    std::env::set_var("HOME", path);
    HomeRestore { old }
}

struct HomeRestore {
    old: Option<String>,
}
impl HomeRestore {
    fn restore(self) {
        match &self.old {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
    }
}

#[test]
fn direct_modal_credentials_only_token_secret_not_enough() {
    let _guard = EnvGuard::lock()
        .unset("MODAL_TOKEN_ID")
        .set("MODAL_TOKEN_SECRET", "sec-456");
    let dir = tmp_dir("secret");
    std::fs::create_dir_all(&dir).unwrap();
    let set_home = set_home_env(dir.to_str().unwrap());
    let result = has_direct_modal_credentials();
    set_home.restore();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(!result);
}

#[test]
fn direct_modal_credentials_env_vars_take_priority_over_file() {
    let _guard = EnvGuard::lock()
        .set("MODAL_TOKEN_ID", "id-123")
        .set("MODAL_TOKEN_SECRET", "sec-456");
    let dir = tmp_dir("envfile");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(".modal.toml"), "[modal]\ntoken_id = 'x'").unwrap();
    let set_home = set_home_env(dir.to_str().unwrap());
    let result = has_direct_modal_credentials();
    set_home.restore();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(result);
}

#[test]
fn direct_modal_credentials_file_alone_counts() {
    let _guard = EnvGuard::lock()
        .unset("MODAL_TOKEN_ID")
        .unset("MODAL_TOKEN_SECRET");
    let dir = tmp_dir("fileonly");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(".modal.toml"), "[modal]").unwrap();
    let set_home = set_home_env(dir.to_str().unwrap());
    let result = has_direct_modal_credentials();
    set_home.restore();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(result);
}

#[cfg(unix)]
#[test]
fn direct_modal_credentials_home_dir_permission_denied() {
    use std::os::unix::fs::PermissionsExt;
    let _guard = EnvGuard::lock()
        .unset("MODAL_TOKEN_ID")
        .unset("MODAL_TOKEN_SECRET");
    let dir = tmp_dir("permdenied");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o000)).unwrap();
    let set_home = set_home_env(dir.to_str().unwrap());
    let result = has_direct_modal_credentials();
    set_home.restore();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    // A failed home probe must not crash and must not fabricate credentials
    // (upstream issue #33525).
    assert!(!result);
}

#[cfg(unix)]
#[test]
fn direct_modal_credentials_permission_denied_with_env_vars() {
    use std::os::unix::fs::PermissionsExt;
    let _guard = EnvGuard::lock()
        .set("MODAL_TOKEN_ID", "id-123")
        .set("MODAL_TOKEN_SECRET", "sec-456");
    let dir = tmp_dir("permdeniedenv");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o000)).unwrap();
    let set_home = set_home_env(dir.to_str().unwrap());
    let result = has_direct_modal_credentials();
    set_home.restore();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(result);
}

// ── prefers_gateway ───────────────────────────────────────────────────────

#[test]
fn prefers_gateway_returns_false_for_quoted_false() {
    // Mock tier: the config override stands in for monkeypatching
    // hermes_cli.config.load_config.
    set_load_config_for_test(Some(json!({"web": {"use_gateway": "false"}})));
    assert!(!prefers_gateway("web"));
    set_load_config_for_test(None);
}

#[test]
fn prefers_gateway_returns_true_for_quoted_true() {
    set_load_config_for_test(Some(json!({"web": {"use_gateway": "true"}})));
    assert!(prefers_gateway("web"));
    set_load_config_for_test(None);
}

#[test]
fn prefers_gateway_honors_boolean_and_truthy_values() {
    set_load_config_for_test(Some(json!({
        "web": {"use_gateway": true},
        "tools": {"use_gateway": false},
        "misc": {"use_gateway": "on"},
        "other": {"use_gateway": "no"},
    })));
    assert!(prefers_gateway("web"));
    assert!(!prefers_gateway("tools"));
    assert!(prefers_gateway("misc"));
    assert!(!prefers_gateway("other"));
    set_load_config_for_test(None);
}

#[test]
fn prefers_gateway_missing_section_or_missing_config_is_false() {
    set_load_config_for_test(Some(json!({"web": {"use_gateway": "true"}})));
    assert!(!prefers_gateway("missing"));
    set_load_config_for_test(None);
    // No config loader in production (P3 seam) → fails open to false.
    assert!(!prefers_gateway("web"));
}

// ── resolve_modal_backend_state ───────────────────────────────────────────

fn resolve(mode: Option<&str>, has_direct: bool, managed_ready: bool, nous_enabled: bool) -> Value {
    // Mock tier: explicit managed_enabled mirrors the upstream test helper
    // monkeypatching managed_nous_tools_enabled.
    resolve_modal_backend_state(mode, has_direct, managed_ready, Some(nous_enabled))
}

#[test]
fn modal_auto_prefers_managed_when_available() {
    let result = resolve(Some("auto"), true, true, true);
    assert_eq!(result["selected_backend"], json!("managed"));
}

#[test]
fn modal_auto_falls_back_to_direct_when_managed_unavailable() {
    let result = resolve(Some("auto"), true, true, false);
    assert_eq!(result["selected_backend"], json!("direct"));
    let result = resolve(Some("auto"), true, false, true);
    assert_eq!(result["selected_backend"], json!("direct"));
    let result = resolve(Some("auto"), false, false, true);
    assert_eq!(result["selected_backend"], Value::Null);
}

#[test]
fn modal_direct_selects_direct_when_available() {
    let result = resolve(Some("direct"), true, true, true);
    assert_eq!(result["selected_backend"], json!("direct"));
}

#[test]
fn modal_direct_none_when_no_credentials() {
    let result = resolve(Some("direct"), false, true, true);
    assert_eq!(result["selected_backend"], Value::Null);
}

#[test]
fn modal_managed_blocked_when_nous_disabled() {
    let result = resolve(Some("managed"), true, true, false);
    assert_eq!(result["selected_backend"], Value::Null);
    assert_eq!(result["managed_mode_blocked"], json!(true));
}

#[test]
fn modal_managed_selected_when_available() {
    let result = resolve(Some("managed"), true, true, true);
    assert_eq!(result["selected_backend"], json!("managed"));
    assert_eq!(result["managed_mode_blocked"], json!(false));
}

#[test]
fn modal_invalid_mode_treated_as_auto() {
    let result = resolve(Some("bogus"), true, false, false);
    assert_eq!(result["requested_mode"], json!("auto"));
    assert_eq!(result["mode"], json!("auto"));
}

#[test]
fn modal_backend_state_structure() {
    let result = resolve(Some("auto"), true, true, false);
    assert_eq!(result["requested_mode"], json!("auto"));
    assert_eq!(result["mode"], json!("auto"));
    assert_eq!(result["has_direct"], json!(true));
    assert_eq!(result["managed_ready"], json!(true));
    assert_eq!(result["managed_mode_blocked"], json!(false));
}

#[test]
fn modal_backend_state_uses_managed_gate_when_enabled_unspecified() {
    // Mock tier: with managed_enabled=None the gate reads
    // managed_nous_tools_enabled() (upstream default); the override stands
    // in for the account-info monkeypatch.
    set_nous_entitlement_for_test(Some(true));
    let result = resolve_modal_backend_state(Some("auto"), true, true, None);
    assert_eq!(result["selected_backend"], json!("managed"));
    set_nous_entitlement_for_test(Some(false));
    let result = resolve_modal_backend_state(Some("auto"), true, true, None);
    assert_eq!(result["selected_backend"], json!("direct"));
    set_nous_entitlement_for_test(None);
}

// ── resolve_provider_secret / resolve_openai_audio_api_key ────────────────

#[test]
fn openai_audio_key_voice_key_preferred() {
    let _guard = EnvGuard::lock()
        .set("VOICE_TOOLS_OPENAI_KEY", "voice-key")
        .set("OPENAI_API_KEY", "general-key");
    assert_eq!(resolve_openai_audio_api_key(), "voice-key");
}

#[test]
fn openai_audio_key_strips_whitespace() {
    let _guard = EnvGuard::lock()
        .set("VOICE_TOOLS_OPENAI_KEY", "  voice-key  ")
        .unset("OPENAI_API_KEY");
    assert_eq!(resolve_openai_audio_api_key(), "voice-key");
}

#[test]
fn openai_audio_key_single_profile_still_reads_environ() {
    // Control: no multiplexing, no profile scope — unchanged behaviour
    // (upstream test_single_profile_still_reads_environ).
    let _guard = EnvGuard::lock()
        .unset("VOICE_TOOLS_OPENAI_KEY")
        .set("OPENAI_API_KEY", "sk-plain");
    assert_eq!(resolve_openai_audio_api_key(), "sk-plain");
}

#[test]
fn openai_audio_key_empty_when_nothing_configured() {
    let _guard = EnvGuard::lock()
        .unset("VOICE_TOOLS_OPENAI_KEY")
        .unset("OPENAI_API_KEY");
    assert_eq!(resolve_openai_audio_api_key(), "");
}

#[test]
fn provider_secret_config_value_wins() {
    let _guard = EnvGuard::lock()
        .unset("WHISPER_KEY")
        .set("WHISPER_KEY", "env-key");
    assert_eq!(
        resolve_provider_secret("WHISPER_KEY", "whisper", "cfg-key", None),
        "cfg-key"
    );
    assert_eq!(
        resolve_provider_secret("WHISPER_KEY", "whisper", "  padded  ", None),
        "padded"
    );
}

#[test]
fn provider_secret_env_used_when_no_config_value() {
    let _guard = EnvGuard::lock().set("WHISPER_KEY", "  env-key  ");
    assert_eq!(
        resolve_provider_secret("WHISPER_KEY", "whisper", "", None),
        "env-key"
    );
}

#[test]
fn provider_secret_env_getter_injected() {
    // Mock tier: upstream tests patch the caller's `get_env_value` module
    // wrapper; the env_getter parameter is the Rust seam for that.
    let _guard = EnvGuard::lock().unset("WHISPER_KEY");
    let getter = |key: &str| -> Option<String> {
        if key == "WHISPER_KEY" {
            Some("getter-key".to_string())
        } else {
            None
        }
    };
    assert_eq!(
        resolve_provider_secret("WHISPER_KEY", "whisper", "", Some(&getter)),
        "getter-key"
    );
    // env_getter is consulted even when the raw env read is empty.
    assert_eq!(
        resolve_provider_secret("WHISPER_KEY", "whisper", "  ", Some(&getter)),
        "getter-key"
    );
}

#[test]
fn provider_secret_empty_return_when_nothing_resolves() {
    let _guard = EnvGuard::lock().unset("WHISPER_KEY");
    assert_eq!(
        resolve_provider_secret("WHISPER_KEY", "whisper", "", None),
        ""
    );
    // Empty provider_id short-circuits the credential-pool step.
    assert_eq!(resolve_provider_secret("WHISPER_KEY", "", "", None), "");
}

// ── fal_key_is_configured ─────────────────────────────────────────────────

#[test]
fn fal_key_configured_when_set() {
    let _guard = EnvGuard::lock().unset("FAL_KEY").set("FAL_KEY", "fal-123");
    assert!(fal_key_is_configured());
}

#[test]
fn fal_key_unset_when_missing_or_whitespace() {
    {
        let _guard = EnvGuard::lock().unset("FAL_KEY");
        assert!(!fal_key_is_configured());
    }
    {
        let _guard = EnvGuard::lock().set("FAL_KEY", "   ");
        assert!(!fal_key_is_configured());
    }
}
