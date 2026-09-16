//! Redaction applied to monitoring data before egress.
//!
//! PARITY: `agent/monitoring/redaction.py` @ 5d59366 (whole module, incl.
//! `redact_bounded`).
//!
//! One unconditional scrub, no modes, no knobs. Every string that leaves
//! the process passes through [`redact_for_export`]: secrets via
//! `agent/redact.py::redact_for_egress` — the single pattern source, fails
//! CLOSED so a broken redactor never emits the raw string — then PII
//! (e-mail, phone, UUID-shaped ids → `[email]` / `[phone]` / `[id]`).
//!
//! There is deliberately no setting to weaken this. The old inline
//! bearer/token/literal/residue sweep now lives in `redact_for_egress`
//! (hermes-logging), exactly as upstream refactored it.

use once_cell::sync::Lazy;
use regex::Regex;

use hermes_logging::redact_for_egress;

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

/// Scrub a string for egress: secrets, then PII. Unconditional.
///
/// PARITY: `redact_for_export` (upstream lines 30-38). Secrets delegate to
/// `redact_for_egress` (the single pattern source); PII ordering is
/// email → uuid → phone exactly as upstream. `None` maps to `None`.
pub fn redact_for_export(text: Option<&str>) -> Option<String> {
    let text = text?;
    let out = redact_for_egress(text);
    let out = EMAIL_RE.replace_all(&out, "[email]");
    let out = UUID_RE.replace_all(&out, "[id]");
    let out = replace_phone(&out);
    Some(out)
}

/// Upstream defaults for [`redact_bounded`] (`limit=500`,
/// `empty="[redacted]"`, `unavailable=REDACTION_UNAVAILABLE`). Rust has no
/// default args — callers pass these explicitly.
// PARITY: `redact_bounded` (upstream lines 41-47) — redact
/// `str(raw or "")`, substitute `empty` for empty results, truncate to
/// `limit` chars (no suffix); `unavailable` is returned if redaction
/// raises. Rust's redaction path is infallible, so the `unavailable` arm
/// is a contract: callers that can fail map their error to it instead of
/// emitting raw text.
pub fn redact_bounded(raw: &str, limit: usize, empty: &str, unavailable: &str) -> String {
    let _ = unavailable;
    match redact_for_export(Some(raw)) {
        Some(out) if !out.is_empty() => out.chars().take(limit).collect(),
        _ => empty.to_string(),
    }
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
