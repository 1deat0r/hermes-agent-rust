//! Stateful scrubber for reasoning/thinking blocks in streamed assistant
//! text.
//!
//! PARITY: `agent/think_scrubber.py` @ b9aa928 (whole module).
//!
//! A per-delta regex destroys the state downstream consumers rely on: when
//! a model streams `"<think>"`, `"Let me check their config"`,
//! `"</think>"` as separate deltas, a per-delta regex erases the opener
//! entirely, so consumers treat the body as regular content and leak
//! reasoning to the user. This module centralises the tag-suppression
//! state machine so every stream callback sees text that already has
//! reasoning blocks removed. Partial tags at delta boundaries are held
//! back until the next delta resolves them, and end-of-stream flushing
//! surfaces any held-back prose that turned out not to be a real tag.
//!
//! Usage: `feed(delta)` per delta, `flush()` at end of stream, `reset()`
//! at the top of each new turn.
//!
//! Tag variants handled (case-insensitive): `<think>`, `<thinking>`,
//! `<reasoning>`, `<thought>`, `<REASONING_SCRATCHPAD>`.
//!
//! Block-boundary rule for opens: an opening tag is only treated as a
//! reasoning-block opener at the start of the stream, after a newline
//! (optionally followed by whitespace), or when only whitespace has been
//! emitted on the current line — so prose that *mentions* the tag name
//! (e.g. "use <think> tags here") is not suppressed. Closed pairs
//! (`<think>X</think>`) are always suppressed regardless of boundary.

/// PARITY: `_OPEN_TAG_NAMES` (upstream line 71).
pub const OPEN_TAG_NAMES: [&str; 5] = [
    "think",
    "thinking",
    "reasoning",
    "thought",
    "REASONING_SCRATCHPAD",
];

/// PARITY: the longest tag length (for partial-tag hold-back bound).
const MAX_TAG_LEN: usize = "<REASONING_SCRATCHPAD>".len();

// PARITY: `THINK_TAG_NAMES` / `THINK_OPEN_TAGS` / `THINK_CLOSE_TAGS`
/// (upstream lines 22-24 @ 5d59366) — the one list every reasoning-hiding
/// surface binds to. Open/close tags are lowercased upstream (consumers
/// match case-insensitively).
pub const THINK_TAG_NAMES: [&str; 5] = OPEN_TAG_NAMES;
pub const THINK_OPEN_TAGS: [&str; 5] = [
    "<think>",
    "<thinking>",
    "<reasoning>",
    "<thought>",
    "<reasoning_scratchpad>",
];
pub const THINK_CLOSE_TAGS: [&str; 5] = [
    "</think>",
    "</thinking>",
    "</reasoning>",
    "</thought>",
    "</reasoning_scratchpad>",
];

/// PARITY: `StreamingThinkScrubber` (upstream lines 66-342).
#[derive(Debug, Default)]
pub struct StreamingThinkScrubber {
    /// True while inside an opened block, waiting for a close tag.
    in_block: bool,
    /// Held-back partial-tag tail.
    buf: String,
    /// True iff the most recent emission ended with `\n`, or nothing has
    /// been emitted yet (start-of-stream counts as a boundary).
    last_emitted_ended_newline: bool,
}

/// The closest-close-tag-wins pair found in a buffer:
/// (start index of the open tag, end index after the close tag).
type ClosedPair = (usize, usize);

impl StreamingThinkScrubber {
    pub fn new() -> Self {
        Self {
            in_block: false,
            buf: String::new(),
            last_emitted_ended_newline: true,
        }
    }

    /// Reset all state. Call at the top of every new turn.
    pub fn reset(&mut self) {
        self.in_block = false;
        self.buf = String::new();
        self.last_emitted_ended_newline = true;
    }

    /// Feed one delta; return the scrubbed visible portion.
    ///
    /// May return an empty string when the entire delta is reasoning
    /// content or is being held back pending resolution of a partial tag
    /// at the boundary.
    ///
    /// PARITY: `feed` (upstream lines 94-181).
    pub fn feed(&mut self, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }
        let mut buf = format!("{}{text}", self.buf);
        self.buf = String::new();
        let mut out: Vec<String> = Vec::new();

        while !buf.is_empty() {
            if self.in_block {
                let (close_idx, close_len) = find_first_tag(&buf, &close_tags());
                if close_idx == -1 {
                    // No close yet — hold back a potential partial close-tag
                    // prefix; discard everything else.
                    let held = max_partial_suffix(&buf, &close_tags());
                    self.buf = if held > 0 {
                        buf[buf.len() - held..].to_string()
                    } else {
                        String::new()
                    };
                    return out.concat();
                }
                buf = buf[(close_idx + close_len as isize) as usize..].to_string();
                self.in_block = false;
            } else {
                // Priority 1 — closed <tag>X</tag> pair anywhere in buf.
                let pair = find_earliest_closed_pair(&buf);
                // Priority 2 — unterminated open tag at a block boundary.
                let (open_idx, open_len) = self.find_open_at_boundary(&buf, &out);

                if pair.is_some() && (open_idx.is_none() || pair.unwrap().0 <= open_idx.unwrap()) {
                    let (start_idx, end_idx) = pair.unwrap();
                    let preceding = &buf[..start_idx];
                    if !preceding.is_empty() {
                        let preceding = strip_orphan_close_tags(preceding);
                        if !preceding.is_empty() {
                            self.last_emitted_ended_newline = preceding.ends_with('\n');
                            out.push(preceding);
                        }
                    }
                    buf = buf[end_idx..].to_string();
                    continue;
                }

                if let Some(open_idx) = open_idx {
                    let preceding = &buf[..open_idx];
                    if !preceding.is_empty() {
                        let preceding = strip_orphan_close_tags(preceding);
                        if !preceding.is_empty() {
                            self.last_emitted_ended_newline = preceding.ends_with('\n');
                            out.push(preceding);
                        }
                    }
                    self.in_block = true;
                    buf = buf[open_idx + open_len..].to_string();
                    continue;
                }

                // No resolvable tag structure in buf. Hold back any
                // partial-tag prefix at the tail, then emit the rest.
                let held = max_partial_suffix(&buf, &open_tags())
                    .max(max_partial_suffix(&buf, &close_tags()));
                // Byte positions from max_partial_suffix are
                // char-boundary-safe (it skips non-boundary splits).
                let (emit_text, held_back) = if held > 0 && buf.is_char_boundary(buf.len() - held) {
                    (
                        buf[..buf.len() - held].to_string(),
                        buf[buf.len() - held..].to_string(),
                    )
                } else {
                    (buf.clone(), String::new())
                };
                self.buf = held_back;
                if !emit_text.is_empty() {
                    let emit_text = strip_orphan_close_tags(&emit_text);
                    if !emit_text.is_empty() {
                        self.last_emitted_ended_newline = emit_text.ends_with('\n');
                        out.push(emit_text);
                    }
                }
                return out.concat();
            }
        }

        out.concat()
    }

    /// End-of-stream flush.
    ///
    /// If still inside an unterminated block, held-back content is
    /// discarded — leaking partial reasoning is worse than a truncated
    /// answer. Otherwise the held-back partial-tag tail is emitted
    /// verbatim. Always treats the next `feed()` as a fresh stream
    /// boundary (intra-turn retries flush then stream again without
    /// `reset()`; leaving the boundary flag false made a new stream's
    /// opening `<think>` look mid-line and leak).
    ///
    /// PARITY: `flush` (upstream lines 184-212).
    pub fn flush(&mut self) -> String {
        if self.in_block {
            self.buf = String::new();
            self.in_block = false;
            self.last_emitted_ended_newline = true;
            return String::new();
        }
        let tail = std::mem::take(&mut self.buf);
        self.last_emitted_ended_newline = true;
        if tail.is_empty() {
            return String::new();
        }
        strip_orphan_close_tags(&tail)
    }
}

/// Materialise literal open tag strings (hot path does string operations,
/// not regex).
fn open_tags() -> Vec<String> {
    OPEN_TAG_NAMES.iter().map(|n| format!("<{n}>")).collect()
}

fn close_tags() -> Vec<String> {
    OPEN_TAG_NAMES.iter().map(|n| format!("</{n}>")).collect()
}

/// PARITY: `_find_first_tag` (upstream lines 216-231) — earliest
/// case-insensitive match over `tags`, as (index, tag_length) or (-1, 0).
fn find_first_tag(buf: &str, tags: &[String]) -> (isize, usize) {
    let buf_lower = buf.to_lowercase();
    let mut best_idx: isize = -1;
    let mut best_len = 0;
    for tag in tags {
        let tag_lower = tag.to_lowercase();
        if let Some(idx) = buf_lower.find(&tag_lower) {
            let idx = idx as isize;
            if best_idx == -1 || idx < best_idx {
                best_idx = idx;
                best_len = tag_lower.len();
            }
        }
    }
    (best_idx, best_len)
}

/// PARITY: `_find_earliest_closed_pair` (upstream lines 234-258) —
/// case-insensitive non-greedy `<tag>...</tag>`; earliest open wins.
fn find_earliest_closed_pair(buf: &str) -> Option<ClosedPair> {
    let buf_lower = buf.to_lowercase();
    let mut best: Option<ClosedPair> = None;
    for name in OPEN_TAG_NAMES {
        let open = format!("<{name}>").to_lowercase();
        let close = format!("</{name}>").to_lowercase();
        let Some(open_idx) = buf_lower.find(&open) else {
            continue;
        };
        let Some(close_idx) = buf_lower[open_idx + open.len()..]
            .find(&close)
            .map(|rel| open_idx + open.len() + rel)
        else {
            continue;
        };
        let end_idx = close_idx + close.len();
        if best.map(|b| open_idx < b.0).unwrap_or(true) {
            best = Some((open_idx, end_idx));
        }
    }
    best
}

impl StreamingThinkScrubber {
    /// PARITY: `_find_open_at_boundary` (upstream lines 261-285).
    fn find_open_at_boundary(
        &self,
        buf: &str,
        already_emitted: &[String],
    ) -> (Option<usize>, usize) {
        let buf_lower = buf.to_lowercase();
        let mut best_idx: Option<usize> = None;
        let mut best_len = 0;
        for tag in open_tags() {
            let tag_lower = tag.to_lowercase();
            let mut search_start = 0;
            loop {
                let Some(idx) = buf_lower[search_start..].find(&tag_lower) else {
                    break;
                };
                let idx = search_start + idx;
                if self.is_block_boundary(buf, idx, already_emitted) {
                    if best_idx.map(|b| idx < b).unwrap_or(true) {
                        best_idx = Some(idx);
                        best_len = tag_lower.len();
                    }
                    break; // first boundary hit for this tag is enough
                }
                search_start = idx + 1;
            }
        }
        (best_idx, best_len)
    }

    /// True iff position `idx` in `buf` is a block boundary.
    ///
    /// PARITY: `_is_block_boundary` (upstream lines 288-315).
    fn is_block_boundary(&self, buf: &str, idx: usize, already_emitted: &[String]) -> bool {
        if idx == 0 {
            if let Some(last) = already_emitted.last() {
                return last.ends_with('\n');
            }
            return self.last_emitted_ended_newline;
        }
        let preceding = &buf[..idx];
        let Some(last_nl) = preceding.rfind('\n') else {
            let prior_newline = match already_emitted.last() {
                Some(last) => last.ends_with('\n'),
                None => self.last_emitted_ended_newline,
            };
            return prior_newline && preceding.trim().is_empty();
        };
        preceding[last_nl + 1..].trim().is_empty()
    }
}

/// PARITY: `_max_partial_suffix` (upstream lines 318-335) — the longest
/// buf-suffix that is a strict prefix of any tag, case-insensitive.
fn max_partial_suffix(buf: &str, tags: &[String]) -> usize {
    if buf.is_empty() {
        return 0;
    }
    let buf_lower = buf.to_lowercase();
    let max_check = buf_lower.len().min(MAX_TAG_LEN - 1);
    for i in (1..=max_check).rev() {
        if !buf_lower.is_char_boundary(buf_lower.len() - i) {
            continue;
        }
        let suffix = &buf_lower[buf_lower.len() - i..];
        for tag in tags {
            let tag_lower = tag.to_lowercase();
            if tag_lower.len() > i && tag_lower.starts_with(suffix) {
                return i;
            }
        }
    }
    0
}

/// PARITY: `_strip_orphan_close_tags` (upstream lines 338-380) — remove
/// close tags (and any trailing whitespace) with no matching open.
fn strip_orphan_close_tags(text: &str) -> String {
    if !text.contains("</") {
        return text.to_string();
    }
    let text_lower = text.to_lowercase();
    let tags = close_tags();
    let mut out: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let mut matched = false;
        if text_lower[i..].starts_with("</") {
            for tag in &tags {
                let tag_lower = tag.to_lowercase();
                if text_lower[i..].starts_with(&tag_lower) {
                    let mut j = i + tag_lower.len();
                    let bytes = text.as_bytes();
                    while j < text.len()
                        && (bytes[j] == b' '
                            || bytes[j] == b'\t'
                            || bytes[j] == b'\n'
                            || bytes[j] == b'\r')
                    {
                        j += 1;
                    }
                    i = j;
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            let ch_len = text[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            out.push(&text[i..i + ch_len]);
            i += ch_len;
        }
    }
    out.concat()
}
