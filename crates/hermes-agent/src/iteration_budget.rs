//! Per-agent iteration budget — thread-safe consume/refund counter.
//!
//! PARITY: `agent/iteration_budget.py` @ 5d59366 (whole module, incl.
//! `normalize_budget_warning_ratio`).
//!
//! Extracted from `run_agent.py`. Each `AIAgent` instance (parent or
//! subagent) holds an [`IterationBudget`]; the parent's cap comes from
//! `max_iterations` (default 500), each subagent's cap comes from
//! `delegation.max_iterations` (default 50).
//!
//! `run_agent` re-exports `IterationBudget` so existing
//! `from run_agent import IterationBudget` imports keep working unchanged.
//!
//! Each agent (parent or subagent) gets its own `IterationBudget`. The
//! parent's budget is capped at `max_iterations` (default 500). Each
//! subagent gets an independent budget capped at
//! `delegation.max_iterations` (default 50) — this means total iterations
//! across parent + subagents can exceed the parent's cap. Users control the
//! per-subagent limit via `delegation.max_iterations` in config.yaml.
//!
//! `execute_code` (programmatic tool calling) iterations are refunded via
//! [`IterationBudget::refund`] so they don't eat into the budget.

use std::sync::Mutex;

/// Coercible warning-ratio input. Upstream takes `Any`; `None` is Rust
/// `None`, and every other shape maps to a variant (`datetime` has no
/// meaning here — N/A).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RatioInput<'a> {
    /// Python `bool` — always disables, even `True`.
    Bool(bool),
    /// Numbers upstream (`float(value)`).
    Number(f64),
    /// Strings upstream (`float(value)`).
    Text(&'a str),
}

// PARITY: `normalize_budget_warning_ratio` (upstream lines 14-22) — a
/// finite ratio strictly between zero and one, or `None` (feature off).
/// `None`/bool → `None`; unparseable → `None`; non-finite or outside
/// `(0, 1)` → `None`.
pub fn normalize_budget_warning_ratio(value: Option<RatioInput<'_>>) -> Option<f64> {
    let ratio = match value? {
        RatioInput::Bool(_) => return None,
        RatioInput::Number(n) => n,
        RatioInput::Text(s) => s.trim().parse::<f64>().unwrap_or(f64::NAN),
    };
    if ratio.is_finite() && 0.0 < ratio && ratio < 1.0 {
        Some(ratio)
    } else {
        None
    }
}

/// Thread-safe iteration counter for an agent.
///
/// PARITY: `IterationBudget` (upstream lines 24-56). The `threading.Lock`
/// becomes a `Mutex`; `used`/`remaining` are methods here (Rust has no
/// properties).
pub struct IterationBudget {
    /// PARITY: `max_total` (upstream line 29) — a plain public attribute
    /// upstream, so it stays public and is not part of the lock.
    pub max_total: i64,
    used: Mutex<i64>,
}

impl IterationBudget {
    /// PARITY: `__init__(self, max_total: int)` (upstream lines 28-32).
    pub fn new(max_total: i64) -> Self {
        Self {
            max_total,
            used: Mutex::new(0),
        }
    }

    /// Try to consume one iteration. Returns true if allowed.
    ///
    /// PARITY: `consume` (upstream lines 34-40).
    pub fn consume(&self) -> bool {
        let mut used = self.used.lock().unwrap_or_else(|e| e.into_inner());
        if *used >= self.max_total {
            return false;
        }
        *used += 1;
        true
    }

    /// Give back one iteration (e.g. for execute_code turns).
    ///
    /// PARITY: `refund` (upstream lines 42-46) — never goes negative.
    pub fn refund(&self) {
        let mut used = self.used.lock().unwrap_or_else(|e| e.into_inner());
        if *used > 0 {
            *used -= 1;
        }
    }

    /// PARITY: the `used` property (upstream lines 48-50).
    pub fn used(&self) -> i64 {
        *self.used.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// PARITY: the `remaining` property (upstream lines 52-56) — the
    /// `max(0, ...)` clamp keeps a shrunk `max_total` from reporting
    /// negative remaining.
    pub fn remaining(&self) -> i64 {
        (self.max_total - self.used()).max(0)
    }
}
