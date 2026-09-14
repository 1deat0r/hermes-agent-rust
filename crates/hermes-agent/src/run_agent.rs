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
//! `tests/run_agent/test_codex_silent_hang_hint.py`,
//! `hermes_cli/models.py::_should_use_copilot_responses_api` (rule body),
//! `tests/run_agent/test_codex_xai_oauth_recovery.py` Fix D (entitlement),
//! `tests/run_agent/test_run_agent.py::TestMaskApiKey` (intent — pin key
//! redacted) @ b9aa928);
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

/// PARITY: `hermes_cli.models._should_use_copilot_responses_api`
/// (upstream `hermes_cli/models.py:4049-4063`; opencode logic, pure
/// `re` only — no CLI runtime needed, so it lives here rather than
/// behind the layer boundary): GPT-5+ models use Responses, except
/// `gpt-5-mini`; non-GPT models stay on chat completions. Case- and
/// prefix-sensitive exactly as upstream (`re.match`, no lowercasing —
/// `GPT-5` does not match).
pub fn copilot_requires_responses_api(model_id: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    let pattern = RE.get_or_init(|| Regex::new(r"^gpt-(\d+)").expect("copilot responses pattern"));
    let major: Option<u32> = pattern
        .captures(model_id)
        .and_then(|captured| captured.get(1))
        .and_then(|digits| digits.as_str().parse().ok());
    match major {
        Some(major) => major >= 5 && !model_id.starts_with("gpt-5-mini"),
        None => false,
    }
}

/// PARITY: `AIAgent._provider_model_requires_responses_api` (pin).
/// Provider/model routing: Nous and generic custom endpoints stay on
/// chat completions. The Copilot arm applies the ported
/// [`copilot_requires_responses_api`] rule (pure `re` logic shared with
/// upstream `hermes_cli.models`, so no higher-layer dependency).
pub fn provider_model_requires_responses_api(model: &str, provider: Option<&str>) -> bool {
    let normalized = provider.unwrap_or("").trim().to_lowercase();
    if normalized == "nous" || normalized == "custom" {
        return false;
    }
    if normalized == "copilot" {
        return copilot_requires_responses_api(model);
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
/// The hostname is derived from `base_url` exactly as the agent caches
/// `_base_url_hostname` (`_base_url_hostname = base_url_hostname(value)`,
/// pin ~433); `api_mode` compares exact.
pub fn is_codex_backend(api_mode: &str, base_url: &str) -> bool {
    api_mode == "codex_responses"
        && base_url_hostname(base_url) == "chatgpt.com"
        && base_url.to_lowercase().contains("/backend-api/codex")
}

fn codex_hang_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?:^|[/\-_])gpt-5\.5(?:$|[\-_])").expect("codex hang pattern"))
}

/// Python `repr()` for short single-line strings (model names): single
/// quotes unless the value contains `'` without `"`, mirroring CPython.
/// Verified against the oracle (`it's-x` → `"it's-x"`, `a"b` → `'a"b'`).
fn py_repr_str(value: &str) -> String {
    if value.contains('\'') && !value.contains('"') {
        return format!("\"{value}\"");
    }
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('\'');
    for ch in value.chars() {
        if ch == '\\' || ch == '\'' {
            quoted.push('\\');
        }
        quoted.push(ch);
    }
    quoted.push('\'');
    quoted
}

/// PARITY: `AIAgent._codex_silent_hang_hint` (pin). Actionable hint for
/// the known Codex silent-reject pattern (gpt-5.5 family on the ChatGPT
/// Codex backend; hermes-agent #21444), else `None`. `model` overrides
/// `current_model`, mirroring `model if model is not None else
/// self.model`; the hostname derives from `base_url` as in
/// [`is_codex_backend`].
pub fn codex_silent_hang_hint(
    api_mode: &str,
    provider: &str,
    base_url: &str,
    model: Option<&str>,
    current_model: &str,
) -> Option<String> {
    let backend = provider == "openai-codex" || is_codex_backend(api_mode, base_url);
    if api_mode != "codex_responses" || !backend {
        return None;
    }
    let effective = model.unwrap_or(current_model).to_string();
    let lowered = effective.to_lowercase();
    if !codex_hang_pattern().is_match(&lowered) {
        return None;
    }
    let quoted = py_repr_str(&effective);
    Some(format!(
        "Codex backend appears to be silently rejecting {quoted} \
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
/// `int(raw)`: floats truncate toward zero, numeric strings (trimmed)
/// parse, `True` counts as 1; non-numeric, missing, or non-positive
/// entries fall through to the next key. Underscore digit separators
/// follow CPython (`"1_0"` parses, `"_1"`/`"1__0"` do not); non-ASCII
/// digits are out of domain. (Python bignums beyond i64 are out of
/// domain; floats saturate at the i64 bounds, unreachable from JSON.)
pub fn requested_output_cap_from_api_kwargs(payload: &Map<String, Value>) -> Option<i64> {
    for key in ["max_output_tokens", "max_completion_tokens", "max_tokens"] {
        let value = match payload.get(key) {
            Some(Value::Number(n)) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
            Some(Value::Bool(b)) => Some(*b as i64),
            Some(Value::String(s)) => parse_py_int(s.trim()),
            _ => None,
        };
        if let Some(positive) = value.filter(|v| *v > 0) {
            return Some(positive);
        }
    }
    None
}

/// Parse an ASCII base-10 integer with CPython `int(str)` underscore
/// rules: an optional sign, then digits with single underscores only
/// between digits.
fn parse_py_int(text: &str) -> Option<i64> {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    if digits.is_empty() {
        return None;
    }
    let mut chars = digits.chars();
    let mut prev_underscore = true;
    for ch in &mut chars {
        if ch == '_' {
            if prev_underscore {
                return None;
            }
            prev_underscore = true;
        } else if ch.is_ascii_digit() {
            prev_underscore = false;
        } else {
            return None;
        }
    }
    if prev_underscore {
        return None;
    }
    text.replace('_', "").parse::<i64>().ok()
}

/// PARITY: `AIAgent._is_entitlement_failure` (pin, staticmethod).
/// Subscription/entitlement 403s masquerading as auth failures: True
/// only when the body matches a known entitlement shape AND status is
/// 401/403/`None`. A non-mapping context can only arrive as `None`
/// here (upstream returns False for non-dicts).
pub fn is_entitlement_failure(
    error_context: Option<&Map<String, Value>>,
    status_code: Option<i64>,
) -> bool {
    if !matches!(status_code, None | Some(401) | Some(403)) {
        return false;
    }
    let context = match error_context {
        Some(map) => map,
        None => return false,
    };
    // Single lowercase haystack over every field shape the body might
    // land in (`message`/`reason` normalized plus raw `code`/`error`
    // keys, so the WKE disambiguator fires regardless of entry point).
    let mut haystack = String::new();
    for key in ["message", "reason", "code", "error"] {
        haystack.push_str(&haystack_text(context.get(key)));
        haystack.push(' ');
    }
    if haystack.trim().is_empty() {
        return false;
    }
    // Stale-token disambiguator (#29344): explicit unauthenticated
    // signals route through credential refresh, never surface here.
    if haystack.contains("[wke=unauthenticated:") {
        return false;
    }
    if haystack.contains("oauth2 access token could not be validated") {
        return false;
    }
    if haystack.contains("do not have an active grok subscription") {
        return true;
    }
    if haystack.contains("out of available resources") && haystack.contains("grok") {
        return true;
    }
    if haystack.contains("does not have permission") && haystack.contains("grok") {
        return true;
    }
    false
}

/// `str(value or "")` for haystack building: missing/`None` read as
/// empty, bools use Python spellings, containers serialize compact.
fn haystack_text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.to_lowercase(),
        Some(Value::Bool(true)) => "true".to_string(),
        Some(Value::Bool(false)) => "false".to_string(),
        Some(Value::Number(n)) => n.to_string().to_lowercase(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| haystack_text(Some(item)))
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase(),
        Some(Value::Object(map)) => {
            // Python `str(dict)` repr; compact JSON keeps the same words
            // in play for keyword matching (quotes differ, phrases don't).
            serde_json::to_string(map)
                .unwrap_or_default()
                .to_lowercase()
        }
    }
}

/// PARITY: `AIAgent._decorate_xai_entitlement_error` (pin,
/// staticmethod). Appends the SuperGrok hint to xAI permission-denied
/// detail text; idempotent (won't double-decorate) and passthrough for
/// empty/non-entitlement input. Hint text verbatim.
pub fn decorate_xai_entitlement_error(detail: &str) -> String {
    if detail.is_empty() {
        return String::new();
    }
    let lower = detail.to_lowercase();
    let is_entitlement = lower.contains("do not have an active grok subscription")
        || (lower.contains("out of available resources") && lower.contains("grok"))
        || (lower.contains("does not have permission") && lower.contains("grok"));
    if !is_entitlement {
        return detail.to_string();
    }
    if detail.contains("X Premium+ does NOT include") {
        return detail.to_string();
    }
    format!(
        "{detail} — xAI rejected this OAuth account. NOTE: X Premium+ does NOT \
         include xAI API access — only standalone SuperGrok subscribers \
         can use this provider. Other possible causes: no Grok \
         subscription, your tier doesn't include this model, or your \
         quota is exhausted. Check https://grok.com/?_s=usage to see \
         which, or run `/model` to switch providers."
    )
}

/// PARITY: `AIAgent._coerce_api_error_detail` (pin, staticmethod).
/// Display-safe string for structured provider error fields: verbatim
/// strings; dicts prefer the first non-blank of
/// message/detail/error/code/type, then recurse, then fall back to
/// sorted-key JSON; lists join non-blank items with `"; "`; `None`
/// reads as empty; bools use Python spellings (`True`/`False`).
pub fn coerce_api_error_detail(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Object(map) => {
            for key in ["message", "detail", "error", "code", "type"] {
                if let Some(Value::String(nested)) = map.get(key) {
                    if !nested.trim().is_empty() {
                        return nested.clone();
                    }
                }
            }
            for key in ["message", "detail", "error", "code", "type"] {
                if let Some(nested) = map.get(key) {
                    let coerced = coerce_api_error_detail(nested);
                    if !coerced.is_empty() {
                        return coerced;
                    }
                }
            }
            sorted_json(value)
        }
        Value::Array(items) => items
            .iter()
            .map(coerce_api_error_detail)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("; "),
        Value::Null => String::new(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::Number(n) => n.to_string(),
    }
}

/// Compact JSON with recursively sorted keys, mirroring
/// `json.dumps(value, ensure_ascii=False, sort_keys=True)` (serde keeps
/// UTF-8 raw, matching `ensure_ascii=False`).
fn sorted_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap_or_default(),
                        sorted_json(&map[*key])
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(sorted_json).collect();
            format!("[{}]", parts.join(","))
        }
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// PARITY: `AIAgent._mask_api_key_for_logs` (pin). Falsy keys read as
/// `None`; 12 chars or fewer collapse to `"***"`; longer keys show
/// first-8/last-4 (`len` counts Unicode scalars as upstream counts code
/// points). The Azure-callable arm (`<entra-id-bearer>`) is
/// unrepresentable on `&str` — callables only exist as provider auth
/// surfaces at runtime; the loop slice owns that arm.
pub fn mask_api_key_for_logs(key: Option<&str>) -> Option<String> {
    let key = match key.filter(|k| !k.is_empty()) {
        Some(key) => key,
        None => return None,
    };
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 12 {
        return Some("***".to_string());
    }
    let head: String = chars.iter().take(8).collect();
    let tail: String = chars
        .iter()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    Some(format!("{head}...{tail}"))
}

/// PARITY: `AIAgent._clean_error_message` (pin). Empty reads as
/// `"Unknown error"`; HTML pages collapse to the fixed notice;
/// otherwise whitespace-collapsed and capped at 150 chars (Unicode
/// scalars, as upstream slices code points) with a `"..."` suffix.
pub fn clean_error_message(error_msg: &str) -> String {
    if error_msg.is_empty() {
        return "Unknown error".to_string();
    }
    if error_msg.trim().starts_with("<!DOCTYPE html") || error_msg.contains("<html") {
        return "Service temporarily unavailable (HTML error page returned)".to_string();
    }
    let cleaned: String = error_msg.split_whitespace().collect::<Vec<_>>().join(" ");
    let truncated: String = cleaned.chars().take(150).collect();
    if truncated.len() < cleaned.len() {
        return format!("{truncated}...");
    }
    truncated
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
