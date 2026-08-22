//! Configurable budget constants for tool result persistence.
//!
//! PARITY: tools/budget_config.py @ b9aa928 (114 LOC, ported 1:1).
//!
//! Per-tool resolution: pinned > config overrides > registry > default.

use std::collections::HashMap;

/// Tools whose thresholds must never be overridden (read_file=inf prevents
/// infinite persist->read->persist loops).
pub const PINNED_THRESHOLDS: &[(&str, BudgetThreshold)] = &[("read_file", BudgetThreshold::Infinite)];

pub const DEFAULT_RESULT_SIZE_CHARS: usize = 100_000;
pub const DEFAULT_TURN_BUDGET_CHARS: usize = 200_000;
pub const DEFAULT_PREVIEW_SIZE_CHARS: usize = 1_500;

const CHARS_PER_TOKEN: i64 = 4;
const PER_RESULT_WINDOW_FRACTION: f64 = 0.15;
const PER_TURN_WINDOW_FRACTION: f64 = 0.30;
const MIN_RESULT_SIZE_CHARS: i64 = 8_000;
const MIN_TURN_BUDGET_CHARS: i64 = 16_000;

/// A char threshold that may be unlimited (Python float("inf")).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BudgetThreshold {
    Infinite,
    Chars(usize),
}

impl BudgetThreshold {
    pub fn is_infinite(&self) -> bool {
        matches!(self, BudgetThreshold::Infinite)
    }
}

/// Immutable budget constants for the 3-layer tool result persistence
/// system.
#[derive(Debug, Clone)]
pub struct BudgetConfig {
    pub default_result_size: usize,
    pub turn_budget: usize,
    pub preview_size: usize,
    pub tool_overrides: HashMap<String, usize>,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        BudgetConfig {
            default_result_size: DEFAULT_RESULT_SIZE_CHARS,
            turn_budget: DEFAULT_TURN_BUDGET_CHARS,
            preview_size: DEFAULT_PREVIEW_SIZE_CHARS,
            tool_overrides: HashMap::new(),
        }
    }
}

/// The default budget — matches the current hardcoded behavior exactly.
pub static DEFAULT_BUDGET: once_cell::sync::Lazy<BudgetConfig> =
    once_cell::sync::Lazy::new(BudgetConfig::default);

impl BudgetConfig {
    /// Resolve the persistence threshold for a tool.
    ///
    /// Priority: pinned -> tool_overrides -> registry per-tool -> default.
    /// The registry per-tool value is capped at `default_result_size`.
    pub fn resolve_threshold(&self, tool_name: &str) -> BudgetThreshold {
        for (name, th) in PINNED_THRESHOLDS {
            if *name == tool_name {
                return *th;
            }
        }
        if let Some(override_val) = self.tool_overrides.get(tool_name) {
            return BudgetThreshold::Chars(*override_val);
        }
        let registry_value =
            crate::registry::registry().get_max_result_size(tool_name, Some(self.default_result_size as i64));
        let registry_value = registry_value as usize;
        if registry_value >= i64::MAX as usize {
            return BudgetThreshold::Infinite; // registry sentinel for inf is not used; keep numeric path
        }
        BudgetThreshold::Chars(registry_value.min(self.default_result_size))
    }
}

/// Build a BudgetConfig scaled to the active model's context window.
pub fn budget_for_context_window(context_length: Option<i64>) -> BudgetConfig {
    let Some(context_length) = context_length else {
        return BudgetConfig::default();
    };
    if context_length <= 0 {
        return BudgetConfig::default();
    }
    let window_chars = context_length * CHARS_PER_TOKEN;
    let per_result = ((window_chars as f64) * PER_RESULT_WINDOW_FRACTION) as i64;
    let per_turn = ((window_chars as f64) * PER_TURN_WINDOW_FRACTION) as i64;
    let per_result = per_result.max(MIN_RESULT_SIZE_CHARS).min(DEFAULT_RESULT_SIZE_CHARS as i64);
    let per_turn = per_turn.max(MIN_TURN_BUDGET_CHARS).min(DEFAULT_TURN_BUDGET_CHARS as i64);
    BudgetConfig {
        default_result_size: per_result as usize,
        turn_budget: per_turn as usize,
        preview_size: DEFAULT_PREVIEW_SIZE_CHARS,
        tool_overrides: HashMap::new(),
    }
}
