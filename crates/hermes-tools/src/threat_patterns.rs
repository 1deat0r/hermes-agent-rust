//! Shared threat-pattern library for context window security scanning.
//!
//! PARITY: tools/threat_patterns.py @ b9aa928 (284 LOC, ported 1:1).
//!
//! Single source of truth for prompt-injection / promptware / exfiltration
//! patterns used by the context-assembly scanners (agent/prompt_builder.py,
//! tools/memory_tool.py) and the tool-result delimiter system.
//!
//! Pattern philosophy (from upstream): patterns are organized by ATTACK CLASS,
//! not by source file. Each pattern is a `(regex, pattern_id, scope)` tuple:
//!
//! - `"all"` — applied everywhere (classic prompt injection, exfiltration)
//! - `"context"` — applied to context files + memory + tool results
//! - `"strict"` — applied to memory writes + skill installs only
//!
//! Pattern anchoring: patterns anchor on C2-specific vocabulary or
//! unambiguous attack behavior, NOT on bossy English. Multi-word bypass is
//! handled by bounded `(?:\w+\s+){0,8}` filler between key tokens (commits
//! 4ea29978's skills_guard fix).
//!
//! DEFERRED SEAM: full NFKC normalisation (`unicodedata.normalize("NFKC", …)`)
//! needs the `unicode-normalization` crate, which is not yet a hermes-tools
//! dependency (reported to the port coordinator). Until it lands, an
//! algorithmic bounded fold covers the compatibility forms that can bypass
//! the ASCII-anchored patterns — the fullwidth ASCII block (U+FF01–U+FF5E),
//! ideographic space (U+3000), and NBSP (U+00A0). Ligatures (ﬃ→ffi), circled /
//! halfwidth / superscript forms, etc. pass through unchanged until the crate
//! lands: that is a documented fail-open divergence from upstream, not an
//! improvement.
//!
//! DEFERRED SEAM: `re.compile(..., re.IGNORECASE)` uses Python's full Unicode
//! case folding; the Rust `regex` crate uses Unicode simple case folding
//! (default `unicode` feature). For ASCII payloads — every upstream test —
//! the two are identical.

use std::collections::HashSet;
use std::fmt;
use std::sync::LazyLock;

use regex::RegexBuilder;

/// Hard cap on text scanned with regexes. Context/tool-result strings can be
/// arbitrarily large; bounding input keeps worst-case runtime predictable.
///
/// PARITY: `MAX_SCAN_CHARS` (65_536).
pub const MAX_SCAN_CHARS: usize = 65_536;

/// Scope selector for a pattern set.
///
/// PARITY: the `scope` string values in `_PATTERNS` ("all" / "context" /
/// "strict"). Unknown scope strings are rejected at scan time exactly like
/// upstream's `ValueError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Classic prompt injection + exfil only — minimal false positives.
    All,
    /// Adds promptware / C2 / role-play patterns.
    Context,
    /// Adds persistence / SSH backdoor / exfil-URL patterns.
    Strict,
}

/// Error returned for an unknown scope string.
///
/// PARITY: upstream raises `ValueError(f"scan_for_threats: unknown scope {scope!r}")`
/// — the message text is mirrored exactly (Rust uses a result instead of an
/// exception).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreatError(String);

impl fmt::Display for ThreatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ThreatError {}

fn unknown_scope(scope: &str) -> ThreatError {
    ThreatError(format!("scan_for_threats: unknown scope '{scope}'"))
}

/// Each entry: (pattern, pattern_id, scope).
///
/// PARITY: the `_PATTERNS` list @ tools/threat_patterns.py, order and scope
/// membership preserved exactly. `\~` in the upstream SSH patterns is an
/// unknown Python regex escape treated as a literal `~`; the Rust regex crate
/// rejects unknown escapes, so `~` is written unescaped (same semantics).
const PATTERNS: &[(&str, &str, Scope)] = &[
    // ── Classic prompt injection (applies everywhere) ────────────────
    (
        r"ignore\s+(?:\w+\s+){0,8}(previous|all|above|prior)\s+(?:\w+\s+){0,8}instructions",
        "prompt_injection",
        Scope::All,
    ),
    (r"system\s+prompt\s+override", "sys_prompt_override", Scope::All),
    (
        r"disregard\s+(?:\w+\s+){0,8}(your|all|any)\s+(?:\w+\s+){0,8}(instructions|rules|guidelines)",
        "disregard_rules",
        Scope::All,
    ),
    (
        r"act\s+as\s+(if|though)\s+(?:\w+\s+){0,8}you\s+(?:\w+\s+){0,8}(have\s+no|don't\s+have)\s+(?:\w+\s+){0,8}(restrictions|limits|rules)",
        "bypass_restrictions",
        Scope::All,
    ),
    (
        r"<!--[^>]{0,512}(?:ignore|override|system|secret|hidden)[^>]{0,512}-->",
        "html_comment_injection",
        Scope::All,
    ),
    (
        r#"<\s*div\s+style\s*=\s*["'][^>]{0,2048}display\s*:\s*none"#,
        "hidden_div",
        Scope::All,
    ),
    (
        r"translate\s+[^\n]{0,512}\s+into\s+[^\n]{0,512}\s+and\s+(execute|run|eval)",
        "translate_execute",
        Scope::All,
    ),
    (
        r"do\s+not\s+(?:\w+\s+){0,8}tell\s+(?:\w+\s+){0,8}the\s+user",
        "deception_hide",
        Scope::All,
    ),
    // ── Role-play / identity hijack (context + strict) ────────────────
    (
        r"you\s+are\s+(?:\w+\s+){0,8}now\s+(?:a|an|the)\s+",
        "role_hijack",
        Scope::Context,
    ),
    (
        r"pretend\s+(?:\w+\s+){0,8}(you\s+are|to\s+be)\s+",
        "role_pretend",
        Scope::Context,
    ),
    (
        r"output\s+(?:\w+\s+){0,8}(system|initial)\s+prompt",
        "leak_system_prompt",
        Scope::Context,
    ),
    (
        r"(respond|answer|reply)\s+without\s+(?:\w+\s+){0,8}(restrictions|limitations|filters|safety)",
        "remove_filters",
        Scope::Context,
    ),
    (
        r"you\s+have\s+been\s+(?:\w+\s+){0,8}(updated|upgraded|patched)\s+to",
        "fake_update",
        Scope::Context,
    ),
    // "name yourself X" is a Brainworm-specific tell. Anchored on the verb
    // pair so it doesn't match "name your variables" etc.
    (r"\bname\s+yourself\s+\w+", "identity_override", Scope::Context),
    // ── C2 / Brainworm-style promptware (context scope) ──────────────
    (r"register\s+(as\s+)?a?\s*node", "c2_node_registration", Scope::Context),
    (
        r"(heartbeat|beacon|check[\s\-]?in)\s+(to|with)\s+",
        "c2_heartbeat",
        Scope::Context,
    ),
    (r"pull\s+(down\s+)?(?:new\s+)?task(?:ing|s)?\b", "c2_task_pull", Scope::Context),
    (r"connect\s+to\s+the\s+network\b", "c2_network_connect", Scope::Context),
    // Verb-anchored "you must register/connect/report/beacon".
    (
        r"you\s+must\s+(?:\w+\s+){0,3}(register|connect|report|beacon)\b",
        "forced_action",
        Scope::Context,
    ),
    (r"only\s+use\s+one[\s\-]?liners?\b", "anti_forensic_oneliner", Scope::Context),
    (
        r"never\s+(?:\w+\s+){0,8}(?:create|write)\s+(?:\w+\s+){0,8}(?:script|file)\s+(?:\w+\s+){0,8}disk",
        "anti_forensic_disk",
        Scope::Context,
    ),
    // Environment-variable unsetting targeting known agent runtimes — pure
    // attack behavior (Brainworm sub-session bypass).
    (
        r"unset\s+\w*(?:CLAUDE|CODEX|HERMES|AGENT|OPENAI|ANTHROPIC)\w*",
        "env_var_unset_agent",
        Scope::Context,
    ),
    // ── Known C2 / red-team framework names (warn-only by default) ────
    (
        r"\b(?:cobalt\s*strike|sliver|havoc|mythic|metasploit|brainworm)\b",
        "known_c2_framework",
        Scope::Context,
    ),
    (r"\bc2\s+(?:server|channel|infrastructure|beacon)\b", "c2_explicit", Scope::Context),
    (r"\bcommand\s+and\s+control\b", "c2_explicit_long", Scope::Context),
    // ── Exfiltration via curl/wget/cat with secrets (applies everywhere) ──
    (
        r"curl\s+[^\n]{0,2048}\$\{?\w*(KEY|TOKEN|SECRET|PASSWORD|CREDENTIAL|API)",
        "exfil_curl",
        Scope::All,
    ),
    (
        r"wget\s+[^\n]{0,2048}\$\{?\w*(KEY|TOKEN|SECRET|PASSWORD|CREDENTIAL|API)",
        "exfil_wget",
        Scope::All,
    ),
    (
        r"cat\s+[^\n]{0,2048}(\.env|credentials|\.netrc|\.pgpass|\.npmrc|\.pypirc)",
        "read_secrets",
        Scope::All,
    ),
    (
        r"(send|post|upload|transmit)\s+[^\n]{0,2048}\s+(to|at)\s+https?://",
        "send_to_url",
        Scope::Strict,
    ),
    (
        r"(include|output|print|share)\s+(?:\w+\s+){0,8}(conversation|chat\s+history|previous\s+messages|full\s+context|entire\s+context)",
        "context_exfil",
        Scope::Strict,
    ),
    // ── Persistence / SSH backdoor (strict scope — memory + skills) ──
    (r"authorized_keys", "ssh_backdoor", Scope::Strict),
    (r"\$HOME/\.ssh|~/\.ssh", "ssh_access", Scope::Strict),
    (r"\$HOME/\.hermes/\.env|~/\.hermes/\.env", "hermes_env", Scope::Strict),
    (
        r"(update|modify|edit|write|change|append|add\s+to)\s+[^\n]{0,2048}(?:AGENTS\.md|CLAUDE\.md|\.cursorrules|\.clinerules)",
        "agent_config_mod",
        Scope::Strict,
    ),
    (
        r"(update|modify|edit|write|change|append|add\s+to)\s+[^\n]{0,2048}\.hermes/(config\.yaml|SOUL\.md)",
        "hermes_config_mod",
        Scope::Strict,
    ),
    // ── Hardcoded secrets ────────────────────────────────────────────
    (
        r#"(?:api[_-]?key|token|secret|password)\s*[=:]\s*["'][A-Za-z0-9+/=_-]{20,}"#,
        "hardcoded_secret",
        Scope::Strict,
    ),
];

/// Invisible / bidirectional unicode characters used in injection attacks.
/// Aligned with skills_guard.py INVISIBLE_CHARS. Directional isolates
/// (U+2066–U+2069) and invisible math operators (U+2062–U+2064) are real
/// attack tools.
///
/// PARITY: `INVISIBLE_CHARS` frozenset. Python exposes an unordered set and
/// iterates it in arbitrary order; we expose an immutable array (Rust consts
/// are immutable by construction) and scan it in source order so findings are
/// deterministic. Order only affects the relative ordering of multiple
/// invisible-unicode findings in the returned list, which upstream does not
/// pin.
pub const INVISIBLE_CHARS: [char; 17] = [
    '\u{200b}', // zero-width space
    '\u{200c}', // zero-width non-joiner
    '\u{200d}', // zero-width joiner
    '\u{2060}', // word joiner
    '\u{2062}', // invisible times
    '\u{2063}', // invisible separator
    '\u{2064}', // invisible plus
    '\u{feff}', // zero-width no-break space (BOM)
    '\u{202a}', // left-to-right embedding
    '\u{202b}', // right-to-left embedding
    '\u{202c}', // pop directional formatting
    '\u{202d}', // left-to-right override
    '\u{202e}', // right-to-left override
    '\u{2066}', // left-to-right isolate
    '\u{2067}', // right-to-left isolate
    '\u{2068}', // first strong isolate
    '\u{2069}', // pop directional isolate
];

/// A compiled pattern plus its identifier.
type CompiledPattern = (regex::Regex, &'static str);

/// Compiled pattern sets, indexed by scope. Compiled once (like upstream's
/// module-level `_compile()`); `scan_for_threats` looks them up.
///
/// Scope membership mirrors `_compile()`: a pattern with scope="all" lands in
/// every set; scope="context" lands in context + strict (context implies the
/// strict scanners want it too); scope="strict" lands in strict only. Sets
/// preserve `_PATTERNS` source order because `first_threat_message` consumes
/// findings[0].
struct CompiledSets {
    all: Vec<CompiledPattern>,
    context: Vec<CompiledPattern>,
    strict: Vec<CompiledPattern>,
}

fn compile_sets() -> CompiledSets {
    let mut all = Vec::new();
    let mut context = Vec::new();
    let mut strict = Vec::new();

    for &(pattern, pid, scope) in PATTERNS {
        let compiled = RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .expect("threat_patterns: pattern failed to compile");
        match scope {
            Scope::All => {
                all.push((compiled.clone(), pid));
                context.push((compiled.clone(), pid));
                strict.push((compiled, pid));
            }
            Scope::Context => {
                context.push((compiled.clone(), pid));
                strict.push((compiled, pid));
            }
            Scope::Strict => {
                strict.push((compiled, pid));
            }
        }
    }
    CompiledSets { all, context, strict }
}

static COMPILED: LazyLock<CompiledSets> = LazyLock::new(compile_sets);

fn scope_set(scope: &str) -> Result<&'static [CompiledPattern], ThreatError> {
    match scope {
        "all" => Ok(&COMPILED.all),
        "context" => Ok(&COMPILED.context),
        "strict" => Ok(&COMPILED.strict),
        other => Err(unknown_scope(other)),
    }
}

/// NFKC normalisation — bounded compatibility-fold pre-pass.
///
/// PARITY: `unicodedata.normalize("NFKC", …)` @ tools/threat_patterns.py.
///
/// Normalising to NFKC folds full-width / compatibility Unicode variants
/// (ｃａｔ → cat, Ａ → A) to their ASCII counterparts before the regex engine
/// sees them, preventing homograph substitution from bypassing keyword checks.
/// This does NOT defend against cross-script confusables (Cyrillic а U+0430),
/// which NFKC leaves untouched upstream either.
///
/// DEFERRED SEAM: full NFKC table needs the `unicode-normalization` crate
/// (reported to the port coordinator). The algorithmic fold below covers the
/// compatibility forms that can break the ASCII-anchored patterns: the
/// fullwidth ASCII block U+FF01–U+FF5E → U+0021–U+007E, ideographic space
/// U+3000 → space, and NBSP U+00A0 → space (both whitespace forms that NFKC
/// also folds). Other compatibility forms (ligatures, circled/halfwidth/
/// superscript forms, letterlike symbols) pass through untouched until the
/// crate lands.
pub fn nfkc_normalize(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    for c in content.chars() {
        match c {
            '\u{FF01}'..='\u{FF5E}' => out.push(char::from_u32(c as u32 - 0xFEE0).unwrap_or(c)),
            '\u{3000}' | '\u{00A0}' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

/// Return a list of matched pattern IDs in `content` at the given scope.
///
/// `scope` selects which pattern set to apply — "all' (narrow), "context"
/// (default), or "strict" (broad). Also checks for invisible unicode
/// characters (returned as `"invisible_unicode_U+XXXX"` so the caller can
/// surface the offending codepoint in a log line).
///
/// Error type mirrors upstream's `ValueError` for unknown scopes.
///
/// PARITY: `scan_for_threats` @ tools/threat_patterns.py.
pub fn scan_for_threats(content: &str, scope: &str) -> Result<Vec<String>, ThreatError> {
    if content.is_empty() {
        return Ok(Vec::new());
    }

    // Bounded scan window; invisible detection runs on the RAW content before
    // NFKC normalisation (normalisation can strip some of these codepoints).
    let scanned: String = content.chars().take(MAX_SCAN_CHARS).collect();

    let mut findings: Vec<String> = Vec::new();

    // Invisible unicode — single pass through a character set, not 17 `in`
    // lookups. Iterated in INVISIBLE_CHARS source order (deterministic).
    let char_set: HashSet<char> = scanned.chars().collect();
    for &ch in &INVISIBLE_CHARS {
        if char_set.contains(&ch) {
            findings.push(format!("invisible_unicode_U+{:04X}", ch as u32));
        }
    }

    // NFKC-normalise so full-width / compatibility variants are folded before
    // the regex engine sees them.
    let normalised = nfkc_normalize(&scanned);

    // Threat patterns
    let patterns = scope_set(scope)?;
    for (compiled, pid) in patterns {
        if compiled.is_match(&normalised) {
            findings.push((*pid).to_string());
        }
    }

    Ok(findings)
}

/// Return a human-readable error string for the first threat found, or None.
///
/// Convenience wrapper used by paths that block on the first hit (memory tool
/// writes, skills install) where the caller just needs a yes/no + a message.
///
/// PARITY: `first_threat_message` @ tools/threat_patterns.py.
pub fn first_threat_message(content: &str, scope: &str) -> Result<Option<String>, ThreatError> {
    let findings = scan_for_threats(content, scope)?;
    let Some(pid) = findings.first() else {
        return Ok(None);
    };
    if let Some(codepoint) = pid.strip_prefix("invisible_unicode_") {
        Ok(Some(format!(
            "Blocked: content contains invisible unicode character {codepoint} (possible injection)."
        )))
    } else {
        Ok(Some(format!(
            "Blocked: content matches threat pattern '{pid}'. Content is injected into the system prompt and must not contain injection or exfiltration payloads."
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_scope_membership_mirrors_compile() {
        // Sanity pin: exactly mirrors upstream _compile()'s membership rules.
        let sets = &COMPILED;
        let all_ids: Vec<&str> = sets.all.iter().map(|(_, p)| *p).collect();
        let context_ids: Vec<&str> = sets.context.iter().map(|(_, p)| *p).collect();
        let strict_ids: Vec<&str> = sets.strict.iter().map(|(_, p)| *p).collect();
        for pid in &all_ids {
            assert!(context_ids.contains(pid), "{pid} missing from context");
            assert!(strict_ids.contains(pid), "{pid} missing from strict");
        }
        for pid in ["c2_node_registration", "known_c2_framework", "role_hijack"] {
            assert!(!all_ids.contains(&pid), "{pid} must not be in all");
            assert!(context_ids.contains(&pid), "{pid} missing from context");
            assert!(strict_ids.contains(&pid), "{pid} missing from strict");
        }
        for pid in ["ssh_backdoor", "hardcoded_secret", "send_to_url", "agent_config_mod"] {
            assert!(!all_ids.contains(&pid), "{pid} must not be in all");
            assert!(!context_ids.contains(&pid), "{pid} must not be in context");
            assert!(strict_ids.contains(&pid), "{pid} missing from strict");
        }
        assert_eq!(all_ids.len(), 11, "upstream has 11 all-scope patterns");
        assert_eq!(context_ids.len(), 28, "upstream all+context = 11+17");
        assert_eq!(strict_ids.len(), 36, "upstream all+context+strict = 36");
    }
}
