//! Utilities for preparing assistant text for speech synthesis.
//!
//! PARITY: tools/tts_text_normalize.py @ b9aa928 (278 LOC, ported 1:1).
//! Lookaround patterns use fancy-regex (the std `regex` crate has none).
//! HTML entity decoding is a faithful CPython html.unescape port backed by
//! the generated crate::html5_entities table.

use fancy_regex::Regex;
use once_cell::sync::Lazy;

const HEAD: char = '\u{0}';

macro_rules! re {
    ($pat:expr) => {{
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new($pat).expect("tts regex"));
        &RE
    }};
}

fn md_code_block_re() -> &'static Regex { re!(r"```[\s\S]*?```") }
fn md_link_re() -> &'static Regex { re!(r"\[([^\]]+)\]\((?:[^()]|\([^)]*\))*\)") }
fn md_image_re() -> &'static Regex { re!(r"!\[([^\]]*)\]\((?:[^()]|\([^)]*\))*\)") }
fn md_inline_code_re() -> &'static Regex { re!(r"`([^`]+)`") }
fn md_bold_re() -> &'static Regex { re!(r"(?s)\*\*(.+?)\*\*") }
fn md_underscore_bold_re() -> &'static Regex { re!(r"(?s)__(.+?)__") }
fn md_italic_re() -> &'static Regex { re!(r"(?s)(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)") }
fn md_underscore_italic_re() -> &'static Regex { re!(r"(?s)(?<!_)_(?!_)(.+?)(?<!_)_(?!_)") }
fn md_strike_re() -> &'static Regex { re!(r"(?s)~~(.+?)~~") }
fn md_heading_line_re() -> &'static Regex { re!(r"(?m)^[ \t]{0,3}#{1,6}[ \t]+(.+?)[ \t]*#*[ \t]*$") }
fn md_blockquote_re() -> &'static Regex { re!(r"(?m)^\s*>\s?") }
fn md_list_item_re() -> &'static Regex { re!(r"(?m)^\s*(?:[-*+]|\d+[.)])\s+") }
fn md_hr_re() -> &'static Regex { re!(r"(?m)^\s*[-*_]{3,}\s*$") }
fn md_table_pipe_re() -> &'static Regex { re!(r"\s*\|\s*") }
fn url_re() -> &'static Regex { re!(r"https?://\S+") }
fn emoji_re() -> &'static Regex { re!(r"[\u{1F1E6}-\u{1F1FF}\u{1F300}-\u{1F5FF}\u{1F600}-\u{1F64F}\u{1F680}-\u{1F6FF}\u{1F700}-\u{1F77F}\u{1F780}-\u{1F7FF}\u{1F800}-\u{1F8FF}\u{1F900}-\u{1F9FF}\u{1FA00}-\u{1FAFF}☀-➿]+") }
fn variation_selector_re() -> &'static Regex { re!("[\u{FE0E}\u{FE0F}]") }
fn nbsp_spaces_re() -> &'static Regex { re!("[\u{a0}\u{2007}\u{202f}]") }
fn temp_range_c_re() -> &'static Regex { re!(r"(?i)(?<!\w)([-+\u{2212}]?\d+(?:\.\d+)?)\s*[\u{2013}\u{2014}-]\s*([-+\u{2212}]?\d+(?:\.\d+)?)\s*°\s*C\b") }
fn temp_range_f_re() -> &'static Regex { re!(r"(?i)(?<!\w)([-+\u{2212}]?\d+(?:\.\d+)?)\s*[\u{2013}\u{2014}-]\s*([-+\u{2212}]?\d+(?:\.\d+)?)\s*°\s*F\b") }
fn temp_single_c_re() -> &'static Regex { re!(r"(?i)(?<!\w)([-+]?\d+(?:\.\d+)?)\s*°\s*C\b") }
fn temp_single_f_re() -> &'static Regex { re!(r"(?i)(?<!\w)([-+]?\d+(?:\.\d+)?)\s*°\s*F\b") }
fn temp_bare_c_re() -> &'static Regex { re!(r"(?i)°\s*C\b") }
fn temp_bare_f_re() -> &'static Regex { re!(r"(?i)°\s*F\b") }
fn temp_angle_re() -> &'static Regex { re!(r"(?<!\w)([-+]?\d+(?:\.\d+)?)\s*°") }
fn unit_kmh_re() -> &'static Regex { re!(r"(?i)(?<=\d)\s*km\s*/\s*h\b") }
fn unit_kmh2_re() -> &'static Regex { re!(r"(?i)(?<=\d)\s*km/h\b") }
fn unit_mm_re() -> &'static Regex { re!(r"(?i)(?<=\d)\s*mm\b") }
fn unit_cm_re() -> &'static Regex { re!(r"(?i)(?<=\d)\s*cm\b") }
fn unit_m_re() -> &'static Regex { re!(r"(?i)(?<=\d)\s*m\b") }
fn numeric_rate_re() -> &'static Regex { re!(r"(?<=\d)\s*/\s*(?=[A-Za-z])") }
fn nz_money_re() -> &'static Regex { re!(r"(?i)NZ\$\s*([\d,]*\d(?:\.\d+)?)") }
fn au_money_re() -> &'static Regex { re!(r"(?i)A\$\s*([\d,]*\d(?:\.\d+)?)") }
fn us_money_re() -> &'static Regex { re!(r"(?i)US\$\s*([\d,]*\d(?:\.\d+)?)") }
fn euro_money_re() -> &'static Regex { re!(r"€\s*([\d,]*\d(?:\.\d+)?)") }
fn pound_money_re() -> &'static Regex { re!(r"£\s*([\d,]*\d(?:\.\d+)?)") }
fn dollar_money_re() -> &'static Regex { re!(r"\$\s*([\d,]*\d(?:\.\d+)?)") }
fn percent_re() -> &'static Regex { re!(r"(?<=\d)\s*%") }
fn bullet_re() -> &'static Regex { re!("[•◦▪▫]") }
// PARITY: upstream `_THINK_BLOCK_RE` is `<think[\s>].*? response` (a literal
// closing ` response` tag, no space). The `<`/`>` are written as \x3c/\x3e so an
// HTML-interpreting renderer can never corrupt them.
fn think_block_re() -> &'static Regex { re!(r"(?is)\x3cthink[\s>].*?\x3c/think\x3e") }
fn think_block_open_re() -> &'static Regex { re!(r"(?is)\x3cthink[\s>].*\z") }
// PARITY: upstream `_VERIFIER_FOOTER_RE` = `^\s*⚠️?\s*File-mutation verifier:...`
// — the warning sign is ⚠ U+26A0 plus an optional variation selector U+FE0F.
fn verifier_footer_re() -> &'static Regex { re!(r"(?m)^\s*\u{26a0}\u{fe0f}?\s*File-mutation verifier:.*(?:\n[ \t]+\u{2022}.*)*") }

fn replace_all(regex: &Regex, text: &str, replacement: &str) -> String {
    regex.replace_all(text, replacement).into_owned()
}

/// PARITY: CPython `html.unescape` @ b9aa928 (the upstream call site is
/// `html.unescape(str(text))`). Mirrors Lib/html/__init__.py `_charref` scan +
/// `_replace_charref` exactly, including invalid-ref handling.
fn html_unescape(text: &str) -> String {
    use crate::html5_entities::{HTML5_ENTITIES, INVALID_CHARREFS, INVALID_CODEPOINTS};
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            // Copy a whole non-& run at once.
            let start = i;
            while i < bytes.len() && bytes[i] != b'&' {
                i += 1;
            }
            out.push_str(&text[start..i]);
            continue;
        }
        // Try to match `&` + charref: `#[0-9]+;?` | `#[xX][0-9a-fA-F]+;?` |
        // `[^\t\n\f <&#;]{1,32};?` (CPython _charref pattern).
        let amp = i;
        i += 1; // skip '&'
        if i >= bytes.len() {
            out.push('&');
            break;
        }
        let mut j = i;
        let first = bytes[j];
        if first == b'#' {
            // Numeric charref.
            j += 1;
            let mut hex = false;
            if j < bytes.len() && (bytes[j] == b'x' || bytes[j] == b'X') {
                hex = true;
                j += 1;
            }
            let num_start = j;
            while j < bytes.len() {
                let b = bytes[j];
                let ok = if hex {
                    b.is_ascii_hexdigit()
                } else {
                    b.is_ascii_digit()
                };
                if !ok {
                    break;
                }
                j += 1;
            }
            if j == num_start {
                // No digits: not a valid charref; emit '&' and rescan from the
                // byte after '&' (CPython regex would not have matched here).
                out.push('&');
                continue;
            }
            if j < bytes.len() && bytes[j] == b';' {
                j += 1;
            }
            // digits excludes the optional trailing ';'
            let digits = if j > num_start && bytes[j - 1] == b';' {
                &text[num_start..j - 1]
            } else {
                &text[num_start..j]
            };
            let num = if hex {
                u32::from_str_radix(digits, 16)
            } else {
                digits.parse::<u32>()
            };
            match num {
                Ok(n) => {
                    // _invalid_charrefs first (HTML5 C1 remappings + NUL + CR).
                    if let Some((_, repl)) = INVALID_CHARREFS.iter().find(|(k, _)| *k == n) {
                        out.push_str(repl);
                    } else if (0xD800..=0xDFFF).contains(&n) || n > 0x10FFFF {
                        out.push('\u{FFFD}');
                    } else if INVALID_CODEPOINTS.binary_search(&n).is_ok() {
                        // Decodes to empty string.
                    } else if let Some(ch) = char::from_u32(n) {
                        out.push(ch);
                    } else {
                        out.push('\u{FFFD}');
                    }
                }
                Err(_) => {
                    out.push('&');
                    i = amp + 1;
                    continue;
                }
            }
            i = j;
        } else {
            // Named charref: `[^\t\n\f <&#;]{1,32};?`
            let name_start = j;
            let mut name_end = j;
            while name_end < bytes.len() && name_end - name_start < 32 {
                let b = bytes[name_end];
                if b == b'\t' || b == b'\n' || b == b'\x0c' || b == b' ' || b == b'<'
                    || b == b'&' || b == b'#' || b == b';'
                {
                    break;
                }
                name_end += 1;
            }
            let has_semi = name_end < bytes.len() && bytes[name_end] == b';';
            let full_end = if has_semi { name_end + 1 } else { name_end };
            let name = &text[name_start..full_end];
            // Binary-search exact name (includes trailing ';' when present).
            let name_ref: &str = name;
            if let Ok(idx) = HTML5_ENTITIES.binary_search_by(|(k, _)| (*k).cmp(name_ref)) {
                out.push_str(HTML5_ENTITIES[idx].1);
                i = full_end;
            } else if name.len() > 1 {
                // CPython: longest matching prefix from len-1 down to 2.
                let mut replaced = false;
                for x in (2..name.len()).rev() {
                    let prefix = &name[..x];
                    if let Ok(idx) = HTML5_ENTITIES.binary_search_by(|(k, _)| (*k).cmp(prefix)) {
                        out.push_str(HTML5_ENTITIES[idx].1);
                        out.push_str(&name[x..]);
                        replaced = true;
                        break;
                    }
                }
                if replaced {
                    i = full_end;
                } else {
                    out.push('&');
                    out.push_str(name);
                    i = full_end;
                }
            } else {
                out.push('&');
                out.push_str(name);
                i = full_end;
            }
        }
    }
    out
}

/// Replace all matches of *regex* with the first capture group's text.
fn replace_with_capture(regex: &Regex, text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0usize;
    for m in regex.captures_iter(text) {
        let m = m.expect("capture match");
        let full = m.get(0).expect("full").range();
        out.push_str(&text[last..full.start]);
        if let Some(g) = m.get(1) {
            out.push_str(g.as_str());
        }
        last = full.end;
    }
    out.push_str(&text[last..]);
    out
}

/// Strip Markdown/Telegram formatting while preserving readable words.
pub fn strip_markdown_for_tts(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut text = html_unescape(text);
    text = replace_all(md_code_block_re(), &text, " ");
    // _MD_IMAGE_RE: ` m.group(1) ` when non-empty else " ".
    text = replace_image_alt(&text);
    text = replace_with_capture(md_link_re(), &text);
    text = replace_all(url_re(), &text, "");
    text = replace_with_capture(md_inline_code_re(), &text);
    text = replace_with_capture(md_bold_re(), &text);
    text = replace_with_capture(md_underscore_bold_re(), &text);
    text = replace_with_capture(md_italic_re(), &text);
    text = replace_with_capture(md_underscore_italic_re(), &text);
    text = replace_with_capture(md_strike_re(), &text);
    text = replace_heading(&text);
    text = replace_all(md_blockquote_re(), &text, "");
    text = replace_all(md_list_item_re(), &text, "");
    text = replace_all(md_hr_re(), &text, "");
    text = replace_all(md_table_pipe_re(), &text, "; ");
    text
}

fn replace_image_alt(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0usize;
    for m in md_image_re().captures_iter(text) {
        let m = m.expect("img match");
        let full = m.get(0).expect("full").range();
        out.push_str(&text[last..full.start]);
        out.push(' ');
        if let Some(g) = m.get(1) {
            let alt = g.as_str();
            if !alt.is_empty() {
                out.push_str(alt);
            }
        }
        out.push(' ');
        last = full.end;
    }
    out.push_str(&text[last..]);
    out
}

fn replace_heading(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0usize;
    for m in md_heading_line_re().captures_iter(text) {
        let m = m.expect("heading match");
        let full = m.get(0).expect("full").range();
        out.push_str(&text[last..full.start]);
        if let Some(g) = m.get(1) {
            out.push_str(g.as_str().trim_end());
            out.push(HEAD);
        }
        last = full.end;
    }
    out.push_str(&text[last..]);
    out
}

/// Expand common symbols/shorthand into words a TTS engine reads well.
pub fn normalize_symbols_for_tts(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut text = text.to_string();
    text = replace_all(nbsp_spaces_re(), &text, " ");
    text = text.replace('\u{2212}', "-");
    text = text.replace('…', "...");
    text = replace_temp_ranges(&text, temp_range_c_re(), "Celsius");
    text = replace_temp_ranges(&text, temp_range_f_re(), "Fahrenheit");
    text = replace_all(temp_single_c_re(), &text, "$1 degrees Celsius");
    text = replace_all(temp_single_f_re(), &text, "$1 degrees Fahrenheit");
    text = replace_all(temp_bare_c_re(), &text, "degrees Celsius");
    text = replace_all(temp_bare_f_re(), &text, "degrees Fahrenheit");
    text = replace_all(temp_angle_re(), &text, "$1 degrees");
    text = text.replace('°', " degrees");
    text = replace_all(unit_kmh_re(), &text, " kilometres per hour");
    text = replace_all(unit_kmh2_re(), &text, " kilometres per hour");
    text = replace_all(unit_mm_re(), &text, " millimetres");
    text = replace_all(unit_cm_re(), &text, " centimetres");
    text = replace_all(unit_m_re(), &text, " metres");
    text = replace_all(numeric_rate_re(), &text, " per ");
    text = replace_money(nz_money_re(), &text, " New Zealand dollars");
    text = replace_money(au_money_re(), &text, " Australian dollars");
    text = replace_money(us_money_re(), &text, " US dollars");
    text = replace_money(euro_money_re(), &text, " euros");
    text = replace_money(pound_money_re(), &text, " pounds");
    text = replace_money(dollar_money_re(), &text, " dollars");
    text = replace_all(percent_re(), &text, " percent");
    text = text.replace('&', " and ");
    text = replace_all(bullet_re(), &text, " ");
    text = text.replace('→', " to ");
    text = text.replace('⇒', " to ");
    text = text.replace('≈', " about ");
    text = text.replace('~', " about ");
    text = replace_all(variation_selector_re(), &text, "");
    text = replace_all(emoji_re(), &text, "");
    text
}

fn replace_temp_ranges(text: &str, regex: &Regex, unit: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0usize;
    for m in regex.captures_iter(text) {
        let m = m.expect("temp match");
        let full = m.get(0).expect("full").range();
        out.push_str(&text[last..full.start]);
        let g1 = m.get(1).map(|g| g.as_str().replace('\u{2212}', "-")).unwrap_or_default();
        let g2 = m.get(2).map(|g| g.as_str().replace('\u{2212}', "-")).unwrap_or_default();
        out.push_str(&format!("{g1} to {g2} degrees {unit}"));
        last = full.end;
    }
    out.push_str(&text[last..]);
    out
}

fn replace_money(regex: &Regex, text: &str, suffix: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0usize;
    for m in regex.captures_iter(text) {
        let m = m.expect("money match");
        let full = m.get(0).expect("full").range();
        out.push_str(&text[last..full.start]);
        if let Some(g) = m.get(1) {
            out.push_str(g.as_str());
        }
        out.push_str(suffix);
        last = full.end;
    }
    out.push_str(&text[last..]);
    out
}

/// Collapse visual formatting into calm spoken paragraphs.
pub fn smooth_whitespace_for_tts(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let raw_lines: Vec<&str> = text.lines().collect();
    let add_sentence_pauses = raw_lines
        .iter()
        .filter(|l| !l.replace(HEAD, "").trim().is_empty())
        .count()
        > 1;
    let mut lines: Vec<String> = Vec::new();
    let mut pending_heading: Option<String> = None;

    for raw_line in raw_lines {
        let is_heading = raw_line.trim_end().ends_with(HEAD);
        let line = raw_line.replace(HEAD, "").trim().to_string();
        if line.is_empty() {
            if pending_heading.is_none() && lines.last().map(|l| !l.is_empty()).unwrap_or(false) {
                lines.push(String::new());
            }
            continue;
        }
        if is_heading {
            flush_pending(&mut lines, &mut pending_heading);
            pending_heading = Some(line.trim_end_matches(['.', ':', ';', ',']).to_string());
            continue;
        }
        let mut line = line;
        if let Some(heading) = pending_heading.take() {
            let trimmed = heading.trim_end_matches(['.', ':', ';', ',']);
            line = format!("{trimmed}, {line}");
        }
        if add_sentence_pauses && !line.ends_with(['.', '!', '?', ';', ':']) {
            line.push('.');
        }
        lines.push(line);
    }
    flush_pending(&mut lines, &mut pending_heading);

    let joined = lines.join("\n");
    let mut text = re!(r"\n{3,}").replace_all(&joined, "\n\n").into_owned();
    text = re!(r"[ \t]{2,}").replace_all(&text, " ").into_owned();
    text = re!(r"\s+([,.;:!?])").replace_all(&text, "$1").into_owned();
    text = re!(r"([,.;:!?])([A-Za-z])").replace_all(&text, "$1 $2").into_owned();
    text = re!(r"\.{4,}").replace_all(&text, "...").into_owned();
    text.trim().to_string()
}

fn flush_pending(lines: &mut Vec<String>, pending_heading: &mut Option<String>) {
    if let Some(heading) = pending_heading.take() {
        lines.push(format!("{}.", heading.trim_end_matches(['.', ':', ';', ','])));
    }
}

/// Remove blocks that must never reach a speech provider.
pub fn strip_nonspoken_blocks(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let text = replace_all(think_block_re(), text, " ");
    let text = replace_all(think_block_open_re(), &text, " ");
    replace_all(verifier_footer_re(), &text, " ")
}

/// Collapse newlines into sentence breaks for single-line TTS payloads.
pub fn flatten_newlines_for_payload(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut text = re!(r"\n{2,}").replace_all(text, ". ").into_owned();
    text = re!(r"(?<=[.!?;:,])\n").replace_all(&text, " ").into_owned();
    text = text.replace('\n', ". ");
    text = re!(r"\.\s*\.").replace_all(&text, ".").into_owned();
    text = re!(r"[ \t]{2,}").replace_all(&text, " ").into_owned();
    text.trim().to_string()
}

/// Return a TTS-friendly script from assistant text.
pub fn prepare_spoken_text(text: &str, max_chars: Option<usize>) -> String {
    let mut spoken = strip_nonspoken_blocks(text);
    spoken = strip_markdown_for_tts(&spoken);
    spoken = normalize_symbols_for_tts(&spoken);
    spoken = smooth_whitespace_for_tts(&spoken);
    spoken = flatten_newlines_for_payload(&spoken);
    if let Some(max) = max_chars {
        if max > 0 && spoken.chars().count() > max {
            spoken = spoken.chars().take(max).collect::<String>().trim_end().to_string();
        }
    }
    spoken
}
