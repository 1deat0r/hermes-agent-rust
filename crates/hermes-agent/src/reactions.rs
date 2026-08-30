//! Token-free detection of user *reactions* to the agent.
//!
//! PARITY: `agent/reactions.py` @ b9aa928 (whole module).
//!
//! Currently the only reaction is `vibe` — an expression of affection or
//! gratitude toward the agent (`ily`, `<3`, `love you`, `good bot`, a heart
//! emoji, …). Detection is a curated regex/lexicon: **no model call, no
//! tokens**.
//!
//! This is the single source of truth shared by every surface — the CLI
//! pet, the TUI heart, and the desktop floating hearts all react off the
//! same signal, delivered via `AIAgent.reaction_callback` (wired per
//! interactive host).
//!
//! Generalized on purpose: [`detect_reaction`] returns a reaction *kind*
//! string, so new kinds (other emoji reactions, etc.) can be added here
//! without touching any caller. We match affection specifically — not
//! general positive sentiment — so "this is great" does NOT fire, but
//! "good bot" / "❤️" do.

use once_cell::sync::Lazy;
use regex::Regex;

/// The affection/gratitude reaction — the only kind today.
///
/// PARITY: `VIBE` (upstream line 16).
pub const VIBE: &str = "vibe";

/// Curated affection lexicon. Kept deliberately narrow: gratitude + love
/// aimed at the agent, heart emoji, and `<3` (but not the broken heart
/// `</3`).
///
/// PARITY: `_VIBE_RE` (upstream lines 19-39), byte-for-byte the same
/// alternatives with `re.IGNORECASE`. `\b` and `\w` are unicode-aware in
/// the `regex` crate exactly like Python's default.
static VIBE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(\bgood\s*bot\b)|(\bi\s*(?:love|luv)\s*(?:you|u|ya)\b)|(\b(?:love|luv)\s*(?:you|u|ya)\b)|(\bily(?:sm)?\b)|(\bthank\s*(?:you|u)\b)|(\b(?:thanks|thx|tysm|ty)\b)|(<3+)|([\u{2764}\u{2665}\u{1F970}\u{1F60D}\u{1F618}\u{1F495}\u{1F496}\u{1F497}\u{1F49E}\u{1F49B}\u{1F49C}\u{1F49A}\u{1F499}\u{1F493}\u{1F498}\u{1F49D}\u{1FA77}])",
    )
    .expect("vibe re")
});

/// Return the reaction kind for `text` (currently [`VIBE`]), or `None`.
///
/// Pure, token-free, and safe to call on every user turn. Python
/// `if not text` makes `None` and the empty string return `None`.
///
/// PARITY: `detect_reaction` (upstream lines 42-47).
pub fn detect_reaction(text: Option<&str>) -> Option<&'static str> {
    let text = match text {
        Some(text) if !text.is_empty() => text,
        _ => return None,
    };
    if VIBE_RE.is_match(text) {
        Some(VIBE)
    } else {
        None
    }
}
