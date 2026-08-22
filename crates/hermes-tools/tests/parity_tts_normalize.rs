//! Parity oracles for TTS text normalization, mirroring upstream
//! tests/tools/test_tts_text_normalize.py + tests/tools/test_tts_prepare_spoken.py
//! (share-cleaner wiring cases excluded — they need gateway/tts_tool) @ b9aa928.
//!
//! In addition to the explicit upstream-mirror tests below, every corpus case
//! in `upstream/golden_tts_text_normalize.json` is compared stage-by-stage
//! (strip_nonspoken_blocks → strip_markdown_for_tts → normalize_symbols_for_tts
//! → smooth_whitespace_for_tts → flatten_newlines_for_payload → final payload).
//! The golden file is generated from the upstream module, never invented.

use hermes_tools::tts_text_normalize::{
    flatten_newlines_for_payload, normalize_symbols_for_tts, prepare_spoken_text,
    smooth_whitespace_for_tts, strip_markdown_for_tts, strip_nonspoken_blocks,
};

// Upstream raw strings use real `<` / `>` characters; we write them as \u{3c}/\u{3e}
// escapes so an HTML-interpreting renderer can never corrupt the literals.

#[test]
fn expands_celsius_and_weather_units() {
    let raw = "## Christchurch today\n\n- **Now:** about **14°C**, feels like **14°C**\n- **Wind:** 9 km/h\n- **Rain:** 1.3 mm\n- **Range:** 11\u{2013}17°C\n";
    let spoken = prepare_spoken_text(raw, None);
    assert!(!spoken.contains("##"));
    assert!(!spoken.contains("**"));
    assert!(spoken.contains("14 degrees Celsius"));
    assert!(spoken.contains("11 to 17 degrees Celsius"));
    assert!(spoken.contains("9 kilometres per hour"));
    assert!(spoken.contains("1.3 millimetres"));
    assert!(!spoken.contains("°C"));
    assert!(!spoken.contains("km/h"));
}

#[test]
fn polish_edge_cases() {
    assert_eq!(
        prepare_spoken_text("## Weather\nIt will be sunny", None),
        "Weather, It will be sunny."
    );
    assert!(prepare_spoken_text("measured in °C", None).contains("degrees Celsius"));
    // Trailing comma not swallowed into the amount.
    assert!(prepare_spoken_text("US$300, next", None).contains("300 US dollars"));
    // Numeric rates expand; and/or, N/A, dates stay intact.
    assert!(prepare_spoken_text("$5/month", None).contains("5 dollars per month"));
    assert!(prepare_spoken_text("choose and/or option", None).contains("and/or"));
    assert!(prepare_spoken_text("status N/A here", None).contains("N/A"));
    assert!(prepare_spoken_text("due 2026/06/02 ok", None).contains("2026/06/02"));
}

#[test]
fn think_block_removed() {
    // "<think>secret reasoning here</think>"
    let raw = "\u{3c}think\u{3e}\nsecret reasoning here\n\u{3c}/think\u{3e}\nThe answer is 42.";
    let spoken = prepare_spoken_text(raw, None);
    assert!(!spoken.contains("secret reasoning"));
    assert!(spoken.contains("42"));
}

#[test]
fn think_block_with_attributes_removed() {
    let raw = "\u{3c}think budget=high\u{3e}chain of thought\u{3c}/think\u{3e}Visible.";
    let spoken = prepare_spoken_text(raw, None);
    assert!(!spoken.contains("chain of thought"));
    assert!(spoken.contains("Visible"));
}

#[test]
fn unterminated_think_block_removed() {
    let raw = "Answer first.  \u{3c}think\u{3e}\ntruncated reasoning stream";
    let spoken = prepare_spoken_text(raw, None);
    assert!(!spoken.contains("truncated reasoning"));
    assert!(spoken.contains("Answer first"));
}

#[test]
fn multiple_think_blocks() {
    let raw = "\u{3c}think\u{3e}a\u{3c}/think\u{3e}one\u{3c}think\u{3e}b\u{3c}/think\u{3e} two";
    let spoken = strip_nonspoken_blocks(raw);
    assert!(spoken.contains("one") && spoken.contains("two"));
}

#[test]
fn verifier_footer_removed() {
    let footer = "⚠️ File-mutation verifier: 2 file(s) were NOT modified this turn despite any wording above that may suggest otherwise. Run `git status` or `read_file` to confirm.\n  • `tools/foo.py` — [patch] old_string not found\n  • `bar.md` — [write_file] failed";
    let raw = format!("I fixed the file.\n\n{footer}");
    let spoken = prepare_spoken_text(&raw, None);
    assert!(!spoken.contains("File-mutation verifier"));
    assert!(!spoken.contains("NOT modified"));
    assert!(spoken.contains("fixed the file"));
}

#[test]
fn text_without_footer_untouched() {
    let raw = "Just a normal reply about files.";
    assert_eq!(strip_nonspoken_blocks(raw).trim(), raw);
}

#[test]
fn emoji_removed() {
    let spoken = prepare_spoken_text("Done! 🎉🚀 All tests pass ✅", None);
    assert!(!spoken.contains('🎉'));
    assert!(!spoken.contains('🚀'));
    assert!(!spoken.contains('✅'));
    assert!(spoken.contains("All tests pass"));
}

#[test]
fn no_newlines_in_output() {
    let raw = "First line\nSecond line\n\nThird paragraph";
    let spoken = prepare_spoken_text(raw, None);
    assert!(!spoken.contains('\n'));
    assert!(spoken.contains("First line"));
    assert!(spoken.contains("Third paragraph"));
}

#[test]
fn existing_punctuation_not_doubled() {
    let out = flatten_newlines_for_payload("Alpha.\nBeta!");
    assert!(!out.contains(".."));
    assert!(out.contains("Alpha.") && out.contains("Beta!"));
}

#[test]
fn markdown_strip_preserves_readable_words() {
    let cleaned = strip_markdown_for_tts("**Loud** and clear 🎉");
    assert!(!cleaned.contains("**"));
    let cleaned = normalize_symbols_for_tts(&cleaned);
    assert!(cleaned.contains("Loud and clear"));
}

#[test]
fn max_chars_truncates_and_trims() {
    let long = "A very long sentence that goes on and on and on and on and on and on and on and on and on and on and on and on and on and on and on.";
    let spoken = prepare_spoken_text(long, Some(40));
    assert!(spoken.chars().count() <= 40);
    assert_eq!(
        spoken,
        "A very long sentence that goes on and on"
    );
}

#[test]
fn golden_corpus_stage_parity() {
    // Every corpus case must match the upstream module byte-for-byte at every stage.
    let corpus: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("../../upstream/golden_tts_text_normalize.json").unwrap())
            .expect("golden corpus");
    let cases = corpus.as_object().expect("object");
    assert_eq!(cases.len(), 36, "golden corpus case count drift");
    let mut checked = 0;
    for (name, val) in cases {
        let v = val.as_object().expect("case");
        let raw = b64_decode(v["raw"].as_str().expect("raw"));
        let strip = b64_decode(v["strip_nonspoken_blocks"].as_str().unwrap());
        assert_eq!(strip_nonspoken_blocks(&raw), strip, "strip_nonspoken_blocks mismatch for {name}");
        let md = b64_decode(v["strip_markdown_for_tts"].as_str().unwrap());
        assert_eq!(strip_markdown_for_tts(&strip), md, "strip_markdown_for_tts mismatch for {name}");
        let norm = b64_decode(v["normalize_symbols_for_tts"].as_str().unwrap());
        assert_eq!(normalize_symbols_for_tts(&md), norm, "normalize_symbols_for_tts mismatch for {name}");
        let smooth = b64_decode(v["smooth_whitespace_for_tts"].as_str().unwrap());
        assert_eq!(smooth_whitespace_for_tts(&norm), smooth, "smooth_whitespace_for_tts mismatch for {name}");
        let flat = b64_decode(v["flatten_newlines_for_payload"].as_str().unwrap());
        assert_eq!(flatten_newlines_for_payload(&smooth), flat, "flatten_newlines_for_payload mismatch for {name}");
        let prepared = b64_decode(v["prepare_spoken_text"].as_str().unwrap());
        assert_eq!(prepare_spoken_text(&raw, None), prepared, "prepare_spoken_text mismatch for {name}");
        let prepared40 = b64_decode(v["prepare_spoken_text_40"].as_str().unwrap());
        assert_eq!(prepare_spoken_text(&raw, Some(40)), prepared40, "prepare_spoken_text(40) mismatch for {name}");
        checked += 1;
    }
    assert_eq!(checked, 36);
}

fn b64_decode(s: &str) -> String {
    use base64::Engine;
    String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .expect("base64"),
    )
    .expect("utf8")
}
