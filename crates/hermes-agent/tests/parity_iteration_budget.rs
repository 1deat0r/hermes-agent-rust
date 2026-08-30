//! Parity tests for `agent/iteration_budget.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); these cases derive from the upstream code as oracle, including
//! the docstring-documented contract (subagents hold independent budgets so
//! the parent's cap can be exceeded in total).

use std::sync::Arc;

use hermes_agent::iteration_budget::IterationBudget;

#[test]
fn consume_allows_up_to_the_cap_then_rejects() {
    let budget = IterationBudget::new(3);
    assert!(budget.consume());
    assert!(budget.consume());
    assert!(budget.consume());
    assert!(!budget.consume(), "cap exhausted");
    assert_eq!(budget.used(), 3);
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn refund_gives_back_and_never_goes_negative() {
    let budget = IterationBudget::new(2);
    budget.refund();
    assert_eq!(budget.used(), 0, "refund on a fresh budget is a no-op");
    assert!(budget.consume());
    budget.refund();
    assert_eq!(budget.used(), 0);
    assert_eq!(budget.remaining(), 2);
    // Refund enables another consume.
    assert!(budget.consume());
}

#[test]
fn remaining_clamps_at_zero_when_max_total_shrinks() {
    // `max_total` is a plain public attribute upstream, so callers can
    // lower it; the max(0, ...) clamp keeps remaining non-negative.
    let mut budget = IterationBudget::new(2);
    assert!(budget.consume());
    assert!(budget.consume());
    // Lowering the cap below `used` leaves remaining clamped at 0 and
    // consume() stays rejected (used >= max_total).
    budget.max_total = 1;
    assert_eq!(budget.remaining(), 0);
    assert!(!budget.consume());
    budget.max_total = -5;
    assert_eq!(budget.remaining(), 0);
}

#[test]
fn budgets_are_independent_per_agent() {
    // Subagents hold independent budgets: total iterations across parent +
    // subagents can exceed the parent's cap.
    let parent = IterationBudget::new(2);
    let subagent = IterationBudget::new(50);
    let _ = parent.consume();
    let _ = parent.consume();
    assert_eq!(parent.remaining(), 0);
    assert!(subagent.consume());
    assert_eq!(subagent.remaining(), 49);
}

#[test]
fn counter_is_thread_safe() {
    // `threading.Lock` semantics: concurrent consume() never exceeds the
    // cap.
    let budget = Arc::new(IterationBudget::new(100));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let budget = Arc::clone(&budget);
            std::thread::spawn(move || {
                let mut granted = 0;
                for _ in 0..50 {
                    if budget.consume() {
                        granted += 1;
                    }
                }
                granted
            })
        })
        .collect();
    let total: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
    assert_eq!(total, 100);
    assert_eq!(budget.used(), 100);
    assert_eq!(budget.remaining(), 0);
}
