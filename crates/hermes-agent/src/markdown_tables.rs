//! CJK/wide-character-aware re-alignment of model-emitted markdown tables.
//!
//! PARITY: `agent/markdown_tables.py` @ b9aa928 (whole module). Uses the
//! `unicode-width` crate as the `wcwidth.wcswidth` analog.
//!
//! Models pad markdown tables assuming each character occupies one
//! terminal cell. CJK glyphs and most emoji render as two cells, so the
//! model's spacing collapses into drift on a real terminal. This module
//! rebuilds row padding using display columns, preserving the table's
//! pipes and dashes.
//!
//! The helper is deliberately conservative:
//! * Only contiguous `| ... |` blocks with a divider line are rewritten.
//! * Anything that does not look like a table is passed through unchanged.
//! * Single-line / mid-stream fragments are left alone.
//!
//! Caveat preserved: some emoji-with-variation-selector sequences measure
//! as zero width (wcwidth returns -1); those are clamped to 0 rather than
//! corrupting the column math.

use once_cell::sync::Lazy;
use regex::Regex;
use unicode_width::UnicodeWidthStr;

/// PARITY: `_DIVIDER_CELL_RE` (upstream line 18) — `^\s*:?-{3,}:?\s*$`.
static DIVIDER_CELL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*:?-{3,}:?\s*$").expect("divider cell re"));

/// Matches the divider's minimum dash run.
/// PARITY: `_MIN_COL_WIDTH` (upstream line 19).
const MIN_COL_WIDTH: usize = 3;

/// `wcswidth` clamped to a non-negative integer — control chars / unknown
/// sequences measure zero rather than letting a negative break the
/// column-width math.
///
/// PARITY: `_disp_width` (upstream lines 22-30).
fn disp_width(s: &str) -> usize {
    s.width()
}

/// PARITY: `_pad_to_width` (upstream lines 33-35).
fn pad_to_width(s: &str, target: usize) -> String {
    let width = disp_width(s);
    format!("{s}{}", " ".repeat(target.saturating_sub(width)))
}

/// Split `| a | b | c |` into `["a", "b", "c"]` with trims.
///
/// PARITY: `split_table_row` (upstream lines 38-44).
pub fn split_table_row(row: &str) -> Vec<String> {
    let mut s = row.trim();
    if let Some(rest) = s.strip_prefix('|') {
        s = rest;
    }
    if let Some(rest) = s.strip_suffix('|') {
        s = rest;
    }
    s.split('|').map(|c| c.trim().to_string()).collect()
}

/// True when `row` is a markdown table separator line.
///
/// PARITY: `is_table_divider` (upstream lines 47-51).
pub fn is_table_divider(row: &str) -> bool {
    let cells = split_table_row(row);
    cells.len() > 1 && cells.iter().all(|c| DIVIDER_CELL_RE.is_match(c))
}

/// True when `row` could plausibly be a markdown table row (permissive —
/// the realigner only rewrites blocks accompanied by a divider).
///
/// PARITY: `looks_like_table_row` (upstream lines 54-70).
pub fn looks_like_table_row(row: &str) -> bool {
    if !row.contains('|') {
        return false;
    }
    let stripped = row.trim();
    if stripped.is_empty() {
        return false;
    }
    if stripped.starts_with('|') {
        return true;
    }
    stripped.matches('|').count() >= 2
}

/// PARITY: `_render_block` (upstream lines 73-118).
fn render_block(rows: &mut Vec<Vec<String>>, available_width: Option<usize>) -> Vec<String> {
    let ncols = rows.iter().map(Vec::len).max().unwrap_or(0);
    for row in rows.iter_mut() {
        row.resize(ncols, String::new());
    }

    let widths: Vec<usize> = (0..ncols)
        .map(|c| {
            rows.iter()
                .map(|r| disp_width(&r[c]))
                .max()
                .unwrap_or(MIN_COL_WIDTH)
                .max(MIN_COL_WIDTH)
        })
        .collect();

    // `| ` + cell + ` ` per column, plus the closing `|`.
    let horizontal_width: usize = widths.iter().sum::<usize>() + 3 * ncols + 1;

    if let Some(available) = available_width {
        if horizontal_width > available.max(20) {
            return render_vertical(rows, ncols, available);
        }
    }

    let row_line = |cells: &[String]| -> String {
        let padded: Vec<String> = cells
            .iter()
            .enumerate()
            .map(|(k, c)| pad_to_width(c, widths[k]))
            .collect();
        format!("| {} |", padded.join(" | "))
    };

    let mut out = vec![row_line(&rows[0])];
    out.push(format!(
        "|{}|",
        widths
            .iter()
            .map(|w| "-".repeat(w + 2))
            .collect::<Vec<_>>()
            .join("|")
    ));
    for r in rows.iter().skip(1) {
        out.push(row_line(r));
    }
    out
}

/// Soft-wrap `text` at word boundaries to fit `width` display cells.
///
/// Falls back to hard-breaking the longest word if a single token is
/// wider than `width`. Empty input yields a single empty string.
///
/// PARITY: `_wrap_to_width` (upstream lines 121-186).
fn wrap_to_width(text: &str, width: usize) -> Vec<String> {
    if width == 0 || text.is_empty() {
        return vec![text.to_string()];
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_w = 0usize;

    let hard_break = |word: &str, w: usize| -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut buf = String::new();
        let mut bw = 0usize;
        for ch in word.chars() {
            let cw = disp_width(&ch.to_string()).max(1);
            if bw + cw > w && !buf.is_empty() {
                out.push(std::mem::take(&mut buf));
                buf = ch.to_string();
                bw = cw;
            } else {
                buf.push(ch);
                bw += cw;
            }
        }
        if !buf.is_empty() {
            out.push(buf);
        }
        out
    };

    for word in words {
        let ww = disp_width(word);
        if current.is_empty() {
            if ww <= width {
                current = word.to_string();
                current_w = ww;
            } else {
                let pieces = hard_break(word, width);
                let last = pieces.last().cloned().unwrap_or_default();
                lines.extend(pieces[..pieces.len().saturating_sub(1)].to_vec());
                current = last;
                current_w = disp_width(&current);
            }
            continue;
        }
        if current_w + 1 + ww <= width {
            current.push(' ');
            current.push_str(word);
            current_w += 1 + ww;
        } else {
            lines.push(std::mem::take(&mut current));
            if ww <= width {
                current = word.to_string();
                current_w = ww;
            } else {
                let pieces = hard_break(word, width);
                let last = pieces.last().cloned().unwrap_or_default();
                lines.extend(pieces[..pieces.len().saturating_sub(1)].to_vec());
                current = last;
                current_w = disp_width(&current);
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Render a too-wide table as vertical `Header: value` rows.
///
/// Mirrors Claude Code's narrow-terminal fallback: each body row becomes a
/// block of `Header: cell-value` lines (continuations indented two
/// spaces) separated by a thin `─` divider, every line narrower than
/// `available_width`.
///
/// PARITY: `_render_vertical` (upstream lines 189-247).
fn render_vertical(rows: &[Vec<String>], ncols: usize, available_width: usize) -> Vec<String> {
    if rows.is_empty() {
        return Vec::new();
    }

    let mut headers = rows[0].clone();
    headers.resize(ncols, String::new());
    let body = &rows[1..];

    let labels: Vec<String> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            if h.is_empty() {
                format!("Column {}", i + 1)
            } else {
                h.clone()
            }
        })
        .collect();

    let sep_width = (available_width.saturating_sub(2)).clamp(20, 40);
    let separator = "\u{2500}".repeat(sep_width);
    let indent = "  ";
    let indent_w = disp_width(indent);

    let mut out: Vec<String> = Vec::new();
    for (ri, row) in body.iter().enumerate() {
        if ri > 0 {
            out.push(separator.clone());
        }
        for ci in 0..ncols {
            let label = &labels[ci];
            let value = row.get(ci).cloned().unwrap_or_default();
            let label_w = disp_width(label);
            let first_budget = (available_width.saturating_sub(label_w + 2)).max(10);
            let cont_budget = (available_width.saturating_sub(indent_w)).max(10);
            if value.is_empty() {
                out.push(format!("{label}:"));
                continue;
            }
            let wrapped = wrap_to_width(&value, first_budget);
            out.push(format!("{label}: {}", wrapped[0]));
            if wrapped.len() > 1 {
                // Re-flow continuation text at the wider continuation
                // budget.
                let cont_text = wrapped[1..].join(" ");
                for line in wrap_to_width(&cont_text, cont_budget) {
                    if !line.trim().is_empty() {
                        out.push(format!("{indent}{line}"));
                    }
                }
            }
        }
    }
    out
}

/// Rewrite every `| … |` + divider block with width-aware padding.
///
/// Lines that are not part of a recognised table are returned verbatim, so
/// this is safe to apply to arbitrary assistant prose. If
/// `available_width` is given, tables wider than that are rendered as
/// vertical key-value pairs instead of a horizontal grid (avoids terminal
/// soft-wrap destroying column alignment).
///
/// PARITY: `realign_markdown_tables` (upstream lines 250-306).
pub fn realign_markdown_tables(text: &str, available_width: Option<usize>) -> String {
    if !text.contains('|') {
        return text.to_string();
    }

    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = lines.len();

    while i < n {
        let line = lines[i];
        // A table starts with a header row whose next line is a divider.
        if line.contains('|') && i + 1 < n && is_table_divider(lines[i + 1]) {
            let header = split_table_row(line);
            let mut body: Vec<Vec<String>> = Vec::new();
            let mut j = i + 2;
            while j < n && lines[j].contains('|') && !lines[j].trim().is_empty() {
                if is_table_divider(lines[j]) {
                    j += 1;
                    continue;
                }
                body.push(split_table_row(lines[j]));
                j += 1;
            }

            if header.iter().any(|c| !c.is_empty()) || !body.is_empty() {
                let mut all = vec![header];
                all.extend(body);
                out.extend(render_block(&mut all, available_width));
                i = j;
                continue;
            }
        }
        out.push(line.to_string());
        i += 1;
    }

    out.join("\n")
}
