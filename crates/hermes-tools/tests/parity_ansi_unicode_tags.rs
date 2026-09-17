//! Parity tests for `strip_unicode_tags` (`tools/ansi_strip.py` @ 5d59366).
//!
//! Oracle: live Python outputs at the pin. Plane-14 TAG chars (U+E0000–
//! U+E007F) are an invisible smuggling channel and strip; emoji tag
//! sequences (TR51 black-flag base + spec + CANCEL TAG) survive. BMP PUA
//! lookalikes (e.g. U+E0069) are NOT tags and stay.

use hermes_tools::ansi_strip::strip_unicode_tags;

/// Fast paths: empty and tag-free text pass through.
#[test]
fn clean_text_passes_through() {
    assert_eq!(strip_unicode_tags(""), "");
    assert_eq!(strip_unicode_tags("plain text"), "plain text");
}

/// Plane-14 tags strip (live oracle).
#[test]
fn plane14_tags_strip() {
    assert_eq!(strip_unicode_tags("a\u{E007F}b"), "ab");
    assert_eq!(strip_unicode_tags("\u{E0000}\u{E007F}"), "");
}

/// Emoji tag sequences survive intact (live oracle: Scotland flag).
#[test]
fn emoji_tag_sequences_survive() {
    let flag = "\u{1F3F4}\u{E0067}\u{E0062}\u{E007F}";
    assert_eq!(strip_unicode_tags(flag), flag);
}

/// BMP PUA lookalikes are not tags (live oracle: untouched). NOTE: these are
/// U+E069 etc. (4-hex-digit BMP PUA), NOT U+E0069 (plane-14 tag, stripped).
#[test]
fn bmp_pua_lookalikes_stay() {
    let text = "hide\u{E069}\u{E067}\u{E06E}here";
    assert_eq!(strip_unicode_tags(text), text);
}
