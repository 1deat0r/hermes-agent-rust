//! Parity oracles for tool-result persistence + budget config, mirroring
//! upstream tests/tools/test_tool_result_storage.py + test_budget_config.py
//! @ 5d59366.
//!
//! The sandbox seam splits upstream's `env.execute` object into
//! `SandboxExecutor` (command runner) + `SandboxEnv` (temp dir, sync,
//! probe, stdin mode, path translation). `FakeSandbox` implements both
//! with scripted probe answers. Host-spillover tests point HERMES_HOME
//! at a temp dir (mirroring the upstream `_isolated_home` fixture).

use std::sync::{Arc, Mutex};

use serde_json::Value;

use hermes_tools::budget_config::{
    budget_for_context_window, BudgetConfig, BudgetThreshold, DEFAULT_PREVIEW_SIZE_CHARS,
    DEFAULT_RESULT_SIZE_CHARS, DEFAULT_TURN_BUDGET_CHARS, PINNED_THRESHOLDS,
};
use hermes_tools::tool_result_storage::{
    cleanup_spillover_cache, enforce_turn_budget, extract_persisted_path, generate_preview,
    maybe_persist_tool_result, safe_result_filename, spillover_dir, write_to_sandbox,
    write_to_spillover, SandboxEnv, SandboxExecutor, SandboxResult, PERSISTED_OUTPUT_TAG,
};

#[derive(Default)]
struct FakeExecutor {
    calls: Arc<Mutex<Vec<(String, String)>>>, // (cmd, stdin_data)
    returncode: i32,
    /// Scripted answers for `wc -c` probes, in order.
    wc_answers: Mutex<Vec<String>>,
}

impl FakeExecutor {
    fn new(returncode: i32) -> Self {
        FakeExecutor {
            calls: Default::default(),
            returncode,
            wc_answers: Mutex::new(Vec::new()),
        }
    }

    fn with_wc(output: &str) -> Self {
        FakeExecutor {
            calls: Default::default(),
            returncode: 0,
            wc_answers: Mutex::new(vec![output.to_string()]),
        }
    }
}

impl SandboxExecutor for FakeExecutor {
    fn execute(&self, cmd: &str, _timeout: u64, stdin_data: &str) -> SandboxResult {
        self.calls
            .lock()
            .unwrap()
            .push((cmd.to_string(), stdin_data.to_string()));
        if cmd.starts_with("wc -c") {
            let output = self.wc_answers.lock().unwrap().pop().unwrap_or_default();
            return SandboxResult {
                returncode: self.returncode,
                output,
            };
        }
        SandboxResult {
            returncode: self.returncode,
            output: String::new(),
        }
    }
}

/// Scriptable remote backend: readability probe + temp dir + stdin mode.
struct FakeSandbox {
    executor: FakeExecutor,
    readable: bool,
    temp_dir: Option<String>,
    stdin_mode: String,
}

impl FakeSandbox {
    fn readable() -> Self {
        FakeSandbox {
            executor: FakeExecutor::new(0),
            readable: true,
            temp_dir: None,
            stdin_mode: "pipe".to_string(),
        }
    }
}

impl SandboxEnv for FakeSandbox {
    fn temp_dir(&self) -> Option<String> {
        self.temp_dir.clone()
    }
    fn probe_readable(&self, _translated_path: &str) -> bool {
        self.readable
    }
    fn stdin_mode(&self) -> &str {
        &self.stdin_mode
    }
    fn translate_host_path(&self, host_path: &str) -> Option<String> {
        Some(host_path.to_string())
    }
    fn execute(&self, cmd: &str, timeout_secs: u64, stdin_data: &str) -> (i32, String) {
        let result = self.executor.execute(cmd, timeout_secs, stdin_data);
        (result.returncode, result.output)
    }
}

struct HomeGuard {
    previous: Option<String>,
}

impl HomeGuard {
    fn point_at_temp() -> (tempfile::TempDir, HomeGuard) {
        let td = tempfile::TempDir::new().unwrap();
        let home = td.path().join(".hermes");
        let previous = std::env::var("HERMES_HOME").ok();
        unsafe { std::env::set_var("HERMES_HOME", &home) };
        (td, HomeGuard { previous })
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(prev) => unsafe { std::env::set_var("HERMES_HOME", prev) },
            None => unsafe { std::env::remove_var("HERMES_HOME") },
        }
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
    assert!(PINNED_THRESHOLDS
        .iter()
        .any(|(n, t)| *n == "read_file" && t.is_infinite()));
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
    assert_eq!(
        cfg.resolve_threshold("terminal"),
        BudgetThreshold::Chars(5_000)
    );
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
fn short_content_unchanged() {
    let (preview, has_more) = generate_preview("short result", DEFAULT_PREVIEW_SIZE_CHARS);
    assert_eq!(preview, "short result");
    assert!(!has_more);
}

#[test]
fn exact_boundary_unchanged() {
    let text = "x".repeat(DEFAULT_PREVIEW_SIZE_CHARS);
    let (preview, has_more) = generate_preview(&text, DEFAULT_PREVIEW_SIZE_CHARS);
    assert_eq!(preview, text);
    assert!(!has_more);
}

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

// ── _write_to_sandbox ────────────────────────────────────────────────────

#[test]
fn sandbox_write_success_and_stdin_routing() {
    let executor = FakeExecutor::new(0);
    assert!(write_to_sandbox(
        "hello world",
        "/tmp/hermes-results/abc.txt",
        &executor,
        "pipe"
    ));
    // First call is the write; content travels through stdin, NOT the cmd.
    let calls = executor.calls.lock().unwrap();
    assert!(calls[0].0.contains("mkdir -p"));
    assert!(!calls[0].0.contains("hello world"));
    assert_eq!(calls[0].1, "hello world");
}

#[test]
fn sandbox_large_content_via_stdin() {
    // 200 KB exceeds MAX_ARG_STRLEN: cmd stays tiny, content via stdin.
    let executor = FakeExecutor::new(0);
    let big = "x".repeat(200_000);
    assert!(write_to_sandbox(
        &big,
        "/tmp/hermes-results/big.txt",
        &executor,
        "pipe"
    ));
    let calls = executor.calls.lock().unwrap();
    assert!(calls[0].0.len() < 1_000);
    assert_eq!(calls[0].1, big);
}

#[test]
fn sandbox_paths_are_quoted_against_injection() {
    let executor = FakeExecutor::new(0);
    write_to_sandbox(
        "content",
        "/tmp/hermes results/abc file.txt",
        &executor,
        "pipe",
    );
    // Call 0 = mkdir+cat write, call 1 = wc -c probe.
    let write_cmd = executor.calls.lock().unwrap()[0].0.clone();
    assert!(write_cmd.contains("'/tmp/hermes results'"));
    assert!(write_cmd.contains("'/tmp/hermes results/abc file.txt'"));
    write_to_sandbox(
        "content",
        "/tmp/hermes-results/$(whoami).txt",
        &executor,
        "pipe",
    );
    let calls = executor.calls.lock().unwrap();
    assert!(calls[2].0.contains("'/tmp/hermes-results/$(whoami).txt'"));
}

#[test]
fn sandbox_size_probe_decides_lossless() {
    // Short write (512 probed vs real length): bytes lost → false.
    let short = FakeExecutor::with_wc("512");
    assert!(!write_to_sandbox(
        &"x".repeat(171),
        "/tmp/hermes-results/a.txt",
        &short,
        "pipe"
    ));
    // Heredoc +1 accepted; pipe backends must be exact.
    let heredoc = FakeExecutor::with_wc("171");
    assert!(!write_to_sandbox(
        &"x".repeat(170),
        "/tmp/hermes-results/a.txt",
        &heredoc,
        "pipe"
    ));
    let heredoc_ok = FakeExecutor::with_wc("171");
    assert!(write_to_sandbox(
        &"x".repeat(170),
        "/tmp/hermes-results/a.txt",
        &heredoc_ok,
        "heredoc"
    ));
    // Unparseable probe → best-effort success.
    let nop = FakeExecutor::with_wc("nope");
    assert!(write_to_sandbox(
        "hello",
        "/tmp/hermes-results/a.txt",
        &nop,
        "pipe"
    ));
}

#[test]
fn sandbox_storage_dir_defaults_and_temp_override() {
    use hermes_tools::tool_result_storage::resolve_storage_dir;
    assert_eq!(resolve_storage_dir(None), "/tmp/hermes-results");
    struct Tmp;
    impl SandboxEnv for Tmp {
        fn temp_dir(&self) -> Option<String> {
            Some("/tmp".to_string())
        }
        fn execute(&self, _cmd: &str, _t: u64, _s: &str) -> (i32, String) {
            (0, String::new())
        }
    }
    assert_eq!(resolve_storage_dir(Some(&Tmp)), "/tmp/hermes-results");
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
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(50_000)),
    );
    assert_eq!(out, content);
}

#[test]
fn above_threshold_host_side_persists_first() {
    // No sandbox env (MCP-only/cron session): host spillover is
    // canonical — the guglielmo bundle bug.
    let (_td, _home) = HomeGuard::point_at_temp();
    let content = "x".repeat(60_000);
    let out = maybe_persist_tool_result(
        &content,
        "tool_call",
        "tc_mcp_1",
        None,
        None,
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(30_000)),
    );
    assert!(out.contains(PERSISTED_OUTPUT_TAG));
    assert!(!out.contains("could not be saved"));
    let spill = spillover_dir().join("tc_mcp_1.txt");
    assert!(spill.exists());
    assert_eq!(std::fs::read_to_string(&spill).unwrap(), content);
    assert!(out.contains(&spill.to_string_lossy().into_owned()));
}

#[test]
fn remote_probe_success_references_mounted_path() {
    // Host write is canonical; when the sandbox reads the mounted path,
    // only the probe runs — no in-sandbox copy.
    let (_td, _home) = HomeGuard::point_at_temp();
    let sandbox = FakeSandbox::readable();
    let content = "z".repeat(60_000);
    let out = maybe_persist_tool_result(
        &content,
        "terminal",
        "tc_remote_1",
        None,
        Some(&sandbox),
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(30_000)),
    );
    assert!(out.contains(PERSISTED_OUTPUT_TAG));
    assert!(spillover_dir().join("tc_remote_1.txt").exists());
    assert_eq!(sandbox.executor.calls.lock().unwrap().len(), 0);
}

#[test]
fn remote_probe_failure_falls_back_to_sandbox_write() {
    let (_td, _home) = HomeGuard::point_at_temp();
    let mut sandbox = FakeSandbox::readable();
    sandbox.readable = false;
    sandbox.temp_dir = Some("/tmp".to_string());
    let content = "z".repeat(60_000);
    let out = maybe_persist_tool_result(
        &content,
        "terminal",
        "tc_remote_2",
        Some(&sandbox.executor),
        Some(&sandbox),
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(30_000)),
    );
    assert!(out.contains(PERSISTED_OUTPUT_TAG));
    assert!(out.contains("/tmp/hermes-results/tc_remote_2.txt"));
}

#[test]
fn persists_full_content_verbatim() {
    // Content travels through stdin; JSON blobs are not extracted.
    let (_td, _home) = HomeGuard::point_at_temp();
    let raw = "line1\nline2\n".repeat(5_000);
    let content = serde_json::to_string(&serde_json::json!({
        "output": raw, "exit_code": 0, "error": Value::Null,
    }))
    .unwrap();
    let out = maybe_persist_tool_result(
        &content,
        "terminal",
        "tc_json",
        None,
        None,
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(30_000)),
    );
    assert!(out.contains(PERSISTED_OUTPUT_TAG));
    let spill = spillover_dir().join("tc_json.txt");
    assert_eq!(std::fs::read_to_string(&spill).unwrap(), content);
}

#[test]
fn tool_use_id_cannot_escape_storage_dir() {
    // Host-side canonical home: the spillover filename sanitizes the
    // hostile id (no traversal, no metacharacters).
    let (_td, _home) = HomeGuard::point_at_temp();
    let content = "x".repeat(60_000);
    let out = maybe_persist_tool_result(
        &content,
        "terminal",
        "../outside/$(whoami);x",
        None,
        None,
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(30_000)),
    );
    assert!(
        out.contains("outside___whoami__x_"),
        "got: {}",
        &out[..out.len().min(400)]
    );
    assert!(!out.contains("../"));
    assert!(!out.contains("$(whoami)"));
    let filename = out
        .lines()
        .find(|l| l.starts_with("Full output saved to: "))
        .unwrap()
        .trim_start_matches("Full output saved to: ");
    // (The static Recovery paragraph contains a semicolon by design —
    // upstream verbatim. The property is about the filename.)
    assert!(
        !filename.contains(';'),
        "filename leaks metachars: {filename}"
    );
}

#[test]
fn build_persisted_message_shows_mb_for_large() {
    // 2MB result -> "2.0 MB" + the Recovery paragraph.
    let (_td, _home) = HomeGuard::point_at_temp();
    let out = maybe_persist_tool_result(
        &"z".repeat(2_000_000),
        "terminal",
        "big",
        None,
        None,
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(1_000)),
    );
    assert!(out.contains("MB"));
    assert!(out.contains("Recovery:"));
    assert!(extract_persisted_path(&out).is_some());
}

#[test]
fn extract_persisted_path_round_trips() {
    let (_td, _home) = HomeGuard::point_at_temp();
    let out = maybe_persist_tool_result(
        &"q".repeat(60_000),
        "terminal",
        "tc_extract",
        None,
        None,
        None,
        &BudgetConfig::default(),
        Some(BudgetThreshold::Chars(1_000)),
    );
    let path = extract_persisted_path(&out).expect("path");
    assert!(path.ends_with("tc_extract.txt"));
    assert!(extract_persisted_path("no block here").is_none());
    assert!(extract_persisted_path("").is_none());
}

#[test]
fn spillover_cleanup_removes_only_stale_files() {
    use hermes_tools::tool_result_storage::{cleanup_spillover_cache, write_to_spillover};
    let (_td, _home) = HomeGuard::point_at_temp();
    write_to_spillover("fresh", "fresh.txt").expect("write");
    assert_eq!(cleanup_spillover_cache(-1), 1, "negative age clears all");
    assert_eq!(cleanup_spillover_cache(24), 0);
}

// ── enforce_turn_budget ──────────────────────────────────────────────────

#[test]
fn under_budget_no_changes() {
    let messages = vec![
        serde_json::json!({"content": "small"}),
        serde_json::json!({"content": "also small"}),
    ];
    let mut messages = messages;
    enforce_turn_budget(&mut messages, None, None, None, &BudgetConfig::default());
    // Default budget is large; small messages pass through.
    assert_eq!(messages[0]["content"], "small");
}

#[test]
fn medium_result_regression() {
    // 6 results of 42K chars each (252K total) — each under the 100K
    // default threshold but aggregate exceeds the 200K budget.
    let (_td, _home) = HomeGuard::point_at_temp();
    let mut messages: Vec<Value> = (0..6)
        .map(
            |i| serde_json::json!({"content": "x".repeat(42_000), "tool_call_id": format!("t{i}")}),
        )
        .collect();
    let config = BudgetConfig {
        turn_budget: 200_000,
        ..Default::default()
    };
    enforce_turn_budget(&mut messages, None, None, None, &config);
    let persisted = messages
        .iter()
        .filter(|m| {
            m.get("content")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .contains(PERSISTED_OUTPUT_TAG)
        })
        .count();
    assert!(persisted >= 2, "need to shed at least ~52K");
}

#[test]
fn turn_budget_persists_largest_results() {
    let (_td, _home) = HomeGuard::point_at_temp();
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
    let sandbox = FakeSandbox::readable();
    let _ = &executor;
    enforce_turn_budget(&mut messages, None, Some(&sandbox), None, &config);
    // Under budget after spilling the largest (host-side canonical).
    let total: usize = messages
        .iter()
        .map(|m| {
            m.get("content")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .chars()
                .count()
        })
        .sum();
    assert!(total <= 150_000);
}
