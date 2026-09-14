//! Ported sections of `run_agent.py`: module-level pure helpers (pin
//! lines ~69-103: session-establishment pair; ~234-371: scaffolding
//! flags, worker/marker constants, Qwen header builders, session
//! filename sanitizer, RouterMint UA; pool-recovery predicate;
//! ~373-411: stream error event; provider/URL predicates, max-tokens
//! and output-cap helpers (explicit-argument forms of methods in
//! ~1334-1680; self-reading no-arg forms belong to the loop slice).
//! stdlib logic plus same-crate pool types; higher-layer seams stay out:
//! - `_routermint_headers` reads `hermes_cli.__version__` lazily upstream;
//!   `hermes-agent` must not depend on the higher-layer `hermes-cli`
//!   crate, so the version arrives as an explicit argument.
//!
//! Evidence tiers: `unit` for cases mirroring upstream tests
//! (`tests/run_agent/test_dropped_tool_call_recovery.py`,
//! `tests/run_agent/test_run_agent.py`,
//! `tests/agent/test_verification_stop_caching.py`,
//! `tests/agent/test_gemini_fast_fallback.py`,
//! `tests/run_agent/test_provider_fallback.py`,
//! `tests/run_agent/test_session_source.py`,
//! `tests/run_agent/test_codex_xai_oauth_recovery.py`,
//! `tests/agent/test_direct_provider_url_detection.py`,
//! `tests/run_agent/test_codex_silent_hang_hint.py` @ b9aa928);
//! `unit/source-derived` where no upstream case pins the behavior
//! (header platform mapping, falsy-flag matrix, truncation bounds,
//! single-entry pool).

use super::credential_pool::CredentialPool;
use std::collections::HashMap;
use std::sync::OnceLock;

use hermes_utils::urls::{
    base_url_host_matches, base_url_hostname, model_forces_max_completion_tokens,
};
use regex::Regex;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// PARITY: `_EPHEMERAL_SCAFFOLDING_FLAGS` (pin ~234-256). Internal
/// recovery/verification/kanban/drop-retry markers that must never be
/// persisted to the durable transcript.
pub const EPHEMERAL_SCAFFOLDING_FLAGS: &[&str] = &[
    "_empty_recovery_synthetic",
    "_empty_terminal_sentinel",
    "_thinking_prefill",
    "_verification_stop_synthetic",
    "_pre_verify_synthetic",
    "_kanban_stop_synthetic",
    "_dropped_toolcall_nudge",
];

/// PARITY: `_MAX_TOOL_WORKERS = 8` (pin ~265).
pub const MAX_TOOL_WORKERS: u32 = 8;

/// PARITY: `_DB_PERSISTED_MARKER` (pin ~279). Stamped on a message dict
/// once written to the session store; the `_` prefix keeps it off the
/// wire (upstream wire sanitizers strip `_`-prefixed top-level keys).
pub const DB_PERSISTED_MARKER: &str = "_db_persisted";

/// PARITY: `_QWEN_CODE_VERSION` (pin ~298).
pub const QWEN_CODE_VERSION: &str = "0.14.1";

/// Python truthiness over JSON values, matching `msg.get(flag)` in the
/// upstream `any(...)`: `None`/`False`/`0`/`""`/`[]`/`{}` are falsy.
fn json_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        // JSON has no NaN/inf; u64/i64 zero-ness survives the f64 cast.
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

/// PARITY: `_is_ephemeral_scaffolding` (pin ~259-264). True when `msg`
/// carries any scaffolding flag with a truthy value. Non-dict inputs
/// are unrepresentable here (upstream returns False for them).
pub fn is_ephemeral_scaffolding(msg: &Map<String, Value>) -> bool {
    EPHEMERAL_SCAFFOLDING_FLAGS
        .iter()
        .any(|flag| msg.get(*flag).map(json_truthy).unwrap_or(false))
}

/// Map platform names to the `platform.system().lower()` /
/// `platform.machine()` spellings upstream interpolates into the Qwen
/// User-Agent. Upstream passes both through verbatim; only the
/// Rust↔CPython spelling gaps are bridged: `macos`→`darwin`, Apple
/// Silicon `aarch64`→`arm64` on macOS only (Linux ARM reports
/// `aarch64` upstream), and Windows `x86_64`→`AMD64` /
/// `aarch64`→`ARM64` (CPython reports kernel arch names there).
/// Everything else passes through byte-identical.
/// Source-derived: no upstream test pins the UA string.
pub fn qwen_platform_tokens(os: &str, arch: &str) -> (String, String) {
    let os_lc = os.to_lowercase();
    let system = match os_lc.as_str() {
        "macos" => "darwin".to_string(),
        other => other.to_string(),
    };
    let machine = match (os_lc.as_str(), arch) {
        ("macos" | "darwin", "aarch64") => "arm64".to_string(),
        ("windows", "x86_64") => "AMD64".to_string(),
        ("windows", "aarch64") => "ARM64".to_string(),
        _ => arch.to_string(),
    };
    (system, machine)
}

/// PARITY: `_pool_may_recover_from_rate_limit` (pin ~310-333). Decide
/// whether to wait for credential-pool rotation instead of falling back
/// (see issues #11314 and #13636): rotation needs the pool to exist,
/// have an entry outside exhaustion cooldown, and have more than one
/// entry to rotate to — a single-credential pool would just re-hit the
/// same exhausted quota. `now` is explicit, following
/// [`CredentialPool::has_available`]; upstream reads wall-clock inside
/// `pool.has_available()`.
///
/// NOTE — sibling predicate: the pin also defines the method
/// `AIAgent._credential_pool_may_recover_rate_limit` (pin ~6007), which
/// checks only `pool.has_available()` with no `len > 1` gate. The live
/// fallback path (`agent/conversation_loop.py:4627`) calls this module
/// function; the loop slice must preserve both shapes. Availability here
/// is evaluated read-only through the ported pool API (no disk sync);
/// the decision clock is the caller's `now`, which must be sampled fresh
/// at the decision point.
pub fn pool_may_recover_from_rate_limit(pool: Option<&CredentialPool>, now: f64) -> bool {
    match pool {
        None => false,
        Some(active) => {
            if !active.has_available(now) {
                return false;
            }
            active.entries().len() > 1
        }
    }
}

/// PARITY: `_qwen_portal_headers` (pin ~335-345), evaluated for this
/// process. See [`qwen_platform_tokens`] for the platform mapping.
pub fn qwen_portal_headers() -> HashMap<String, String> {
    qwen_portal_headers_for(std::env::consts::OS, std::env::consts::ARCH)
}

/// PARITY: `_qwen_portal_headers` with injectable platform names so
/// tests pin the grammar without touching the process platform.
pub fn qwen_portal_headers_for(system: &str, machine: &str) -> HashMap<String, String> {
    let (system, machine) = qwen_platform_tokens(system, machine);
    let user_agent = format!("QwenCode/{QWEN_CODE_VERSION} ({system}; {machine})");
    HashMap::from([
        ("User-Agent".to_string(), user_agent.clone()),
        ("X-DashScope-CacheControl".to_string(), "enable".to_string()),
        ("X-DashScope-UserAgent".to_string(), user_agent),
        ("X-DashScope-AuthType".to_string(), "qwen-oauth".to_string()),
    ])
}

/// PARITY: `_launch_cwd_for_session` (pin ~69-90). Working directory to
/// stamp on a new session row, or `None`. Only local CLI sessions record
/// a cwd; gateway/cron/remote sessions and non-`local` `TERMINAL_ENV`
/// backends record nothing. An unlinked cwd (`OSError` upstream, any
/// `std::io` error here) also yields `None`. A non-UTF8 cwd degrades to
/// U+FFFD replacement chars (`to_string_lossy`); upstream round-trips
/// such paths via surrogateescape, so byte-exotic cwds are a documented
/// edge, not a covered arm.
pub fn launch_cwd_for_session(source: &str) -> Option<String> {
    if source != "cli" {
        return None;
    }
    let backend = std::env::var("TERMINAL_ENV")
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    if !backend.is_empty() && backend != "local" {
        return None;
    }
    std::env::current_dir()
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

/// PARITY: `_session_source_for_agent` (pin ~93-103). Resolve the
/// session source: gateway session-context override first, then the
/// `HERMES_SESSION_SOURCE` env var, then `platform`, defaulting to
/// `"cli"`. The gateway context layer (`gateway.session_context`) is
/// unported, so the future gateway caller passes its resolved value as
/// `context_source` (`None` = contextvar unset); every other arm reads
/// exactly as upstream.
pub fn session_source_for_agent(platform: Option<&str>, context_source: Option<&str>) -> String {
    // Gateway session-context override. An explicitly set context value
    // masks the env layer entirely — even a blank one (`get_session_env`
    // returns explicitly-set `""` with no `os.environ` fallback, and the
    // blank then falls to platform); only a never-set context (`None`)
    // falls through to env.
    let platform_default = || {
        platform
            .filter(|s| !s.is_empty())
            .unwrap_or("cli")
            .to_string()
    };
    if let Some(raw) = context_source {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
        return platform_default();
    }
    let from_env = std::env::var("HERMES_SESSION_SOURCE").unwrap_or_default();
    let from_env = from_env.trim();
    if !from_env.is_empty() {
        return from_env.to_string();
    }
    platform_default()
}

/// PARITY: `_StreamErrorEvent` (pin ~373-411). Synthesized provider
/// error surfaced from a Responses `type=error` SSE frame, carrying the
/// OpenAI SDK-shaped `.body` so the (unported) error summarizers see a
/// familiar shape. Display is the message, matching `str(exc)`.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct StreamErrorEvent {
    pub message: String,
    pub code: Option<String>,
    pub param: Option<String>,
    /// HTTP-ish status; `u16` follows the crate's `AuxiliaryError` precedent.
    pub status_code: Option<u16>,
    pub body: Value,
}

impl StreamErrorEvent {
    pub fn new<M, C, P>(
        message: M,
        code: Option<C>,
        param: Option<P>,
        status_code: Option<u16>,
    ) -> Self
    where
        M: Into<String>,
        C: Into<String>,
        P: Into<String>,
    {
        let message = message.into();
        let code = code.map(Into::into);
        let param: Option<String> = param.map(Into::into);
        let body = serde_json::json!({
            "error": {
                "message": message,
                "code": code,
                "param": param,
                "type": "error",
            }
        });
        Self {
            message,
            code,
            param,
            status_code,
            body,
        }
    }
}

/// PARITY: `AIAgent._model_requires_responses_api` (pin, staticmethod).
/// True for models requiring the Responses API path (GPT-5.x rejected
/// on `/v1/chat/completions`). Vendor prefix stripped to the tail after
/// the last `/`.
pub fn model_requires_responses_api(model: &str) -> bool {
    let tail = match model.rsplit_once('/') {
        Some((_, tail)) => tail,
        None => model,
    };
    tail.to_lowercase().starts_with("gpt-5")
}

/// PARITY: `AIAgent._provider_model_requires_responses_api` (pin).
/// Provider/model routing: Nous and generic custom endpoints stay on
/// chat completions. The Copilot-specific `hermes_cli.models` check is
/// unported — upstream's own `except Exception: pass` falls back to the
/// generic GPT-5 rule, which is exactly what this port applies.
pub fn provider_model_requires_responses_api(model: &str, provider: Option<&str>) -> bool {
    let normalized = provider.unwrap_or("").trim().to_lowercase();
    if normalized == "nous" || normalized == "custom" {
        return false;
    }
    model_requires_responses_api(model)
}

/// PARITY: `AIAgent._is_direct_openai_url` (pin), explicit-URL form.
/// The no-arg form reads cached agent URL fields; the loop slice owns it.
pub fn is_direct_openai_url(base_url: &str) -> bool {
    base_url_hostname(base_url) == "api.openai.com"
}

/// PARITY: `AIAgent._is_azure_openai_url` (pin), explicit-URL form.
/// Substring match on the lowered URL, exactly as upstream (Azure must
/// stay off the Responses path even though it is OpenAI-compatible).
pub fn is_azure_openai_url(base_url: &str) -> bool {
    base_url.to_lowercase().contains("openai.azure.com")
}

/// PARITY: `AIAgent._is_github_copilot_url` (pin), explicit-URL form.
pub fn is_github_copilot_url(base_url: &str) -> bool {
    let hostname = base_url_hostname(base_url);
    if hostname.is_empty() {
        return false;
    }
    hostname == "api.githubcopilot.com" || hostname.ends_with(".githubcopilot.com")
}

/// PARITY: `AIAgent._is_openrouter_url` (pin), explicit-URL form.
pub fn is_openrouter_url(base_url: &str) -> bool {
    base_url_host_matches(base_url, "openrouter.ai")
}

/// PARITY: `AIAgent._is_copilot_url` (pin), explicit-URL form. Upstream
/// reads the pre-lowered `_base_url_lower` field; the input is lowered
/// here to reproduce the no-arg behavior for any casing.
pub fn is_copilot_url(base_url: &str) -> bool {
    let lowered = base_url.to_lowercase();
    lowered.contains("api.githubcopilot.com") || lowered.contains("models.github.ai")
}

/// PARITY: `AIAgent._is_copilot_provider` (pin). Single owner of the
/// Copilot check: alias spellings first, Copilot base URL as fallback.
pub fn is_copilot_provider(provider: Option<&str>, base_url: &str) -> bool {
    let normalized = provider.unwrap_or("").trim().to_lowercase();
    if matches!(normalized.as_str(), "copilot" | "github-copilot" | "github") {
        return true;
    }
    is_copilot_url(base_url)
}

/// PARITY: `AIAgent._is_codex_backend` (pin), explicit-argument form.
/// Hostname/URL are lowered here to reproduce the pre-lowered agent
/// fields the no-arg form reads; `api_mode` compares exact.
pub fn is_codex_backend(api_mode: &str, hostname: &str, base_url: &str) -> bool {
    api_mode == "codex_responses"
        && hostname.to_lowercase() == "chatgpt.com"
        && base_url.to_lowercase().contains("/backend-api/codex")
}

fn codex_hang_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?:^|[/\-_])gpt-5\.5(?:$|[\-_])").expect("codex hang pattern"))
}

/// PARITY: `AIAgent._codex_silent_hang_hint` (pin). Actionable hint for
/// the known Codex silent-reject pattern (gpt-5.5 family on the ChatGPT
/// Codex backend; hermes-agent #21444), else `None`. `model` overrides
/// `current_model`, mirroring `model if model is not None else
/// self.model`. The `{eff_model!r}` interpolation renders with Python
/// single quotes.
pub fn codex_silent_hang_hint(
    api_mode: &str,
    provider: &str,
    hostname: &str,
    base_url: &str,
    model: Option<&str>,
    current_model: &str,
) -> Option<String> {
    let backend = provider == "openai-codex"
        || (hostname.to_lowercase() == "chatgpt.com"
            && base_url.to_lowercase().contains("/backend-api/codex"));
    if api_mode != "codex_responses" || !backend {
        return None;
    }
    let effective = model.unwrap_or(current_model).to_string();
    let lowered = effective.to_lowercase();
    if !codex_hang_pattern().is_match(&lowered) {
        return None;
    }
    Some(format!(
        "Codex backend appears to be silently rejecting '{effective}' \
         on chatgpt.com/backend-api/codex (no stream events, no error). \
         This is a known backend-side pattern that has affected ChatGPT \
         Plus accounts intermittently. \
         Workaround: try `gpt-5.4` on the same OAuth profile, or `gpt-5.3-codex`, \
         or switch to a different model/provider in your fallback chain. \
         Some ChatGPT Codex accounts do not support `gpt-5.4-codex`. \
         See hermes-agent#21444 for symptom history."
    ))
}

/// PARITY: `AIAgent._max_tokens_param` (pin). URL-first, then a
/// model-name fallback so third-party endpoints fronting newer families
/// are recognised. Returns the kwarg key with the value (upstream
/// returns a single-entry dict).
pub fn max_tokens_param(value: i64, base_url: &str, model: &str) -> (&'static str, i64) {
    if is_direct_openai_url(base_url)
        || is_azure_openai_url(base_url)
        || is_github_copilot_url(base_url)
        || model_forces_max_completion_tokens(model)
    {
        return ("max_completion_tokens", value);
    }
    ("max_tokens", value)
}

/// PARITY: `AIAgent._requested_output_cap_from_api_kwargs` (pin,
/// staticmethod). First positive int across `max_output_tokens`,
/// `max_completion_tokens`, `max_tokens`. Coercion mirrors `int(raw)`:
/// floats truncate toward zero, numeric strings (trimmed) parse,
/// `True` counts as 1; non-numeric, missing, or non-positive entries
/// fall through to the next key. (Python bignums beyond i64 are out of
/// domain; floats saturate at the i64 bounds.)
pub fn requested_output_cap_from_api_kwargs(payload: &Map<String, Value>) -> Option<i64> {
    for key in ["max_output_tokens", "max_completion_tokens", "max_tokens"] {
        let value = match payload.get(key) {
            Some(Value::Number(n)) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
            Some(Value::Bool(b)) => Some(*b as i64),
            Some(Value::String(s)) => s.trim().parse::<i64>().ok(),
            _ => None,
        };
        if let Some(positive) = value.filter(|v| *v > 0) {
            return Some(positive);
        }
    }
    None
}

/// PARITY: `_routermint_headers` (pin ~303-309). The Hermes version is
/// an explicit argument (see module docs): upstream reads
/// `hermes_cli.__version__` via a lazy in-function import.
pub fn routermint_headers(hermes_version: &str) -> HashMap<String, String> {
    HashMap::from([(
        "User-Agent".to_string(),
        format!("HermesAgent/{hermes_version}"),
    )])
}

fn sanitize_filename_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Calibrated against the live Python oracle: `re` `\w` on `str` keeps
    // exactly letters + numbers + `_` (drops all marks Mn/Mc/Me, all Pc
    // except U+005F, Cf format controls incl. ZWNJ/ZWJ, symbols/emoji).
    // That set is `[\p{L}\p{N}_]` — verified char-by-char (é/中/²/Ⅷ/ñ
    // kept; ZWNJ/ZWJ/emoji/combining-acute/‿ dropped). A bare `\w` would
    // wrongly keep Join_Control in the Rust `regex` engine.
    RE.get_or_init(|| Regex::new(r"[^\p{L}\p{N}_-]").expect("filename sanitizer regex"))
}

/// Python `str.strip()` whitespace: Unicode `White_Space` (what Rust
/// `char::is_whitespace` reports) plus the C0 controls `\x1c`-`\x1f`
/// (FS/GS/RS/US), which Python strips but Rust does not.
fn trim_py_spaces(text: &str) -> &str {
    text.trim_matches(|c: char| c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c))
}

/// PARITY: `_safe_session_filename_component` (pin ~348-371). Collapse
/// every non-letter/number/`_`/`-` char to `_`, strip edge `.`/`_`, cap
/// at 96 chars (by Unicode scalar, as upstream slices code points), fall
/// back to `"session"`, and — when sanitization changed the string —
/// append a 12-hex-char sha256 disambiguator so distinct IDs never
/// collide. Always returns a single traversal-free path segment.
/// Upstream hashes with `errors="surrogatepass"`: the digest domain here
/// is `&str` (valid UTF-8 by construction), so lone surrogates are
/// unrepresentable — callers must pass decoded text, and non-`str`
/// inputs must be coerced with `str(x or "")` before calling, exactly as
/// the upstream signature accepts `Any`.
pub fn safe_session_filename_component(session_id: &str) -> String {
    let raw = trim_py_spaces(session_id);
    let mut sanitized: String = sanitize_filename_re()
        .replace_all(raw, "_")
        .trim_matches(['.', '_'])
        .chars()
        .take(96)
        .collect();
    if sanitized.is_empty() {
        sanitized = "session".to_string();
    }
    if !raw.is_empty() && sanitized == raw {
        return sanitized;
    }
    let digest = format!("{:x}", Sha256::digest(raw.as_bytes()));
    format!("{sanitized}_{}", &digest[..12])
}
