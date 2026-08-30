//! Per-agent iteration budget — thread-safe consume/refund counter.
//!
//! PARITY: `agent/iteration_budget.py` @ b9aa928 (whole module).
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
