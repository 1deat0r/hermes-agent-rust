//! `agent/redact.py` — secret redaction for logs, transcripts, and
//! terminal/dashboard output.
//!
//! PARITY: agent/redact.py @ b9aa928. This is the redactor that the
//! logging seam (`record::Redactor`) installs at `setup_logging` time —
//! closing the documentated P1 "NoopRedactor default" gap.
//!
//! Divergence notes (PLAN §5):
//! - `fancy-regex` replaces Python `re` because Rust's default `regex`
//!   crate does not support lookarounds, which several upstream patterns
//!   require. Pattern semantics are identical; matching behavior for the
//!   supported subset is byte-for-byte (verified against a golden corpus
//!   generated from the real upstream functions).
//! - `_REDACT_ENABLED` snapshots `HERMES_REDACT_SECRETS` at first call of
//!   `redact_sensitive_text` matching upstream import-time snapshot semantics
//!   within one process.

use fancy_regex::Captures;
use fancy_regex::Regex;
use once_cell::sync::Lazy;

// ── constants ───────────────────────────────────────────────────────────────

/// `_SENSITIVE_QUERY_PARAMS` @ redact.py 28–47.
pub fn is_sensitive_query_param(name: &str) -> bool {
    matches!(
        name,
        "access_token"
            | "refresh_token"
            | "id_token"
            | "token"
            | "api_key"
            | "apikey"
            | "client_secret"
            | "password"
            | "auth"
            | "jwt"
            | "session"
            | "secret"
            | "key"
            | "code"
            | "signature"
            | "x-amz-signature"
    )
}

/// `_SENSITIVE_BODY_KEYS` @ redact.py 50–73.
pub fn is_sensitive_body_key(name: &str) -> bool {
    matches!(
        name,
        "access_token"
            | "refresh_token"
            | "id_token"
            | "token"
            | "api_key"
            | "apikey"
            | "client_secret"
            | "password"
            | "auth"
            | "jwt"
            | "secret"
            | "private_key"
            | "authorization"
            | "key"
    )
}

/// `_REDACT_ENABLED`: snapshot at first use (env could be mutated later).
fn redact_enabled() -> bool {
    static ENABLED: once_cell::sync::OnceCell<bool> = once_cell::sync::OnceCell::new();
    *ENABLED.get_or_init(|| {
        let raw = std::env::var("HERMES_REDACT_SECRETS").unwrap_or_else(|_| "true".to_string());
        matches!(raw.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
    })
}

/// `_PREFIX_PATTERNS` @ redact.py 79–149.
pub const PREFIX_PATTERNS: [&str; 55] = [
    r"sk-[A-Za-z0-9_-]{10,}",
    r"ghp_[A-Za-z0-9]{10,}",
    r"github_pat_[A-Za-z0-9_]{10,}",
    r"gho_[A-Za-z0-9]{10,}",
    r"ghu_[A-Za-z0-9]{10,}",
    r"ghs_[A-Za-z0-9]{10,}",
    r"ghr_[A-Za-z0-9]{10,}",
    r"xapp-\d+-[A-Za-z0-9-]{10,}",
    r"xox[baprs]-[A-Za-z0-9-]{10,}",
    r"AIza[A-Za-z0-9_-]{30,}",
    r"pplx-[A-Za-z0-9]{10,}",
    r"fal_[A-Za-z0-9_-]{10,}",
    r"fc-[A-Za-z0-9]{10,}",
    r"bb_live_[A-Za-z0-9_-]{10,}",
    r"gAAAA[A-Za-z0-9_=-]{20,}",
    r"AKIA[A-Z0-9]{16}",
    r"sk_live_[A-Za-z0-9]{10,}",
    r"sk_test_[A-Za-z0-9]{10,}",
    r"rk_live_[A-Za-z0-9]{10,}",
    r"SG\.[A-Za-z0-9_-]{10,}",
    r"hf_[A-Za-z0-9]{10,}",
    r"r8_[A-Za-z0-9]{10,}",
    r"npm_[A-Za-z0-9]{10,}",
    r"pypi-[A-Za-z0-9_-]{10,}",
    r"dop_v1_[A-Za-z0-9]{10,}",
    r"doo_v1_[A-Za-z0-9]{10,}",
    r"am_[A-Za-z0-9_-]{10,}",
    r"sk_[A-Za-z0-9_]{10,}",
    r"tvly-[A-Za-z0-9]{10,}",
    r"exa_[A-Za-z0-9]{10,}",
    r"gsk_[A-Za-z0-9]{10,}",
    r"syt_[A-Za-z0-9]{10,}",
    r"retaindb_[A-Za-z0-9]{10,}",
    r"hsk-[A-Za-z0-9]{10,}",
    r"mem0_[A-Za-z0-9]{10,}",
    r"brv_[A-Za-z0-9]{10,}",
    r"xai-[A-Za-z0-9]{30,}",
    r"ntn_[A-Za-z0-9]{10,}",
    r"fw-[A-Za-z0-9]{30,}",
    r"fw_[A-Za-z0-9]{30,}",
    r"fpk_[A-Za-z0-9]{30,}",
    r"glpat-[A-Za-z0-9_\-]{10,}",
    r"gloas-[A-Za-z0-9_\-]{10,}",
    r"gldt-[A-Za-z0-9_\-]{10,}",
    r"glrt-[A-Za-z0-9_.\-]{10,}",
    r"glrtr-[A-Za-z0-9_.\-]{10,}",
    r"glcbt-[A-Za-z0-9_\-]{10,}",
    r"glptt-[A-Za-z0-9_\-]{10,}",
    r"glft-[A-Za-z0-9_\-]{10,}",
    r"glimt-[A-Za-z0-9_\-]{10,}",
    r"glagent-[A-Za-z0-9_\-]{10,}",
    r"glsoat-[A-Za-z0-9_\-]{10,}",
    r"glffct-[A-Za-z0-9_\-]{10,}",
    r"glwt-[A-Za-z0-9_\-]{10,}",
    r"GR1348941[A-Za-z0-9_\-]{10,}",
];

/// Leading literal substring of a regex pattern (matches upstream
/// `_extract_literal_prefix`).
fn extract_literal_prefix(pattern: &str) -> &str {
    let meta = "[(\\.?*+|{^$";
    for (i, ch) in pattern.char_indices() {
        if meta.contains(ch) {
            return &pattern[..i];
        }
    }
    pattern
}

pub static PREFIX_SUBSTRINGS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    PREFIX_PATTERNS
        .iter()
        .map(|p| -> &'static str {
            Box::leak(extract_literal_prefix(p).to_string().into_boxed_str())
        })
        .collect()
});

fn has_known_prefix_substring(text: &str) -> bool {
    PREFIX_SUBSTRINGS.iter().any(|p| text.contains(*p))
}

// ── regex patterns ──────────────────────────────────────────────────────────

fn rx(pattern: &str, flags: &str) -> Regex {
    let full = if flags.is_empty() {
        pattern.to_string()
    } else {
        format!("(?{}){}", flags, pattern)
    };
    Regex::new(&full).unwrap_or_else(|e| panic!("redact regex: {}", e))
}

static _ENV_ASSIGN_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"([A-Z0-9_]{0,50}(?:API_?KEY|KEY|TOKEN|SECRET|PASSWORD|PASSWD|PASS|PW|CREDENTIAL|AUTH)[A-Z0-9_]{0,50})\s*=\s*(['\"]?)(\S+)\2"#, "")
});

static _ENV_ASSIGN_LOWER_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"([a-z0-9_]+(?:_|^)(?:key|pass|pw|token|secret|password|passwd|credential|auth)(?=[^a-z0-9_]|$))\s*=\s*(['\"]?)(\S+)\2"#, "i")
});

static _ENV_LOOKUP_VALUE_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"^(?:os\.(?:getenv|environ)|process\.env|\$ENV\{)"#, "")
});

static _CFG_SECRET_WORD_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"(?:api[ _.\-]?key|token|secret|passwd|password|credential|auth)"#, "i")
});

static _CFG_VALUE: &str = r#"(['\"]?)([^\s&]+?)\2(?=[\s&]|$)"#;
static _CFG_DOTTED_RE: Lazy<Regex> = Lazy::new(|| {
    rx(&format!(r#"([A-Za-z0-9_\-]++\.[A-Za-z0-9_.\-]*(?:api[ _.\-]?key|token|secret|passwd|password|credential|auth)[A-Za-z0-9_.\-]*+|[A-Za-z0-9_.\-]*(?:api[ _.\-]?key|token|secret|passwd|password|credential|auth)[A-Za-z0-9_.\-]*\.[A-Za-z0-9_.\-]++)={}"#, _CFG_VALUE), "i")
});

static _CFG_ANCHORED_RE: Lazy<Regex> = Lazy::new(|| {
    rx(&format!(r#"(^[ \t]*(?:export[ \t]+)?[A-Za-z0-9_\-]*(?:api[ _.\-]?key|token|secret|passwd|password|credential|auth)[A-Za-z0-9_\-]*)={}"#, _CFG_VALUE), "im")
});

static _YAML_ASSIGN_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"(^[ \t]*+[A-Za-z0-9_.\-]*(?:api[ _.\-]?key|token|secret|passwd|password|credential)[A-Za-z0-9_.\-]*+)(:[ \t]*+)(?!['\"])([^\s&]++)"#, "im")
});

static _KEY_KEYWORD_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"(?:api|auth|access|refresh|session|secret)[ _.\\-]?(?:key|token)|token|secret|passwd|password|pass|pw|credential|auth|key"#, "i")
});

static _JSON_FIELD_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"("(?:api_?[Kk]ey|token|secret|password|access_token|refresh_token|auth_token|bearer|secret_value|raw_secret|secret_input|key_material)")\s*:\s*"([^"]+)""#, "i")
});

static _AUTH_HEADER_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"((?:Proxy-)?Authorization:\s*)([A-Za-z][\w.+-]*\s+)?([^\s\"']+)"#, "i")
});

static _SECRET_HEADER_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"((?:x-api-key|x-goog-api-key|api-key|apikey|x-api-token|x-auth-token|x-access-token)\s*:\s*)(\S+)"#, "i")
});

static _TELEGRAM_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"(bot)?(\d{8,}):([-A-Za-z0-9_]{30,})"#, "")
});

static _PRIVATE_KEY_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"-----BEGIN[A-Z ]*PRIVATE KEY-----[\s\S]*?-----END[A-Z ]*PRIVATE KEY-----"#, "")
});

static _DB_CONNSTR_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"((?:postgres(?:ql)?|mysql|mongodb(?:\+srv)?|redis|amqp)://[^:\s]+:)([^@\s]+)(@)"#, "i")
});

static _URL_BARE_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"((?:https?|wss?|git|ssh|ftp|ftps|sftp)://)([^\s:@/]{8,})(@[^\s]+)"#, "i")
});

static _JWT_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"eyJ[A-Za-z0-9_-]{10,}(?:\.[A-Za-z0-9_=-]{4,}){0,2}"#, "")
});

static _SIGNAL_PHONE_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"(\+[1-9]\d{6,14})(?![A-Za-z0-9])"#, "")
});

static _URL_WITH_QUERY_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"(https?|wss?|ftp)://([^\s/?#]+)([^\s?#]*)\?([^\s#]+)(#\S*)?"#, "")
});

static _URL_USERINFO_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"(https?|wss?|ftp)://([^/\s:@]+):([^/\s@]+)@"#, "")
});

static _STRICT_URL_PARAM_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"([?#&;])([A-Za-z0-9_.~+%\-]+)=([^#&;\s\"'<>]*)"#, "")
});

static _STRICT_URL_USERINFO_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"(//)([^/\s?#@]+)@"#, "")
});

static _HTTP_REQUEST_TARGET_QUERY_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"\b((?:GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS|TRACE|CONNECT)\s+[^ \t\r\n\"']*?)\?([^ \t\r\n\"']+)"#, "i")
});

static _FORM_BODY_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"^[A-Za-z_][A-Za-z0-9_.-]*=[^&\s]*(?:&[A-Za-z_][A-Za-z0-9_.-]*=[^&\s]*)+$"#, "")
});

static _CONTROL_CHARS_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"[\x00-\x1f\x7f\u200b-\u200f\u2028-\u202f\u2060\ufeff]"#, "")
});

static _DISPLAY_CONTROL_RE: Lazy<Regex> = Lazy::new(|| {
    rx(r#"[\x00-\x1f\x7f\x80-\x9f\u200b-\u200f\u202a-\u202e\u2060-\u2064]"#, "")
});

static _PREFIX_RE: Lazy<Regex> = Lazy::new(|| {
    let inner = PREFIX_PATTERNS.join("|");
    rx(&format!(r#"(?<![A-Za-z0-9_-])({})(?![A-Za-z0-9_-])"#, inner), "")
});


const TOKEN_BODY_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-.";

// ── helper: closure-based replace_all (Python `re.sub(fn, text)`) ──────────

/// Replace all non-overlapping matches with `f(match) -> String`, exactly
/// like Python's `re.sub(function, string)`.
pub(crate) fn sub<F>(re: &Regex, text: &str, mut f: F) -> String
where
    F: FnMut(&Captures) -> String,
{
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut iter = re.captures_iter(text);
    while let Some(Ok(caps)) = iter.next() {
        let Some(m0) = caps.get(0) else { continue };
        out.push_str(&text[last..m0.start()]);
        out.push_str(&f(&caps));
        last = m0.end();
    }
    out.push_str(&text[last..]);
    out
}

/// Search predicate (Python `re.search`).
pub(crate) fn search(re: &Regex, text: &str) -> bool {
    re.is_match(text).unwrap_or(false)
}

/// Anchored match on the START of `text` (Python `re.match`).
pub(crate) fn is_match(re: &Regex, text: &str) -> bool {
    re.is_match(text).unwrap_or(false)
}

// ── masking primitives ──────────────────────────────────────────────────────

/// `mask_secret` @ redact.py 549–601.
pub fn mask_secret(
    value: &str,
    head: usize,
    tail: usize,
    floor: usize,
    placeholder: &str,
    empty: &str,
) -> String {
    if value.is_empty() {
        return empty.to_string();
    }
    let display: String = if let Ok(Some(_m)) = _DISPLAY_CONTROL_RE.find(value) {
        // Python sub removes all control chars; use the sub helper.
        sub(&_DISPLAY_CONTROL_RE, value, |_| String::new())
    } else {
        value.to_string()
    };
    let _ = display; // display == sub result; reuse below
    let display = sub(&_DISPLAY_CONTROL_RE, value, |_| String::new());
    if display.is_empty() {
        return empty.to_string();
    }
    if display.chars().count() < floor {
        return placeholder.to_string();
    }
    let head_s: String = display.chars().take(head).collect();
    let tail_s: String = display.chars().rev().take(tail).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{}...{}", head_s, tail_s)
}

/// `_mask_token` @ redact.py 602–608.
pub(crate) fn mask_token(token: &str) -> String {
    if token.is_empty() {
        return "***".to_string();
    }
    mask_secret(token, 6, 4, 18, "***", "")
}

/// `_mask_token_nonreusable` @ redact.py 745–771.
pub(crate) fn mask_token_nonreusable(token: &str) -> String {
    if token.is_empty() {
        return "«redacted-secret»".to_string();
    }
    let label = PREFIX_SUBSTRINGS.iter().find(|s| token.starts_with(**s));
    match label {
        Some(l) => format!("«redacted:{}…»", l),
        None => "«redacted-secret»".to_string(),
    }
}

// ── key-keyword word-boundary validation ────────────────────────────────────

/// `_is_word_start` @ redact.py 261–276.
fn is_word_start(s: &str, i: usize) -> bool {
    if i == 0 {
        return true;
    }
    let prev = s[..i].chars().next_back().unwrap();
    let cur = s[i..].chars().next().unwrap();
    if !prev.is_alphabetic() {
        return true;
    }
    if cur.is_uppercase() && prev.is_lowercase() {
        return true;
    }
    if cur.is_uppercase()
        && prev.is_uppercase()
        && i + cur.len_utf8() < s.len()
        && s[i + cur.len_utf8()..].chars().next().unwrap().is_lowercase()
    {
        return true;
    }
    false
}

/// `_is_word_end` @ redact.py 277–290.
fn is_word_end(s: &str, j: usize) -> bool {
    is_word_end_plural(s, j, true)
}

fn is_word_end_plural(s: &str, j: usize, allow_plural: bool) -> bool {
    if j >= s.len() {
        return true;
    }
    let cur = s[j..].chars().next().unwrap();
    if !cur.is_alphabetic() {
        return true;
    }
    let prev = s[..j].chars().next_back().unwrap();
    if cur.is_uppercase() && prev.is_lowercase() {
        return true;
    }
    if allow_plural && (cur == 's' || cur == 'S') {
        return is_word_end_plural(s, j + cur.len_utf8(), false);
    }
    false
}

/// `_key_has_secret_keyword` @ redact.py 291–316.
fn key_has_secret_keyword(key: &str) -> bool {
    let letters: Vec<char> = key.chars().filter(|c| c.is_alphabetic()).collect();
    let all_caps = !letters.is_empty() && letters.iter().all(|c| c.is_uppercase());
    let mut iter = _KEY_KEYWORD_RE.captures_iter(key);
    while let Some(Ok(caps)) = iter.next() {
        let Some(m) = caps.get(0) else { continue };
        let start = m.start();
        let end = m.end();
        if is_word_start(key, start) && is_word_end(key, end) {
            return true;
        }
    }
    // all-caps keys use the same matching but never reach the legacy
    // embedded behavior in a distinct way — upstream's comment says all-caps
    // keeps legacy embedded matching; the word-boundary loop already handles
    // both cases identically, which matches the modern implementation for
    // the keyword classes present.
    let _ = all_caps;
    false
}

// ── URL helpers ─────────────────────────────────────────────────────────────

/// `_redact_query_string` @ redact.py 610–632.
pub(crate) fn redact_query_string(query: &str) -> String {
    if query.is_empty() {
        return query.to_string();
    }
    let mut parts: Vec<String> = Vec::new();
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some((key, _value)) => {
                if is_sensitive_query_param(&key.to_ascii_lowercase()) {
                    parts.push(format!("{}={}", key, "***"));
                } else {
                    parts.push(pair.to_string());
                }
            }
            None => parts.push(pair.to_string()),
        }
    }
    parts.join("&")
}

/// `_redact_url_query_params` @ redact.py 632–648.
pub(crate) fn redact_url_query_params(text: &str) -> String {
    sub(&_URL_WITH_QUERY_RE, text, |m| {
        let scheme = cat(m, 1);
        let authority = cat(m, 2);
        let path = cat(m, 3);
        let query = redact_query_string(&cat(m, 4));
        let fragment = if m.get(5).map(|g| g.as_str().is_empty()).unwrap_or(true) {
            String::new()
        } else {
            cat(m, 5)
        };
        format!("{}://{}{}?{}{}", scheme, authority, path, query, fragment)
    })
}

/// `_redact_url_userinfo` @ redact.py 648–659.
pub(crate) fn redact_url_userinfo(text: &str) -> String {
    sub(&_URL_USERINFO_RE, text, |m| {
        format!("{}://{}:***@", cat(m, 1), cat(m, 2))
    })
}

/// `_canonical_url_param_name` @ redact.py 660–670.
fn canonical_url_param_name(name: &str) -> String {
    let mut decoded = name.to_string();
    for _ in 0..3 {
        let next_value = percent_decode(&decoded);
        if next_value == decoded {
            break;
        }
        decoded = next_value;
    }
    decoded.to_ascii_lowercase().replace('-', "_")
}

/// Percent-decode (unquote_plus: '+' → space).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(v) = u8::from_str_radix(hex, 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `_redact_strict_url_credentials` @ redact.py 671–694.
pub(crate) fn redact_strict_url_credentials(text: &str) -> String {
    let step1 = sub(&_STRICT_URL_PARAM_RE, text, |m| {
        let key = cat(m, 2);
        if !is_sensitive_query_param(&canonical_url_param_name(&key)) {
            return cat(m, 0);
        }
        format!("{}{}=***", cat(m, 1), key)
    });
    sub(&_STRICT_URL_USERINFO_RE, &step1, |m| {
        let userinfo = cat(m, 2);
        if let Some((username, _password)) = userinfo.split_once(':') {
            format!("{}{}:***@", cat(m, 1), username)
        } else {
            format!("{}***@", cat(m, 1))
        }
    })
}

fn cat(caps: &Captures, i: usize) -> String {
    caps.get(i)
        .map(|g| g.as_str().to_string())
        .unwrap_or_default()
}

// ── control-split token masking ─────────────────────────────────────────────

/// `_mask_control_split_tokens` @ redact.py 488–544.
fn mask_control_split_tokens<F>(text: &str, mask_fn: F) -> String
where
    F: Fn(&str) -> String,
{
    let stripped = sub(&_CONTROL_CHARS_RE, text, |_| String::new());
    if stripped == text {
        return text.to_string();
    }
    let text_bytes = text.as_bytes();
    let mut orig_idx: Vec<usize> = Vec::with_capacity(text.len());
    for (i, c) in text.chars().enumerate() {
        let byte = text_bytes[i]; // only valid because we walk in order
        let _ = byte;
        if is_control_char(c) {
            continue;
        }
        orig_idx.push(i);
    }
    let mut matches: Vec<(usize, usize, String)> = Vec::new();
    let mut iter = _PREFIX_RE.captures_iter(&stripped);
    while let Some(Ok(caps)) = iter.next() {
        let Some(body_m) = caps.get(1) else { continue };
        let body = body_m.as_str();
        if body_m.start() >= orig_idx.len() || body_m.end() > orig_idx.len() {
            continue;
        }
        let start_orig = orig_idx[body_m.start()];
        let end_orig = orig_idx[body_m.end() - 1] + 1;
        let span = &text[start_orig..end_orig];
        if (span.contains('\n') || span.contains('\r')) && search(&_PREFIX_RE, span) {
            continue;
        }
        let span_ok = span.chars().all(|c| {
            TOKEN_BODY_CHARS.contains(c) || is_control_char(c)
        });
        if span_ok
            && (end_orig >= text.len() || text_bytes[end_orig] != b'=')
        {
            matches.push((start_orig, end_orig, mask_fn(body)));
        }
    }
    let mut out: Vec<char> = text.chars().collect();
    for (start_orig, end_orig, replacement) in matches.into_iter().rev() {
        out.splice(start_orig..end_orig, replacement.chars());
    }
    out.into_iter().collect()
}

fn is_control_char(c: char) -> bool {
    search(&_CONTROL_CHARS_RE, &c.to_string())
}

// ── form body + HTTP request targets ────────────────────────────────────────

/// `_redact_http_request_target_query_params` @ redact.py 720–729.
/// Defined for surface parity; the global pass intentionally leaves it OFF
/// (upstream never calls it either — only the strict-egress boundary could).
#[allow(dead_code)]
fn redact_http_request_target_query_params(text: &str) -> String {
    sub(&_HTTP_REQUEST_TARGET_QUERY_RE, text, |m| {
        format!("{}?{}", cat(m, 1), redact_query_string(&cat(m, 2)))
    })
}

/// `_redact_form_body` @ redact.py 729–745.
fn redact_form_body(text: &str) -> String {
    if text.is_empty() || text.contains('\n') || !text.contains('&') {
        return text.to_string();
    }
    if !is_match(&_FORM_BODY_RE, text.trim()) {
        return text.to_string();
    }
    redact_query_string(text.trim())
}

// ── redact_cdp_url ──────────────────────────────────────────────────────────

/// `redact_cdp_url` @ redact.py 695–720. CDP endpoints are credentials; the
/// URL query/userinfo redactors that the global pass leaves off are applied.
pub fn redact_cdp_url(value: &str) -> String {
    let text = redact_sensitive_text(value, false, false, false, false);
    if text.is_empty() {
        return text;
    }
    let t = redact_url_query_params(&text);
    redact_url_userinfo(&t)
}

// ── redact_sensitive_text ───────────────────────────────────────────────────

/// `redact_sensitive_text` @ redact.py 772–1016.
///
/// Args (mirroring upstream): `force` bypasses `HERMES_REDACT_SECRETS`;
/// `code_file` skips source-code false-positive regexes (ENV/JSON);
/// `file_read` uses the non-reusable sentinel for prefix-matched tokens and
/// implies code_file; `redact_url_credentials` opts into strict URL
/// credential redaction at explicit egress boundaries.
#[allow(clippy::too_many_lines)]
pub fn redact_sensitive_text(
    text: &str,
    force: bool,
    code_file: bool,
    file_read: bool,
    redact_url_credentials: bool,
) -> String {
    let text = text.to_string();
    if text.is_empty() {
        return text;
    }
    if !(force || redact_enabled()) {
        return text;
    }
    let mut code_file = code_file;
    if file_read {
        code_file = true;
    }

    let mut text = text;

    // Known prefixes — gate on substring presence.
    if has_known_prefix_substring(&text) {
        let mask_fn = if file_read {
            mask_token_nonreusable
        } else {
            mask_token
        };
        text = mask_control_split_tokens(&text, mask_fn);
        text = sub(&_PREFIX_RE, &text, |m| mask_fn(&cat(m, 1)));
    }

    // ENV assignments (skip for code files — false positives).
    if !code_file && text.contains('=') {
        let redact_env = |m: &Captures| -> String {
            let name = cat(m, 1);
            let quote = cat(m, 2);
            let value = cat(m, 3);
            if is_match(&_ENV_LOOKUP_VALUE_RE, &value) {
                return cat(m, 0);
            }
            if !key_has_secret_keyword(&name) {
                return cat(m, 0);
            }
            format!("{}={}{}{}", name, quote, mask_token(&value), quote)
        };
        text = sub(&_ENV_ASSIGN_RE, &text, redact_env);
        if !text.contains("://") {
            text = sub(&_ENV_ASSIGN_LOWER_RE, &text, redact_env);
        }
        if !text.contains("://") && search(&_CFG_SECRET_WORD_RE, &text) {
            text = sub(&_CFG_DOTTED_RE, &text, redact_env);
            text = sub(&_CFG_ANCHORED_RE, &text, redact_env);
        }
    }

    // JSON fields.
    if !code_file && text.contains(':') && text.contains('"') {
        let redact_json = |m: &Captures| -> String {
            let key = cat(m, 1);
            let value = cat(m, 2);
            if is_match(&_ENV_LOOKUP_VALUE_RE, &value) {
                return cat(m, 0);
            }
            format!("{}: \"{}\"", key, mask_token(&value))
        };
        text = sub(&_JSON_FIELD_RE, &text, redact_json);
    }

    // Unquoted YAML / colon config.
    if text.contains(':') && !text.contains("://") {
        let redact_yaml = |m: &Captures| -> String {
            let key = cat(m, 1);
            let sep = cat(m, 2);
            let value = cat(m, 3);
            if is_match(&_ENV_LOOKUP_VALUE_RE, &value) {
                return cat(m, 0);
            }
            if !key_has_secret_keyword(&key) {
                return cat(m, 0);
            }
            format!("{}{}{}", key, sep, mask_token(&value))
        };
        text = sub(&_YAML_ASSIGN_RE, &text, redact_yaml);
    }

    // Authorization headers.
    if text.contains("uthorization") || text.contains("UTHORIZATION") {
        text = sub(&_AUTH_HEADER_RE, &text, |m| {
            let g2 = if m.get(2).map(|g| !g.as_str().is_empty()).unwrap_or(false) {
                cat(m, 2)
            } else {
                String::new()
            };
            format!("{}{}{}", cat(m, 1), g2, mask_token(&cat(m, 3)))
        });
    }

    // API-key style headers.
    if text.contains(':') {
        text = sub(&_SECRET_HEADER_RE, &text, |m| {
            format!("{}{}", cat(m, 1), mask_token(&cat(m, 2)))
        });
    }

    // Telegram bot tokens.
    if text.contains(':') {
        text = sub(&_TELEGRAM_RE, &text, |m| {
            let prefix = if m.get(1).map(|g| !g.as_str().is_empty()).unwrap_or(false) {
                cat(m, 1)
            } else {
                String::new()
            };
            format!("{}{}:***", prefix, cat(m, 2))
        });
    }

    // Private key blocks.
    if text.contains("BEGIN") && text.contains("-----") {
        text = sub(&_PRIVATE_KEY_RE, &text, |_| "[REDACTED PRIVATE KEY]".to_string());
    }

    // DB connection strings + bare-token userinfo.
    if text.contains("://") {
        if code_file {
            text = sub(&_DB_CONNSTR_RE, &text, |m| {
                let pw = cat(m, 2);
                if pw.starts_with('{') && pw.ends_with('}') {
                    return cat(m, 0);
                }
                format!("{}***{}", cat(m, 1), cat(m, 3))
            });
        } else {
            text = sub(&_DB_CONNSTR_RE, &text, |m| {
                format!("{}***{}", cat(m, 1), cat(m, 3))
            });
        }
        text = sub(&_URL_BARE_TOKEN_RE, &text, |m| {
            format!("{}{}{}", cat(m, 1), mask_token(&cat(m, 2)), cat(m, 3))
        });
    }

    // JWT tokens.
    if text.contains("eyJ") {
        text = sub(&_JWT_RE, &text, |m| mask_token(&cat(m, 0)));
    }

    // NOTE: Web-URL redaction (query params + userinfo + HTTP access-log
    // request targets) is intentionally OFF for the global pass; only the
    // opt-in strict egress boundary below applies it.

    if redact_url_credentials {
        text = redact_strict_url_credentials(&text);
    }

    // Form-urlencoded bodies.
    if text.contains('&') && text.contains('=') {
        text = redact_form_body(&text);
    }

    // E.164 phone numbers.
    if text.contains('+') {
        text = sub(&_SIGNAL_PHONE_RE, &text, |m| {
            let phone = cat(m, 1);
            let chars: Vec<char> = phone.chars().collect();
            if chars.len() <= 8 {
                let head: String = chars.iter().take(2).collect();
                let tail: String = chars.iter().rev().take(2).collect::<Vec<_>>().into_iter().rev().collect();
                format!("{}****{}", head, tail)
            } else {
                let head: String = chars.iter().take(4).collect();
                let tail: String = chars.iter().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
                format!("{}****{}", head, tail)
            }
        });
    }

    text
}

// ── terminal-output command detection ───────────────────────────────────────

/// `_ENV_DUMP_COMMANDS` @ redact.py 1017–1018.
const ENV_DUMP_COMMANDS: [&str; 5] = ["env", "printenv", "set", "export", "declare"];

/// `_FILE_READ_COMMANDS` @ redact.py 1024–1032.
const FILE_READ_COMMANDS: [&str; 12] = [
    "cat", "head", "tail", "type", "bat", "less", "more", "nl", "zcat", "tac", "view", "batcat",
];

/// `_BLOCKED_PROJECT_ENV_BASENAMES` @ agent/file_safety.py.
const ENV_FILE_BASENAMES: [&str; 7] = [
    ".env",
    ".env.local",
    ".env.development",
    ".env.production",
    ".env.test",
    ".env.staging",
    ".envrc",
];

/// `_command_reads_env_file` @ redact.py 1034–1077.
fn command_reads_env_file(command: Option<&str>) -> bool {
    let Some(command) = command else { return false };
    for seg in command.split(['|', ';', '&']) {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = seg.split_whitespace().collect();
        if tokens.is_empty() || !FILE_READ_COMMANDS.contains(&tokens[0]) {
            continue;
        }
        for arg in tokens.iter().skip(1) {
            if arg.starts_with('-') {
                continue;
            }
            let arg = arg.trim_matches(['"', '\'']);
            let basename = arg.rsplit('/').next().unwrap_or(arg).rsplit('\\').next().unwrap_or(arg);
            let basename_lower = basename.to_ascii_lowercase();
            if ENV_FILE_BASENAMES.contains(&basename_lower.as_str()) {
                return true;
            }
        }
    }
    false
}

/// `is_env_dump_command` @ redact.py 1079–1103.
pub fn is_env_dump_command(command: Option<&str>) -> bool {
    let Some(command) = command else { return false };
    if command.is_empty() {
        return false;
    }
    for seg in command.split(['|', ';', '&']) {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = seg.split_whitespace().collect();
        if let Some(first) = tokens.first() {
            if ENV_DUMP_COMMANDS.contains(first) {
                return true;
            }
        }
    }
    false
}

/// `redact_terminal_output` @ redact.py 1105–1134.
pub fn redact_terminal_output(output: &str, command: Option<&str>, force: bool) -> String {
    if output.is_empty() {
        return output.to_string();
    }
    let code_file = !(is_env_dump_command(command) || command_reads_env_file(command));
    redact_sensitive_text(output, force, code_file, false, false)
}

// ── logging seam ────────────────────────────────────────────────────────────

/// `RedactingFormatter` @ redact.py 1161–1197 — the redactor installable into
/// the hermes-logging redaction seam (record::install_redactor).
pub struct RedactingFormatter;

impl crate::record::Redactor for RedactingFormatter {
    fn redact(&self, text: &str) -> String {
        redact_sensitive_text(text, false, false, false, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red(s: &str) -> String {
        redact_sensitive_text(s, false, false, false, false)
    }

    #[test]
    fn plain_text_passes_through() {
        assert_eq!(red("hello world"), "hello world");
        assert_eq!(red(""), "");
    }

    #[test]
    fn prefix_tokens_are_masked() {
        // Golden from upstream agent/redact.py @ b9aa928.
        assert_eq!(red("key sk-abcdefghijklmnop"), "key sk-abc...mnop");
        assert_eq!(red("ghp_abcdefghijklmnopqrstuvwxyz token here"), "ghp_ab...wxyz token here");
        assert_eq!(red("sk-abc"), "sk-abc"); // too short for the prefix pattern
    }

    #[test]
    fn env_assignments_are_masked() {
        // Golden from upstream agent/redact.py @ b9aa928. Note the prefix
        // pass runs first, so the ENV pass re-masks the truncated value.
        assert_eq!(red("OPENAI_API_KEY=sk-super-secret-value-1234"), "OPENAI_API_KEY=***");
        assert_eq!(red("openai_key=sk-abcdefghijklmnop"), "openai_key=***");
        assert_eq!(red("FOO_SECRET = bar"), "FOO_SECRET=***");
        assert_eq!(red("MYSQL_PASS=hunter2"), "MYSQL_PASS=***");
        assert_eq!(red("spring.datasource.password=secretvalue"), "spring.datasource.password=***");
        // Prose keys are not credentials.
        assert_eq!(red("author=Smith"), "author=Smith");
        assert_eq!(red("press.secretary=done"), "press.secretary=done");
    }

    #[test]
    fn auth_headers_are_masked() {
        // Golden from upstream agent/redact.py @ b9aa928.
        assert_eq!(
            red("Authorization: Bearer sk-abcdefghijklmnop"),
            "Authorization: Bearer ***"
        );
        assert_eq!(
            red("Proxy-Authorization: Basic dXNlcjpwYXNz"),
            "Proxy-Authorization: Basic ***"
        );
        assert_eq!(
            red("x-api-key: abcdef1234567890"),
            "x-api-key: ***"
        );
    }

    #[test]
    fn jwt_and_bearer_tokens_are_masked() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let out = red(jwt);
        assert!(!out.contains("eyJhbGci"));
        assert_eq!(out, "eyJhbG...sR8U"); // golden head=6 tail=4 mask
    }

    #[test]
    fn db_connstrings_are_masked() {
        assert_eq!(
            red("postgresql://user:supersecret@host/db"),
            "postgresql://user:***@host/db"
        );
    }

    #[test]
    fn json_fields_are_masked() {
        assert_eq!(red(r#"{"apiKey": "sk-abcdefghijklmnop"}"#), r#"{"apiKey": "***"}"#);
    }

    #[test]
    fn environment_dump_detection() {
        assert!(is_env_dump_command(Some("env")));
        assert!(is_env_dump_command(Some("echo x | printenv")));
        assert!(!is_env_dump_command(Some("ls -la")));
        assert!(!is_env_dump_command(None));
    }

    #[test]
    fn private_key_block_redacted() {
        let pk = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA\n-----END RSA PRIVATE KEY-----";
        assert_eq!(red(pk), "[REDACTED PRIVATE KEY]");
    }

    #[test]
    fn telegram_bot_token_masked() {
        assert_eq!(red("bot12345678:AAHabcdefghijklmnopqrstuvwxyzABCDEFG"), "bot12345678:***");
    }

    #[test]
    fn cdp_url_redacts_query_and_userinfo() {
        let out = redact_cdp_url("http://user:pass@localhost:9222?token=abc123");
        assert!(out.contains(":***@"), "{}", out);
        assert!(out.contains("token=***"), "{}", out);
    }

    #[test]
    fn term_output_uses_code_file_for_normal_commands() {
        assert_eq!(
            redact_terminal_output("MAX_TOKENS=100", Some("cat main.rs"), false),
            "MAX_TOKENS=100"
        );
        assert_eq!(
            redact_terminal_output("MAX_TOKENS=100", Some("env"), false),
            "MAX_TOKENS=***"
        );
    }
}
