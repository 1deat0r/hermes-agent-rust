//! First ported section of `run_agent.py`: module-level pure helpers.
//!
//! PARITY: `run_agent.py` @ b9aa928 (pin lines ~234-371: scaffolding flags,
//! worker/marker constants, Qwen header builders, session filename
//! sanitizer, RouterMint UA). Everything here is dependency-free stdlib
//! logic; higher-layer seams stay out:
//! - `_routermint_headers` reads `hermes_cli.__version__` lazily upstream;
//!   `hermes-agent` must not depend on the higher-layer `hermes-cli`
//!   crate, so the version arrives as an explicit argument.
//!
//! Evidence tiers: `unit` for cases mirroring upstream tests
//! (`tests/run_agent/test_dropped_tool_call_recovery.py`,
//! `tests/run_agent/test_run_agent.py`,
//! `tests/agent/test_verification_stop_caching.py`,
//! `tests/agent/test_gemini_fast_fallback.py`,
//! `tests/run_agent/test_provider_fallback.py` @ b9aa928);
//! `unit/source-derived` where no upstream case pins the behavior
//! (header platform mapping, falsy-flag matrix, truncation bounds,
//! single-entry pool).

use super::credential_pool::CredentialPool;
use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

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
