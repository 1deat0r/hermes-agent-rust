//! Shared CLI output helpers for Hermes CLI modules.
//!
//! PARITY: `hermes_cli/cli_output.py` @ b9aa928 (whole module). The print
//! helpers route through [`hermes_cli::colors`]; the prompt functions take
//! an injected line reader — the seam that makes the interactive logic
//! testable (upstream reads `input()` directly).
//!
//! Extracts the identical `print_info/success/warning/error` and `prompt()`
//! functions previously duplicated across setup.py, tools_config.py,
//! mcp_config.py, and memory_setup.py.

use crate::colors::{color, Colors};

/// Print a dim informational message.
///
/// PARITY: `print_info` (upstream lines 15-17).
pub fn print_info(text: &str) {
    println!("{}", color(&format!("  {text}"), &[Colors::DIM]));
}

/// Print a green success message with ✓ prefix.
///
/// PARITY: `print_success` (upstream lines 20-22).
pub fn print_success(text: &str) {
    println!("{}", color(&format!("✓ {text}"), &[Colors::GREEN]));
}

/// Print a yellow warning message with ⚠ prefix.
///
/// PARITY: `print_warning` (upstream lines 25-27).
pub fn print_warning(text: &str) {
    println!("{}", color(&format!("⚠ {text}"), &[Colors::YELLOW]));
}

/// Print a red error message with ✗ prefix.
///
/// PARITY: `print_error` (upstream lines 30-32).
pub fn print_error(text: &str) {
    println!("{}", color(&format!("✗ {text}"), &[Colors::RED]));
}

/// Print a bold yellow header.
///
/// PARITY: `print_header` (upstream lines 35-37).
pub fn print_header(text: &str) {
    println!("{}", color(&format!("\n  {text}"), &[Colors::YELLOW]));
}

/// Prompt the user for input with optional default.
///
/// Returns the user's input (stripped), or *default* if the user presses
/// Enter. Returns empty string on EOF.
///
/// PARITY: `prompt` (upstream lines 43-68) with the reader injected; the
/// `password` arm routes through [`crate::secret_prompt`] (the
/// `masked_secret_prompt` call), the default arm through plain line
/// reading. Python `(KeyboardInterrupt, EOFError)` → `""` becomes the
/// reader returning `None`.
pub fn prompt_from(
    question: &str,
    default: Option<&str>,
    password: bool,
    read_line: &mut dyn FnMut() -> Option<String>,
) -> String {
    let suffix = match default {
        Some(default) if !default.is_empty() => format!(" [{default}]"),
        _ => String::new(),
    };
    let _display = color(&format!("  {question}{suffix}: "), &[Colors::YELLOW]);

    if password {
        return crate::secret_prompt::masked_secret_prompt(&format!("  {question}{suffix}: "));
    }
    match read_line() {
        Some(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                default.unwrap_or("").to_string()
            } else {
                value
            }
        }
        // `(KeyboardInterrupt, EOFError): print(); return ""`.
        None => {
            println!();
            String::new()
        }
    }
}

/// Fancy-reader failure modes for [`line_input_from`].
///
/// PARITY: `line_input` except arms (upstream lines 44-50) — KI/EOF
/// re-raise (here: `None`, matching the prompt seam's EOF convention);
/// any other prompt_toolkit runtime failure degrades to the plain reader.
#[derive(Debug, Clone, PartialEq)]
pub enum LineInputFailure {
    /// prompt_toolkit missing (`ImportError`) — use the plain reader.
    Unavailable,
    /// prompt_toolkit present but failed at runtime (e.g. macOS kqueue
    /// EINVAL on fd 0) — use the plain reader.
    Runtime(String),
    /// `KeyboardInterrupt` — propagates (caller sees `None`).
    Interrupted,
    /// `EOFError` — propagates (caller sees `None`).
    Eof,
}

/// Read non-secret text with cursor editing on a real TTY.
///
/// PARITY: `line_input` (upstream lines 29-50 @ 5d59366). Setup commands
/// run outside the chat's prompt_toolkit app, so a short-lived fancy
/// prompt is safe; redirected stdio keeps the plain reader. The fancy
/// reader (prompt_toolkit) crosses the seam as an injected closure since
/// Rust has no prompt_toolkit: `Ok` wins, `Unavailable`/`Runtime` fall
/// back to `fallback_read`, `Interrupted`/`Eof` propagate as `None`.
pub fn line_input_from(
    _prompt_text: &str,
    is_tty: bool,
    fancy_read: &mut dyn FnMut() -> Result<Option<String>, LineInputFailure>,
    fallback_read: &mut dyn FnMut() -> Option<String>,
) -> Option<String> {
    if !is_tty {
        return fallback_read();
    }
    match fancy_read() {
        Ok(value) => value,
        Err(LineInputFailure::Unavailable | LineInputFailure::Runtime(_)) => fallback_read(),
        Err(LineInputFailure::Interrupted | LineInputFailure::Eof) => None,
    }
}

/// Interactive form of [`prompt_from`] reading from stdin.
pub fn prompt(question: &str, default: Option<&str>, password: bool) -> String {
    let stdin = std::io::stdin();
    prompt_from(question, default, password, &mut || {
        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => Some(line),
            Err(_) => None,
        }
    })
}

/// Prompt for a yes/no answer. Returns bool.
///
/// PARITY: `prompt_yes_no` (upstream lines 71-76) — the `Y/n` / `y/N` hint
/// reflects the default, an empty answer takes the default, and any other
/// answer matches by `starts_with("y")` on the lowercased text.
pub fn prompt_yes_no_with(
    question: &str,
    default: bool,
    read_line: &mut dyn FnMut() -> Option<String>,
) -> bool {
    let hint = if default { "Y/n" } else { "y/N" };
    let answer = prompt_from(&format!("{question} ({hint})"), None, false, read_line);
    if answer.is_empty() {
        return default;
    }
    answer.to_lowercase().starts_with('y')
}
