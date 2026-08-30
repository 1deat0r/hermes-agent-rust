//! Default SOUL.md template seeded into HERMES_HOME on first run.
//!
//! PARITY: `hermes_cli/default_soul.py` @ b9aa928 (whole module).
//!
//! The legacy-template table is a safety guarantee: these scaffolds carry
//! zero user intent (pure comment scaffolding, no persona text), so a
//! SOUL.md matching one of them was demonstrably never customized and is
//! safe to upgrade to [`DEFAULT_SOUL_MD`] in place. Any deviation — the
//! user typed a persona, even one character outside the comment — makes
//! [`is_legacy_template_soul`] return false.

/// PARITY: `DEFAULT_SOUL_MD` (upstream lines 5-13).
pub const DEFAULT_SOUL_MD: &str = "You are Hermes Agent, an intelligent AI assistant created by Nous Research. You are helpful, knowledgeable, and direct. You assist users with a wide range of tasks including answering questions, writing and editing code, analyzing information, creative work, and executing actions via your tools. You communicate clearly, admit uncertainty when appropriate, and prioritize being genuinely useful over being verbose unless otherwise directed below. Be targeted and efficient in your exploration and investigations.";

/// PARITY: `_LEGACY_TEMPLATE_SOULS` (upstream lines 19-67) — the two
/// comment-only scaffolds (with and without the "Examples" block) shipped
/// by install.sh / install.ps1 / docker/SOUL.md.
const LEGACY_TEMPLATE_SOULS: [&str; 2] = [
    "# Hermes Agent Persona\n\n<!--\nThis file defines the agent's personality and tone.\nThe agent will embody whatever you write here.\nEdit this to customize how Hermes communicates with you.\n\nExamples:\n  - \"You are a warm, playful assistant who uses kaomoji occasionally.\"\n  - \"You are a concise technical expert. No fluff, just facts.\"\n  - \"You speak like a friendly coworker who happens to know everything.\"\n\nThis file is loaded fresh each message -- no restart needed.\nDelete the contents (or this file) to use the default personality.\n-->",
    "# Hermes Agent Persona\n\n<!--\nThis file defines the agent's personality and tone.\nThe agent will embody whatever you write here.\nEdit this to customize how Hermes communicates with you.\n\nThis file is loaded fresh each message -- no restart needed.\nDelete the contents (or this file) to use the default personality.\n-->",
];

/// Normalize SOUL.md content for legacy-template comparison.
///
/// PARITY: `_normalize_soul` (upstream lines 70-75) — unify line endings
/// (CRLF and bare CR → LF), strip a leading UTF-8 BOM, trim surrounding
/// whitespace.
fn normalize_soul(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim_start_matches('\u{feff}')
        .trim()
        .to_string()
}

/// True if `text` is an old empty-template SOUL.md (no user persona).
///
/// A file matching one of the known scaffolds is safe to upgrade in place;
/// any deviation (the user typed a persona, even one character outside the
/// comment) makes this return false.
///
/// PARITY: `is_legacy_template_soul` (upstream lines 78-90).
pub fn is_legacy_template_soul(text: &str) -> bool {
    let normalized = normalize_soul(text);
    LEGACY_TEMPLATE_SOULS
        .iter()
        .any(|template| normalized == normalize_soul(template))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_soul_is_the_nous_research_persona() {
        assert!(DEFAULT_SOUL_MD.starts_with("You are Hermes Agent,"));
        assert!(DEFAULT_SOUL_MD.contains("Nous Research"));
        assert!(!DEFAULT_SOUL_MD.contains("<!--"));
    }

    #[test]
    fn legacy_scaffolds_with_and_without_examples_match() {
        assert!(is_legacy_template_soul(LEGACY_TEMPLATE_SOULS[0]));
        assert!(is_legacy_template_soul(LEGACY_TEMPLATE_SOULS[1]));
        // A truncated Examples block is neither scaffold — it is user
        // content (any deviation defeats the match).
        let truncated = "# Hermes Agent Persona\n\n<!--\nThis file defines the agent's personality and tone.\nEdit this to customize how Hermes communicates with you.\n\nExamples:\n  - \"You are a warm, playful assistant who uses kaomoji occasionally.\"\n\nThis file is loaded fresh each message -- no restart needed.\nDelete the contents (or this file) to use the default personality.\n-->";
        assert!(!is_legacy_template_soul(truncated));
    }

    #[test]
    fn normalization_handles_crlf_bom_and_whitespace() {
        // CRLF from Windows installers must not defeat the comparison.
        let crlf = LEGACY_TEMPLATE_SOULS[0].replace('\n', "\r\n");
        assert!(is_legacy_template_soul(&crlf));
        // Leading BOM.
        let bom = format!("\u{feff}{}", LEGACY_TEMPLATE_SOULS[1]);
        assert!(is_legacy_template_soul(&bom));
        // Surrounding whitespace.
        let padded = format!("  {}\n\n", LEGACY_TEMPLATE_SOULS[1]);
        assert!(is_legacy_template_soul(&padded));
    }

    #[test]
    fn any_user_persona_defeats_the_match() {
        assert!(!is_legacy_template_soul(DEFAULT_SOUL_MD));
        // One character outside the comment is user intent.
        let edited = format!("Be brief.\n{}", LEGACY_TEMPLATE_SOULS[1]);
        assert!(!is_legacy_template_soul(&edited));
        // An edited comment inside the scaffold too.
        let edited_comment =
            LEGACY_TEMPLATE_SOULS[1].replace("no restart needed", "no restart needed at all");
        assert!(!is_legacy_template_soul(&edited_comment));
    }
}
