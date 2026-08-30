//! Parity tests for `agent/reactions.py` @ b9aa928.
//!
//! Upstream has no dedicated test file for the detector (the
//! `tests/test_message_reactions.py` oracle covers the SessionDB tapback
//! store, already ported in hermes-state); these cases derive from the
//! upstream lexicon as oracle.

use hermes_agent::reactions::{detect_reaction, VIBE};

#[test]
fn affection_phrases_fire_vibe() {
    for text in [
        "good bot",
        "Good Bot!",
        "good   bot",
        "i love you",
        "I LOVE YOU",
        "i luv ya",
        "ily",
        "ILYSM",
        "thank you",
        "thanks",
        "THX",
        "tysm",
        "ty",
        "<3",
        "<333",
    ] {
        assert_eq!(detect_reaction(Some(text)), Some(VIBE), "input: {text}");
    }
}

#[test]
fn heart_emoji_fire_vibe() {
    for text in [
        "\u{2764}",
        "\u{2665}",
        "\u{1F970}",
        "\u{1F60D}",
        "\u{1F496}",
        "\u{1FA77}",
    ] {
        assert_eq!(detect_reaction(Some(text)), Some(VIBE), "input: {text}");
    }
    // Emoji inside a sentence.
    assert_eq!(detect_reaction(Some("ok that worked \u{2764}")), Some(VIBE));
}

#[test]
fn broken_heart_and_positive_sentiment_do_not_fire() {
    // `</3` must not match the `<3+` arm.
    assert_eq!(detect_reaction(Some("</3")), None);
    // We match affection specifically, not general positive sentiment.
    assert_eq!(detect_reaction(Some("this is great")), None);
    assert_eq!(detect_reaction(Some("hello")), None);
    assert_eq!(detect_reaction(Some("thanksbot")) == Some(VIBE), false);
}

#[test]
fn word_boundaries_apply() {
    // "goodbot" is a single word but still matches `good\s*bot` — the
    // pattern has no boundary between good and bot (verified against the
    // Python oracle).
    assert_eq!(detect_reaction(Some("goodbot")), Some(VIBE));
    // `\bty\b` needs boundaries: "empty" and "tyty" never fire.
    assert_eq!(detect_reaction(Some("empty")), None);
    assert_eq!(detect_reaction(Some("tyty")), None);
    // But a leading sentence does.
    assert_eq!(detect_reaction(Some("well, good bot!")), Some(VIBE));
}

#[test]
fn falsy_text_returns_none() {
    // Python `if not text: return None` — None and empty string.
    assert_eq!(detect_reaction(None), None);
    assert_eq!(detect_reaction(Some("")), None);
}

#[test]
fn vibe_constant_is_the_kind_string() {
    assert_eq!(VIBE, "vibe");
}
