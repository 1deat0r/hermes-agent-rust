//! Redaction applied to monitoring data before egress.
//!
//! PARITY: `agent/monitoring/redaction.py` @ b9aa928 (whole module).
//!
//! One unconditional scrub, no modes, no knobs. Every string that leaves
//! the process passes through [`redact_for_export`]:
//!
//! * Secrets first — wraps `agent/redact.py::redact_sensitive_text`
//!   with `force = true` (upstream `force=True` so user config can't
//!   disable it) plus bearer/token-shape patterns, and fails CLOSED: if
//!   the redactor cannot run, the raw string is never emitted.
//! * PII second — e-mail addresses, phone numbers, and UUID-shaped
//!   identifiers are rewritten to `[email]` / `[phone]` / `[id]`.
//!
//! There is deliberately no setting to weaken this.

use once_cell::sync::Lazy;
use regex::Regex;

use hermes_logging::redact_sensitive_text;

// Secret shapes (belt-and-suspenders on top of agent/redact.py).

/// PARITY: `_BEARER_RE` (upstream line 24) — `re.IGNORECASE`.
static BEARER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._~+\-/]+=*").expect("bearer re"));

/// PARITY: `_TOKEN_RE` (upstream lines 25-27).
static TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(xox[baprs]-[A-Za-z0-9-]+|sk-[A-Za-z0-9_-]{8,}|gh[pousr]_[A-Za-z0-9_]{8,})\b")
        .expect("token re")
});

/// PARITY: `_SECRET_LITERAL_RE` (upstream line 28).
static SECRET_LITERAL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\*{3,}").expect("literal re"));

/// PARITY: `_BEARER_RESIDUE_RE` (upstream line 29).
static BEARER_RESIDUE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\bBearer\s+\[[^\]]+\]").expect("bearer residue re"));

// PII shapes.

/// PARITY: `_EMAIL_RE` (upstream line 32).
static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}").expect("email re")
});

/// E.164-ish and common separators; conservative to avoid nuking code/IDs.
/// PARITY: `_PHONE_RE` (upstream lines 33-35). Rust `regex` has no
/// lookbehind, so the `(?<!\w)` / `(?!\w)` guards become boundary captures.
static PHONE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(^|[^\w])(\+?\d{1,3}[\s.\-]?)?(\(\d{2,4}\)[\s.\-]?)?\d{3}[\s.\-]?\d{3,4}([\s.\-]?\d{2,4})?($|[^\w])",
    )
    .expect("phone re")
});

/// Long opaque hex/uuid-ish user identifiers.
/// PARITY: `_UUID_RE` (upstream lines 36-38).
static UUID_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b")
        .expect("uuid re")
});

/// Always-on secret redaction. `force = true` so user config can't disable
/// it.
///
/// PARITY: `_secret_redact` (upstream lines 41-54). The import-failure arm
/// can't occur here (the redactor is a compile-time dependency), but the
/// fail-CLOSED contract is preserved: the input is never returned raw on a
/// redaction error path.
fn secret_redact(text: &str) -> String {
    let out = redact_sensitive_text(text, true, false, false, false);
    let out = BEARER_RE.replace_all(&out, "[redacted]");
    let out = TOKEN_RE.replace_all(&out, "[redacted]");
    let out = SECRET_LITERAL_RE.replace_all(&out, "[redacted]");
    BEARER_RESIDUE_RE
        .replace_all(&out, "[redacted]")
        .into_owned()
}

/// Scrub a string for egress: secrets, then PII. Unconditional.
///
/// PARITY: `redact_for_export` (upstream lines 57-65). `None` maps to
/// `None` (the `if text is None` arm); PII ordering is email → uuid →
/// phone exactly as upstream.
pub fn redact_for_export(text: Option<&str>) -> Option<String> {
    let text = text?;
    let out = secret_redact(text);
    let out = EMAIL_RE.replace_all(&out, "[email]");
    let out = UUID_RE.replace_all(&out, "[id]");
    let out = replace_phone(&out);
    Some(out)
}

/// `_PHONE_RE.sub("[phone]", ...)` with the lookaround guards: the
/// boundary groups are restored so overlapping separators stay intact.
fn replace_phone(text: &str) -> String {
    PHONE_RE
        .replace_all(text, |caps: &regex::Captures| {
            let leading = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let trailing = caps.get(caps.len() - 1).map(|m| m.as_str()).unwrap_or("");
            format!("{leading}[phone]{trailing}")
        })
        .into_owned()
}
