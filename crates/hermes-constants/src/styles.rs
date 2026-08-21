//! TUI busy-indicator styles — single source of truth shared by the CLI
//! /indicator command, the TUI gateway config handler, and /help.
//!
//! PARITY: hermes_constants.py lines 23–27.

/// Valid indicator styles.
pub const INDICATOR_STYLES: [&str; 4] = ["ascii", "emoji", "kaomoji", "unicode"];

/// Default indicator style.
pub const DEFAULT_INDICATOR_STYLE: &str = "kaomoji";
