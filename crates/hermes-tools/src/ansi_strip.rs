//! Strip ANSI escape sequences from subprocess / persisted output.
//!
//! PARITY: tools/ansi_strip.py @ b9aa928 (79 LOC, ported 1:1).

use once_cell::sync::Lazy;
use regex::Regex;

const ANSI_ESCAPE: &str = r"\x1b(?:\[[\x30-\x3f]*[\x20-\x2f]*[\x40-\x7e]|\][\s\S]*?(?:\x07|\x1b\\)|[PX^_][\s\S]*?(?:\x1b\\)|[\x20-\x2f]+[\x30-\x7e]|[\x30-\x7e])|\x9b[\x30-\x3f]*[\x20-\x2f]*[\x40-\x7e]|\x9d[\s\S]*?(?:\x07|\x9c)|[\x80-\x9f]";

static ANSI_ESCAPE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(ANSI_ESCAPE).expect("ansi escape re")
});
static HAS_ESCAPE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\x1b\x80-\x9f]").expect("has escape"));
static CONTROL_CHARS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]").expect("control re")
});
static HAS_CONTROL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\x00-\x08\x0b-\x1f\x7f-\x9f]").expect("has control")
});

/// Remove ANSI escape sequences from text (fast path when clean).
pub fn strip_ansi(text: &str) -> String {
    if text.is_empty() || !HAS_ESCAPE.is_match(text) {
        return text.to_string();
    }
    ANSI_ESCAPE_RE.replace_all(text, "").into_owned()
}

/// Sanitize stored/untrusted text before echoing it to a terminal: strips
/// ANSI escapes AND bare control characters, preserving only newlines and
/// tabs (CR normalized to newline).
pub fn sanitize_display_text(text: &str) -> String {
    if text.is_empty() || !HAS_CONTROL.is_match(text) {
        return text.to_string();
    }
    let text = strip_ansi(text);
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    CONTROL_CHARS_RE.replace_all(&text, "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_csi_sequences() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
    }

    #[test]
    fn fast_path_clean_text_passthrough() {
        let t = "plain text";
        assert_eq!(strip_ansi(t), t);
    }

    #[test]
    fn sanitizes_control_chars_and_normalizes_cr() {
        assert_eq!(sanitize_display_text("a\x00b"), "ab");
        assert_eq!(sanitize_display_text("a\r\nb"), "a\nb");
        assert_eq!(sanitize_display_text("a\rb"), "a\nb");
        assert_eq!(sanitize_display_text("\x1b[1mB\x1b[0m"), "B");
    }
}
