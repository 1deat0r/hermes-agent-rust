//! Shared helpers for tool backend selection.
//!
//! PARITY: tools/tool_backend_helpers.py @ b9aa928 (311 LOC, ported 1:1
//! for the observable surfaces; fail-open paths are pinned to the upstream
//! exception fallbacks).
//!
//! DEFERRED SEAMS (each fails open exactly like the upstream `except
//! Exception` path):
//! - `hermes_cli.nous_account` (`get_nous_portal_account_info`,
//!   `format_nous_portal_entitlement_message`) — P3 CLI crate; until it
//!   lands, `managed_nous_tools_enabled` returns `false` (fails closed)
//!   and `nous_tool_gateway_unavailable_message` returns the plain
//!   fallback guidance.  `#[doc(hidden)]` test overrides stand in for the
//!   upstream `monkeypatch.setattr("hermes_cli.nous_account...")` hooks.
//! - `agent.secret_scope` (`get_secret`, `is_multiplex_active`) and
//!   `agent.credential_pool` (`load_pool`) — agent/ crate (later phase);
//!   `_scoped_credential` degrades to a plain env read and the credential
//!   pool is skipped, which is the upstream behavior outside a multiplexed
//!   gateway turn with no pool.
//! - `hermes_cli.config` (`load_config`, `get_env_value`) — P3 CLI crate;
//!   `prefers_gateway` degrades to `false` unless a `#[doc(hidden)]` test
//!   config override is installed; env reads are direct `std::env::var`.
//!
//! `is_truthy_value` remains a tiny JSON-facing local copy of `utils.py`'s
//! helper (22–30); the shared crate's Rust-shaped helper accepts a different
//! value representation, so routing this function through it would change
//! coercion behavior.

use std::cell::{Cell, RefCell};
use std::path::Path;

use serde_json::{json, Value};

/// Upstream `utils.py` TRUTHY_STRINGS (19–21).
const TRUTHY_STRINGS: [&str; 4] = ["1", "true", "yes", "on"];

const DEFAULT_BROWSER_PROVIDER: &str = "local";

/// Caller-supplied `get_env_value` wrapper (the seam upstream tests patch).
pub type EnvGetter = dyn Fn(&str) -> Option<String>;
const DEFAULT_MODAL_MODE: &str = "auto";
const VALID_MODAL_MODES: [&str; 3] = ["auto", "direct", "managed"];

/// Coerce bool-ish values using the project's shared truthy string set.
///
/// PARITY: utils.py `is_truthy_value` @ b9aa928 (22–30). JSON-facing local
/// copy; see the module-level seam note.
fn is_truthy_value(value: &Value, default: bool) -> bool {
    match value {
        Value::Null => default,
        Value::Bool(b) => *b,
        Value::String(s) => {
            let lowered = s.trim().to_lowercase();
            TRUTHY_STRINGS.contains(&lowered.as_str())
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i != 0
            } else if let Some(f) = n.as_f64() {
                f != 0.0
            } else {
                true
            }
        }
        // Python `bool(container)` is True for non-empty containers; JSON
        // arrays/objects here are non-empty by construction of a Value.
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

// ── test / seam overrides (upstream relies on monkeypatch) ────────────────

thread_local! {
    /// `managed_nous_tools_enabled` override (stands in for monkeypatching
    /// `hermes_cli.nous_account.get_nous_portal_account_info`).
    static NOUS_ENTITLEMENT_OVERRIDE: Cell<Option<bool>> = const { Cell::new(None) };
    /// `prefers_gateway` config override (stands in for monkeypatching
    /// `hermes_cli.config.load_config`).
    static CONFIG_OVERRIDE: RefCell<Option<Value>> = const { RefCell::new(None) };
}

/// Test seam: force the Nous-account entitlement verdict.
#[doc(hidden)]
pub fn set_nous_entitlement_for_test(value: Option<bool>) {
    NOUS_ENTITLEMENT_OVERRIDE.with(|slot| slot.set(value));
}

/// Test seam: force `load_config()` output for `prefers_gateway`.
#[doc(hidden)]
pub fn set_load_config_for_test(config: Option<Value>) {
    CONFIG_OVERRIDE.with(|slot| *slot.borrow_mut() = config);
}

// ── Nous Tool Gateway gating ──────────────────────────────────────────────

/// Return True when the user is entitled to the Nous Tool Gateway.
///
/// Entitlement is paid Nous Portal service access OR a live free tool pool.
/// Tool Gateway availability fails closed on unknown/error entitlement.
/// `force_fresh` is for interactive configuration flows that should
/// reflect a just-purchased subscription immediately.
///
/// PARITY: tools/tool_backend_helpers.py `managed_nous_tools_enabled`
/// @ b9aa928.  The `hermes_cli.nous_account` read is a deferred seam; this
/// port fails closed (`false`) like the upstream exception path.
pub fn managed_nous_tools_enabled(force_fresh: bool) -> bool {
    if let Some(verdict) = NOUS_ENTITLEMENT_OVERRIDE.with(|slot| slot.get()) {
        return verdict;
    }
    // Upstream imports hermes_cli.nous_account.get_nous_portal_account_info
    // and returns account_info.tool_gateway_entitled when logged in.  The
    // CLI crate is unported, so this always takes the `except Exception:
    // return False` path.
    let _ = force_fresh;
    false
}

/// Return account-aware guidance for an unavailable Nous Tool Gateway path.
///
/// PARITY: tools/tool_backend_helpers.py
/// `nous_tool_gateway_unavailable_message` @ b9aa928.  The
/// `hermes_cli.nous_account` formatting is a deferred seam; the plain
/// fallback guidance is returned (the exact upstream behavior whenever the
/// entitlement formatter returns an empty string or raises).
pub fn nous_tool_gateway_unavailable_message(capability: &str, force_fresh: bool) -> String {
    let _ = force_fresh;
    format!(
        "{capability} is unavailable. Run `hermes model` to refresh your \
         Nous Portal login and billing status."
    )
}

// ── pure coercion helpers ─────────────────────────────────────────────────

/// Return a normalized browser provider key.
///
/// PARITY: tools/tool_backend_helpers.py `normalize_browser_cloud_provider`
/// @ b9aa928.  Python's `str(value or "local")` coercion is caller-side in
/// Rust, so `value` arrives already stringified.
pub fn normalize_browser_cloud_provider(value: Option<&str>) -> String {
    let provider = value
        .unwrap_or(DEFAULT_BROWSER_PROVIDER)
        .trim()
        .to_lowercase();
    if provider.is_empty() {
        DEFAULT_BROWSER_PROVIDER.to_string()
    } else {
        provider
    }
}

/// Return the requested modal mode when valid, else the default (`"auto"`).
pub fn coerce_modal_mode(value: Option<&str>) -> String {
    let mode = value.unwrap_or(DEFAULT_MODAL_MODE).trim().to_lowercase();
    if VALID_MODAL_MODES.contains(&mode.as_str()) {
        mode
    } else {
        DEFAULT_MODAL_MODE.to_string()
    }
}

/// Return a normalized modal execution mode (alias of `coerce_modal_mode`).
pub fn normalize_modal_mode(value: Option<&str>) -> String {
    coerce_modal_mode(value)
}

/// Return True when direct Modal credentials/config are available.
///
/// PARITY: tools/tool_backend_helpers.py `has_direct_modal_credentials`
/// @ b9aa928.  Home resolution goes through hermes-constants
/// (`get_real_home`, the `Path.home()` equivalent); a failed home/file
/// probe degrades to `false` exactly like the upstream `except
/// (PermissionError, OSError)` path.
pub fn has_direct_modal_credentials() -> bool {
    // Upstream truthiness: an empty env value is treated as unset.
    let token_id = std::env::var("MODAL_TOKEN_ID")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let token_secret = std::env::var("MODAL_TOKEN_SECRET")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let modal_file_exists = {
        let home = hermes_constants::get_real_home(None);
        Path::new(&home).join(".modal.toml").exists()
    };
    (token_id && token_secret) || modal_file_exists
}

/// Resolve direct vs managed Modal backend selection.
///
/// Semantics:
/// - `direct` means direct-only
/// - `managed` means managed-only
/// - `auto` prefers managed when available, then falls back to direct
///
/// PARITY: tools/tool_backend_helpers.py `resolve_modal_backend_state`
/// @ b9aa928.
pub fn resolve_modal_backend_state(
    modal_mode: Option<&str>,
    has_direct: bool,
    managed_ready: bool,
    managed_enabled: Option<bool>,
) -> Value {
    let requested_mode = coerce_modal_mode(modal_mode);
    let normalized_mode = normalize_modal_mode(modal_mode);
    let managed_enabled = match managed_enabled {
        Some(v) => v,
        None => managed_nous_tools_enabled(false),
    };
    let managed_mode_blocked = requested_mode == "managed" && !managed_enabled;

    let selected_backend = if normalized_mode == "managed" {
        if managed_enabled && managed_ready {
            Some("managed")
        } else {
            None
        }
    } else if normalized_mode == "direct" {
        if has_direct {
            Some("direct")
        } else {
            None
        }
    } else if managed_enabled && managed_ready {
        Some("managed")
    } else if has_direct {
        Some("direct")
    } else {
        None
    };

    json!({
        "requested_mode": requested_mode,
        "mode": normalized_mode,
        "has_direct": has_direct,
        "managed_ready": managed_ready,
        "managed_mode_blocked": managed_mode_blocked,
        "selected_backend": selected_backend,
    })
}

// ── credential resolution ─────────────────────────────────────────────────

/// Read a credential env var under the active profile secret scope.
///
/// Falls back to a raw env read when `agent.secret_scope` cannot be
/// imported (the deferred seam), so a packaging edge never leaves the
/// caller without a key.
///
/// PARITY: tools/tool_backend_helpers.py `_scoped_credential` @ b9aa928.
fn scoped_credential(name: &str) -> String {
    std::env::var(name).unwrap_or_default().trim().to_string()
}

/// Resolve a voice-provider API key.  Single owner for STT/TTS key lookup.
///
/// Resolution order (upstream #68003 fix):
///
/// 1. An explicit `config_value` from config.yaml, when the caller has one.
/// 2. The environment / `~/.hermes/.env` (profile secret scope —
///    `agent.secret_scope` is a deferred seam, so this is a plain env
///    read, which is the upstream behavior outside multiplexing).
/// 3. The credential pool / auth store for `provider_id` (deferred seam —
///    `agent.credential_pool` unported; skipped exactly like a
///    non-multiplexed turn with no `hermes auth add` pool entries).
///
/// Never raises — returns `""` when no key is found anywhere.
///
/// `env_getter` mirrors the caller-supplied `get_env_value` wrapper that
/// upstream tests patch.
///
/// PARITY: tools/tool_backend_helpers.py `resolve_provider_secret`
/// @ b9aa928.
pub fn resolve_provider_secret(
    env_var: &str,
    provider_id: &str,
    config_value: &str,
    env_getter: Option<&EnvGetter>,
) -> String {
    let value = config_value.trim();
    if !value.is_empty() {
        return value.to_string();
    }

    // Scope-aware env read (degraded to a plain env read — see above).
    let key = scoped_credential(env_var);
    if !key.is_empty() {
        return key;
    }

    // `is_multiplex_active()` check: deferred seam — treated as inactive,
    // so we fall through to the env/.env read like the upstream
    // non-multiplexed turn.
    let key = match env_getter {
        Some(getter) => getter(env_var).unwrap_or_default().trim().to_string(),
        None => std::env::var(env_var)
            .unwrap_or_default()
            .trim()
            .to_string(),
    };
    if !key.is_empty() {
        return key;
    }

    if provider_id.is_empty() {
        return String::new();
    }

    // Credential pool seam: `agent.credential_pool.load_pool` is unported;
    // there is no pool to read, so nothing to return here.
    let _ = provider_id;
    String::new()
}

/// Prefer the voice-tools key, but fall back to the normal OpenAI key.
///
/// PARITY: tools/tool_backend_helpers.py `resolve_openai_audio_api_key`
/// @ b9aa928.  The profile secret scope and credential-pool paths are the
/// same deferred seams as `resolve_provider_secret`.
pub fn resolve_openai_audio_api_key() -> String {
    let voice = resolve_provider_secret("VOICE_TOOLS_OPENAI_KEY", "", "", None);
    if !voice.is_empty() {
        return voice;
    }
    resolve_provider_secret("OPENAI_API_KEY", "openai-api", "", None)
}

/// Return True when the user opted into the Tool Gateway for this tool.
///
/// Reads `<section>.use_gateway` from config.yaml.  Never raises.
///
/// PARITY: tools/tool_backend_helpers.py `prefers_gateway` @ b9aa928.
/// `hermes_cli.config.load_config` is a deferred seam; production callers
/// degrade to `false` (the upstream `except Exception` path) until the CLI
/// crate lands.
pub fn prefers_gateway(config_section: &str) -> bool {
    let config = CONFIG_OVERRIDE.with(|slot| slot.borrow().clone());
    let Some(config) = config else {
        return false;
    };
    let section = config.get(config_section);
    let Some(Value::Object(map)) = section else {
        return false;
    };
    let use_gateway = map.get("use_gateway").cloned().unwrap_or(Value::Null);
    is_truthy_value(&use_gateway, false)
}

/// Return True when FAL_KEY is set to a non-whitespace value.
///
/// Consults both `os.environ` and `~/.hermes/.env` upstream; here the
/// `hermes_cli.config.get_env_value` fallback is the same env read (deferred
/// seam).  A whitespace-only value is treated as unset everywhere.
///
/// PARITY: tools/tool_backend_helpers.py `fal_key_is_configured`
/// @ b9aa928.
pub fn fal_key_is_configured() -> bool {
    let value = scoped_credential("FAL_KEY");
    let value = if value.is_empty() {
        // Upstream falls back to `hermes_cli.config.get_env_value("FAL_KEY")`
        // for CLI paths that run before dotenv loads; in this port the
        // config seam reads the same env var.
        std::env::var("FAL_KEY")
            .unwrap_or_default()
            .trim()
            .to_string()
    } else {
        value
    };
    !value.trim().is_empty()
}
