//! Parity oracles for tool-result persistence + budget config, mirroring
//! upstream tests/tools/test_tool_result_storage.py + test_budget_config.py
//! @ b9aa928.

use std::sync::{Arc, Mutex};

use hermes_tools::budget_config::{
    budget_for_context_window, BudgetConfig, BudgetThreshold, DEFAULT_PREVIEW_SIZE_CHARS,
    DEFAULT_RESULT_SIZE_CHARS, DEFAULT_TURN_BUDGET_CHARS, PINNED_THRESHOLDS,
};
use hermes_tools::tool_result_storage::{
    enforce_turn_budget, generate_preview, maybe_persist_tool_result, safe_result_filename,
    SandboxExecutor, SandboxResult, PERSISTED_OUTPUT_TAG,
};

#[derive(Default)]
struct FakeExecutor {
    calls: Arc<Mutex<Vec<(String, String)>>>, // (cmd, stdin_data)
    returncode: i32,
}

impl FakeExecutor {
    fn new(returncode: i32) -> Self {
        FakeExecutor { calls: Default::default(), returncode }
    }
}

impl SandboxExecutor for FakeExecutor {
    fn execute(&self, cmd: &str, _timeout: u64, stdin_data: &str) -> SandboxResult {
        self.calls.lock().unwrap().push((cmd.to_string(), stdin_data.to_string()));
        SandboxResult { returncode: self.returncode }
    }
}

// ── budget_config ────────────────────────────────────────────────────────

#[test]
fn module_constants_have_expected_values() {
    assert_eq!(DEFAULT_RESULT_SIZE_CHARS, 100_000);
    assert_eq!(DEFAULT_PREVIEW_SIZE_CHARS, 1_500);
    assert_eq!(DEFAULT_TURN_BUDGET_CHARS, 200_000);
}

#[test]
fn pinned_thresholds_include_read_file_infinite() {
    assert!(PINNED_THRESHOLDS.iter().any(|(n, t)| *n == "read_file" && t.is_infinite()));
    assert!(!PINNED_THRESHOLDS.is_empty());
}

#[test]
fn default_budget_matches_defaults() {
    let cfg = BudgetConfig::default();
    assert_eq!(cfg.default_result_size, DEFAULT_RESULT_SIZE_CHARS);
    assert_eq!(cfg.turn_budget, DEFAULT_TURN_BUDGET_CHARS);
    assert_eq!(cfg.preview_size, DEFAULT_PREVIEW_SIZE_CHARS);
}

#[test]
fn threshold_priority_pinned_overrides_registry_default() {
    // Pinned: read_file always infinite.
    let cfg = BudgetConfig::default();
    assert!(cfg.resolve_threshold("read_file").is_infinite());

    // Overrides beat the registry.
    let mut cfg = BudgetConfig::default();
    let mut overrides = std::collections::HashMap::new();
    overrides.insert("terminal".to_string(), 5_000usize);
    cfg.tool_overrides = overrides;
    assert_eq!(cfg.resolve_threshold("terminal"), BudgetThreshold::Chars(5_000));
}

#[test]
fn budget_for_context_window_scales_and_floors() {
    // Large window clamps to the historical defaults.
    let large = budget_for_context_window(Some(200_000));
    assert_eq!(large.default_result_size, DEFAULT_RESULT_SIZE_CHARS);
    assert_eq!(large.turn_budget, DEFAULT_TURN_BUDGET_CHARS);
    // Small window floors.
    let small = budget_for_context_window(Some(10_000));
    assert!(small.default_result_size >= 8_000);
    assert!(small.turn_budget >= 16_000);
    assert!(small.default_result_size < DEFAULT_RESULT_SIZE_CHARS);
}

// ── generate_preview ─────────────────────────────────────────────────────

#[test]
fn preview_truncates_at_last_newline() {
    let (preview, has_more) = generate_preview("aaaa\nbbbb\ncccc", 8);
    assert!(has_more);
    assert!(preview.len() <= 8);
    // No newline at all: plain truncation.
    let (p2, more2) = generate_preview("abcdefghij", 4);
    assert!(more2);
    assert_eq!(p2, "abcd");
}

// ── maybe_persist_tool_result ────────────────────────────────────────────

#[test]
fn below_threshold_returns_unchanged() {
    let content = "small result".to_string();
    let out = maybe_persist_tool_result(
        &content,
        "terminal",
        "tc_123",
        None,
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(50_000)),
    );
    assert_eq!(out, content);
}

#[test]
fn infinite_threshold_returns_unchanged() {
    let content = "x".repeat(200_000);
    let out = maybe_persist_tool_result(
        &content,
        "read_file",
        "tc_rd",
        None,
        None,
        &BudgetConfig::default(),
        None,
    );
    assert_eq!(out, content);
}

#[test]
fn above_threshold_with_env_persists() {
    let executor = FakeExecutor::new(0);
    let content = "x".repeat(60_000);
    let out = maybe_persist_tool_result(
        &content,
        "terminal",
        "tc_456",
        Some(&executor),
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(30_000)),
    );
    assert!(out.contains(PERSISTED_OUTPUT_TAG));
    assert!(out.contains("tc_456.txt"));
    assert!(out.chars().count() < content.chars().count());
    assert_eq!(executor.calls.lock().unwrap().len(), 1);
    // Content routed through stdin_data verbatim.
    assert_eq!(executor.calls.lock().unwrap()[0].1, content);
}

#[test]
fn no_env_falls_back_to_inline_truncation() {
    let content = "y".repeat(40_000);
    let out = maybe_persist_tool_result(
        &content,
        "terminal",
        "tc_789",
        None,
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(10_000)),
    );
    assert!(!out.contains(PERSISTED_OUTPUT_TAG));
    assert!(out.contains("Truncated"));
}

#[test]
fn tool_use_id_cannot_escape_storage_dir() {
    let executor = FakeExecutor::new(0);
    let content = "x".repeat(60_000);
    let out = maybe_persist_tool_result(
        &content,
        "terminal",
        "../outside/$(whoami);x",
        Some(&executor),
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(30_000)),
    );
    assert!(out.contains("/tmp/hermes-results/outside"), "got: {}", &out[..out.len().min(300)]);
    assert!(!out.contains("/tmp/hermes-results/../"));
    assert!(!out.contains("$(whoami)"));
    assert!(!out.contains(';'));
    let calls = executor.calls.lock().unwrap();
    assert!(!calls[0].0.contains("$(whoami)"));
    assert!(!calls[0].0.contains(';'));
}

#[test]
fn safe_filename_sanitizes() {
    assert_eq!(safe_result_filename("tc_123"), "tc_123.txt");
    let f = safe_result_filename("../outside/$(whoami);x");
    assert!(!f.contains('/'));
    assert!(!f.contains(".."));
    assert!(!f.contains("$("));
    assert!(!f.contains(';'));
}

#[test]
fn build_persisted_message_shows_mb_for_large() {
    // 2MB result -> "2.0 MB"
    let out = maybe_persist_tool_result(
        &"z".repeat(2_000_000),
        "terminal",
        "big",
        Some(&FakeExecutor::new(0)),
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(1_000)),
    );
    assert!(out.contains("MB"));
}

// ── enforce_turn_budget ──────────────────────────────────────────────────

#[test]
fn turn_budget_persists_largest_results() {
    let executor = FakeExecutor::new(0);
    let mut messages = vec![
        serde_json::json!({"content": "a".repeat(90_000), "tool_call_id": "t1"}),
        serde_json::json!({"content": "b".repeat(90_000), "tool_call_id": "t2"}),
        serde_json::json!({"content": "c".repeat(30_000), "tool_call_id": "t3"}),
    ];
    let config = BudgetConfig {
        turn_budget: 150_000,
        ..Default::default()
    };
    enforce_turn_budget(&mut messages, Some(&executor), None, &config);
    // Under budget after spilling the largest.
    let total: usize = messages
        .iter()
        .map(|m| m.get("content").and_then(serde_json::Value::as_str).unwrap_or("").chars().count())
        .sum();
    assert!(total <= 150_000);
    // The largest got persisted (small replacement block).
    assert!(messages[0]["content"].as_str().unwrap().contains(PERSISTED_OUTPUT_TAG));
    // Already-persisted results are skipped on re-run.
    enforce_turn_budget(&mut messages, Some(&executor), None, &config);
    assert_eq!(executor.calls.lock().unwrap().len(), 1);
}
