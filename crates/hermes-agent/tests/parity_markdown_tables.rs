//! Parity tests for `agent/markdown_tables.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle. CJK widths use
//! the unicode-width crate (the wcswidth analog).

use hermes_agent::markdown_tables::{
    is_table_divider, looks_like_table_row, realign_markdown_tables, split_table_row,
};

#[test]
fn split_table_row_trims_and_drops_outer_pipes() {
    assert_eq!(split_table_row("| a | b | c |"), vec!["a", "b", "c"]);
    assert_eq!(split_table_row("a | b"), vec!["a", "b"]);
    assert_eq!(split_table_row("| x |"), vec!["x"]);
    assert_eq!(split_table_row("  |  spaced  |  "), vec!["spaced"]);
}

#[test]
fn divider_detection() {
    assert!(is_table_divider("|---|---|"));
    assert!(is_table_divider("| :--- | ---: |"));
    // Minimum dash run is 3.
    assert!(!is_table_divider("| - | - |"));
    assert!(!is_table_divider("| -- | -- |"));
    assert!(!is_table_divider("| a | b |"));
}

#[test]
fn looks_like_table_row_is_permissive() {
    assert!(looks_like_table_row("| a | b |"));
    assert!(
        looks_like_table_row("a | b | c"),
        "two pipes without a leading pipe"
    );
    assert!(!looks_like_table_row("no pipes here"));
    assert!(!looks_like_table_row("x |"));
}

#[test]
fn realign_pads_cjk_to_display_width() {
    // The CJK header is 2 cells per glyph, so the body column must pad to
    // display width 4+ like the header.
    let input = "| 名称 | 数量 |\n|---|---|\n| ab | 1 |";
    let out = realign_markdown_tables(input, None);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3);
    // Every rendered line must have the same display length.
    let widths: Vec<usize> = lines.iter().map(|l| hermes_cjk_width(l)).collect();
    assert!(
        widths.windows(2).all(|w| w[0] == w[1]),
        "columns aligned by display width: {out}"
    );
    // Content preserved.
    assert!(out.contains("名称") && out.contains("ab") && out.contains("1"));
}

// Minimal display-width helper for assertions: ASCII=1, CJK=2.
fn hermes_cjk_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            let u = c as u32;
            if (0x1100..=0x115F).contains(&u)
                || (0x2E80..=0xA4CF).contains(&u) && u != 0x303F
                || (0xAC00..=0xD7A3).contains(&u)
                || (0xF900..=0xFAFF).contains(&u)
                || (0xFF00..=0xFF60).contains(&u)
            {
                2
            } else {
                1
            }
        })
        .sum()
}

#[test]
fn non_table_prose_passes_through_verbatim() {
    let text = "just some prose\nwith no tables at all\n";
    assert_eq!(realign_markdown_tables(text, None), text);
    // No pipes at all: early return of the identical text.
    assert_eq!(realign_markdown_tables("plain", None), "plain");
}

#[test]
fn header_without_divider_is_untouched() {
    let text = "| a | b |\nno divider follows";
    assert_eq!(realign_markdown_tables(text, None), text);
}

#[test]
fn divider_mid_block_is_skipped_not_emitted_twice() {
    let input = "| a | b |\n|---|---|\n|---|---|\n| x | y |";
    let out = realign_markdown_tables(input, None);
    let lines: Vec<&str> = out.lines().collect();
    // The second divider is consumed as a skipped line, not rendered.
    assert_eq!(lines.len(), 3, "{out}");
    assert!(!lines[2].contains("---"), "{out}");
}

#[test]
fn wide_table_falls_back_to_vertical_rendering() {
    let input = "| col_a | col_b | col_c | col_d |\n|---|---|---|---|\n| 1 | 2 | 3 | 4 |\n| 5 | 6 | 7 | 8 |";
    // Width 20 forces the vertical key-value fallback.
    let out = realign_markdown_tables(input, Some(20));
    assert!(out.contains("col_a: 1"), "{out}");
    assert!(out.contains("col_b: 2"), "{out}");
    // A thin divider separates the two body-row blocks.
    assert!(out.contains("\u{2500}"), "divider between rows: {out}");
    assert!(!out.contains("| col_a"), "no horizontal grid");
}

#[test]
fn narrow_fit_keeps_horizontal_grid() {
    let input = "| a | b |\n|---|---|\n| 1 | 2 |";
    let out = realign_markdown_tables(input, Some(80));
    // Horizontal grid kept: columns padded to the 3-char minimum width.
    assert!(out.starts_with("| a"), "{out}");
    assert!(out.contains("---"), "divider dashes: {out}");
    assert_eq!(out.lines().count(), 3);
}

#[test]
fn empty_cells_get_column_default_labels_in_vertical_mode() {
    // Two body rows + long first header forces the vertical fallback, and
    // the empty second header renders as the "Column 2" default label.
    let input = "| very_long_header_one | |\n|---|---|\n| v1 | |\n| v2 | |";
    let out = realign_markdown_tables(input, Some(20));
    assert!(
        out.contains("Column 2:"),
        "empty header gets Column N label: {out}"
    );
    assert!(out.contains("very_long_header_one: v1"), "{out}");
}

#[test]
fn ragged_rows_padded_to_header_width() {
    let input = "| a | b | c |\n|---|---|---|\n| one |";
    let out = realign_markdown_tables(input, None);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3);
    // The short body row is padded with empty cells to 3 columns.
    assert_eq!(hermes_cjk_width(lines[1]), hermes_cjk_width(lines[0]));
    assert_eq!(hermes_cjk_width(lines[2]), hermes_cjk_width(lines[0]));
}
