//! Sanitize user prompt text leaked from terminal / paste control sequences.
//!
//! PARITY: `hermes_cli/input_sanitize.py` @ b9aa928 (whole module, lines 1-72).

use std::sync::LazyLock;

/// PARITY: `_BRACKETED_PASTE_BOUNDARY_START` (upstream line 7).
static BRACKETED_PASTE_BOUNDARY_START: LazyLock<fancy_regex::Regex> =
    LazyLock::new(|| compile(r"(^|[\s\n>:\]\)])\[200~"));

/// PARITY: `_BRACKETED_PASTE_BOUNDARY_END` (upstream line 8).
static BRACKETED_PASTE_BOUNDARY_END: LazyLock<fancy_regex::Regex> =
    LazyLock::new(|| compile(r"\[201~(?=$|[\s\n<\[\(\):;.,!?])"));

/// PARITY: `_BRACKETED_PASTE_DEGRADED_START` (upstream line 9).
static BRACKETED_PASTE_DEGRADED_START: LazyLock<fancy_regex::Regex> =
    LazyLock::new(|| compile(r"(^|[\s\n>:\]\)])00~"));

/// PARITY: `_BRACKETED_PASTE_DEGRADED_END` (upstream line 10).
static BRACKETED_PASTE_DEGRADED_END: LazyLock<fancy_regex::Regex> =
    LazyLock::new(|| compile(r"01~(?=$|[\s\n<\[\(\):;.,!?])"));

/// PARITY: `_DESKTOP_PASTE_ARTIFACT` (upstream line 13), the corruption
/// signature from desktop bracketed-paste leaks (#62557).
const DESKTOP_PASTE_ARTIFACT: &str = "~[[e";

/// The patterns are static and use only the constructs both engines accept
/// (the lookaheads are why this crate needs `fancy-regex` rather than the
/// backreference-free default engine).
fn compile(pattern: &str) -> fancy_regex::Regex {
    fancy_regex::Regex::new(pattern).expect("static bracketed-paste pattern must compile")
}

/// Strip leaked bracketed-paste wrapper markers from user-visible text.
///
/// PARITY: `strip_leaked_bracketed_paste_wrappers` (upstream lines 16-39).
/// Defensive normalization for the case where terminal/`prompt_toolkit` parsing
/// fails and the markers end up in the buffer as literal text. Canonical
/// wrappers are stripped unconditionally; the degraded visible forms are
/// removed only at boundaries, so embedded literals such as `literal[200~tag`
/// stay intact.
pub fn strip_leaked_bracketed_paste_wrappers(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }
    let mut text = text
        .replace("\x1b[200~", "")
        .replace("\x1b[201~", "")
        .replace("^[[200~", "")
        .replace("^[[201~", "");
    text = substitute(&BRACKETED_PASTE_BOUNDARY_START, &text, "$1");
    text = substitute(&BRACKETED_PASTE_BOUNDARY_END, &text, "");
    text = substitute(&BRACKETED_PASTE_DEGRADED_START, &text, "$1");
    substitute(&BRACKETED_PASTE_DEGRADED_END, &text, "")
}

/// One `re.sub(pattern, replacement, text)` step. The `$1` group reference
/// reproduces the source's `r"\1"` boundary-preserving replacement.
fn substitute(re: &fancy_regex::Regex, text: &str, replacement: &str) -> String {
    re.replace_all(text, replacement).into_owned()
}

/// Drop a trailing run of the desktop `~[[e` corruption signature (#62557).
///
/// PARITY: `collapse_repeated_input_artifacts` (upstream lines 42-63) called
/// with the source's `min_repeats: int = 4` default.
pub fn collapse_repeated_input_artifacts(text: &str) -> String {
    collapse_repeated_input_artifacts_with(text, DEFAULT_MIN_REPEATS)
}

/// Explicit-`min_repeats` form of [`collapse_repeated_input_artifacts`].
/// Index arithmetic runs on code points, matching Python's `str` slicing.
pub fn collapse_repeated_input_artifacts_with(text: &str, min_repeats: usize) -> String {
    if text.is_empty() {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let marker: Vec<char> = DESKTOP_PASTE_ARTIFACT.chars().collect();
    let mut index = chars.len();
    let mut repeat_count = 0usize;
    while index >= marker.len() && chars[index - marker.len()..index] == marker[..] {
        repeat_count += 1;
        index -= marker.len();
    }
    if repeat_count < min_repeats {
        return text.to_string();
    }

    let mut start = index;
    if start >= 2 && chars[start - 2] == '[' && chars[start - 1] == 'e' {
        start -= 2;
    } else if start >= 1 && chars[start - 1] == '[' {
        start -= 1;
    }
    chars[..start].iter().collect()
}

/// Normalize user-authored prompt text before persistence or model input.
///
/// PARITY: `sanitize_user_prompt_text` (upstream lines 66-72).
pub fn sanitize_user_prompt_text(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }
    collapse_repeated_input_artifacts(&strip_leaked_bracketed_paste_wrappers(text))
}

/// PARITY: the `min_repeats: int = 4` keyword default.
pub const DEFAULT_MIN_REPEATS: usize = 4;
