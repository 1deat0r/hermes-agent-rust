//! Shared ANSI color utilities for Hermes CLI modules.
//!
//! PARITY: `hermes_cli/colors.py` @ b9aa928 (whole module).
//!
//! Respects the NO_COLOR environment variable (https://no-color.org/) and
//! `TERM=dumb`, in addition to the TTY check.

use std::io::IsTerminal;

/// PARITY: the `Colors` class constants (upstream lines 29-39).
pub struct Colors;

impl Colors {
    pub const RESET: &'static str = "\u{1b}[0m";
    pub const BOLD: &'static str = "\u{1b}[1m";
    pub const DIM: &'static str = "\u{1b}[2m";
    pub const RED: &'static str = "\u{1b}[31m";
    pub const GREEN: &'static str = "\u{1b}[32m";
    pub const YELLOW: &'static str = "\u{1b}[33m";
    pub const BLUE: &'static str = "\u{1b}[34m";
    pub const MAGENTA: &'static str = "\u{1b}[35m";
    pub const CYAN: &'static str = "\u{1b}[36m";
}

/// Return true when colored output is appropriate.
///
/// PARITY: `should_use_color` (upstream lines 14-24). `NO_COLOR` present at
/// all (Python `is not None` — even an empty value) disables color;
/// `TERM=dumb` disables; a non-TTY stdout disables.
pub fn should_use_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var("TERM")
        .map(|term| term == "dumb")
        .unwrap_or(false)
    {
        return false;
    }
    if !std::io::stdout().is_terminal() {
        return false;
    }
    true
}

/// Apply color codes to text (only when color output is appropriate).
///
/// PARITY: `color` (upstream lines 42-46) — `"".join(codes) + text +
/// Colors.RESET`.
pub fn color(text: &str, codes: &[&str]) -> String {
    if !should_use_color() {
        return text.to_string();
    }
    format!("{}{}{}", codes.concat(), text, Colors::RESET)
}
