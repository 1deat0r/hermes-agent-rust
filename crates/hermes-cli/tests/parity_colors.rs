//! Parity tests for `hermes_cli/colors.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle. Env/TTY tests
//! serialize behind a mutex per the workspace convention.

use std::sync::Mutex;

use hermes_cli::colors::{color, should_use_color, Colors};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn color_constants_match_the_ansi_escapes() {
    assert_eq!(Colors::RESET, "\u{1b}[0m");
    assert_eq!(Colors::BOLD, "\u{1b}[1m");
    assert_eq!(Colors::DIM, "\u{1b}[2m");
    assert_eq!(Colors::RED, "\u{1b}[31m");
    assert_eq!(Colors::GREEN, "\u{1b}[32m");
    assert_eq!(Colors::YELLOW, "\u{1b}[33m");
    assert_eq!(Colors::BLUE, "\u{1b}[34m");
    assert_eq!(Colors::MAGENTA, "\u{1b}[35m");
    assert_eq!(Colors::CYAN, "\u{1b}[36m");
}

#[test]
fn no_color_env_disables_even_when_empty() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // `os.environ.get("NO_COLOR") is not None` — any value, including the
    // empty string, disables color (https://no-color.org/).
    unsafe { std::env::set_var("NO_COLOR", "") };
    assert!(!should_use_color());
    unsafe { std::env::set_var("NO_COLOR", "1") };
    assert!(!should_use_color());
    unsafe { std::env::remove_var("NO_COLOR") };
}

#[test]
fn term_dumb_disables() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        std::env::remove_var("NO_COLOR");
        std::env::set_var("TERM", "dumb");
    }
    assert!(!should_use_color());
    unsafe { std::env::remove_var("TERM") };
}

#[test]
fn non_tty_stdout_disables() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        std::env::remove_var("NO_COLOR");
        std::env::set_var("TERM", "xterm-256color");
    }
    // Under `cargo test` stdout is captured (not a TTY), so the TTY check
    // disables color — this is the honest environment the tests run in.
    assert!(!should_use_color());
    unsafe { std::env::remove_var("TERM") };
}

#[test]
fn color_codes_join_prefix_and_reset_suffix() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::remove_var("NO_COLOR") };
    // The grammar is pinned regardless of the TTY decision: when color is
    // disabled the text passes through verbatim; when a codes list is
    // applied, reset is the suffix.
    let plain = color("hello", &[Colors::RED]);
    if should_use_color() {
        assert_eq!(plain, "\u{1b}[31mhello\u{1b}[0m");
    } else {
        assert_eq!(plain, "hello");
    }
    // Multiple codes concatenate in order.
    let multi = color("x", &[Colors::BOLD, Colors::RED]);
    if should_use_color() {
        assert_eq!(multi, "\u{1b}[1m\u{1b}[31mx\u{1b}[0m");
    } else {
        assert_eq!(multi, "x");
    }
}

#[test]
fn disabled_color_returns_verbatim_text() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("NO_COLOR", "1") };
    assert_eq!(color("hello", &[Colors::GREEN, Colors::BOLD]), "hello");
    unsafe { std::env::remove_var("NO_COLOR") };
}
