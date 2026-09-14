//! Parity oracles for the first `run_agent` section (module-level pure
//! helpers), mirroring upstream cases @ b9aa928:
//! - `tests/run_agent/test_dropped_tool_call_recovery.py:154-193`
//!   (`_dropped_toolcall_nudge` classified ephemeral)
//! - `tests/agent/test_verification_stop_caching.py:36-46`
//!   (`_pre_verify_synthetic` / `_verification_stop_synthetic` ephemeral,
//!   plain user/assistant messages not)
//! - `tests/run_agent/test_run_agent.py:527-537`
//!   (traversal sanitizer: segment-safe, legit IDs untouched, no collisions)
//!
//! Evidence tiers: `unit` for mirrored cases; `unit/source-derived`
//! (gap noted) for the header platform mapping, falsy-flag matrix, and
//! truncation/empty bounds, which no upstream case pins.

use std::collections::HashMap;

use hermes_agent::credential_pool::{CredentialPool, PoolStrategy, PooledCredential};
use hermes_agent::run_agent::{
    is_ephemeral_scaffolding, launch_cwd_for_session, pool_may_recover_from_rate_limit,
    qwen_platform_tokens, qwen_portal_headers, qwen_portal_headers_for, routermint_headers,
    safe_session_filename_component, session_source_for_agent, StreamErrorEvent,
    DB_PERSISTED_MARKER, EPHEMERAL_SCAFFOLDING_FLAGS, MAX_TOOL_WORKERS, QWEN_CODE_VERSION,
};
use serde_json::{json, Map, Value};

fn msg(value: Value) -> Map<String, Value> {
    value
        .as_object()
        .cloned()
        .expect("test message must be an object")
}

// ── module constants ─────────────────────────────────────────────────

#[test]
fn scaffolding_flags_pinned_in_order() {
    // Order follows the pin source table (`run_agent.py` ~234-256);
    // upstream tests pin membership (2 flags), not order.
    assert_eq!(
        EPHEMERAL_SCAFFOLDING_FLAGS,
        [
            "_empty_recovery_synthetic",
            "_empty_terminal_sentinel",
            "_thinking_prefill",
            "_verification_stop_synthetic",
            "_pre_verify_synthetic",
            "_kanban_stop_synthetic",
            "_dropped_toolcall_nudge",
        ]
    );
    assert_eq!(MAX_TOOL_WORKERS, 8);
    assert_eq!(DB_PERSISTED_MARKER, "_db_persisted");
    assert_eq!(QWEN_CODE_VERSION, "0.14.1");
}

// ── _is_ephemeral_scaffolding (mirrored) ─────────────────────────────

#[test]
fn dropped_toolcall_nudge_is_ephemeral() {
    // tests/run_agent/test_dropped_tool_call_recovery.py:188-193
    assert!(is_ephemeral_scaffolding(&msg(
        json!({"role": "user", "content": "nudge", "_dropped_toolcall_nudge": true})
    )));
}

#[test]
fn verification_synthetic_flags_are_ephemeral() {
    // tests/agent/test_verification_stop_caching.py:38-43
    assert!(is_ephemeral_scaffolding(&msg(
        json!({"role": "user", "content": "[System: run tests]", "_pre_verify_synthetic": true})
    )));
    assert!(is_ephemeral_scaffolding(&msg(
        json!({"role": "user", "content": "[System: run tests]", "_verification_stop_synthetic": true})
    )));
}

#[test]
fn plain_messages_are_not_ephemeral() {
    // tests/agent/test_verification_stop_caching.py:45-46
    assert!(!is_ephemeral_scaffolding(&msg(
        json!({"role": "user", "content": "hi"})
    )));
    assert!(!is_ephemeral_scaffolding(&msg(
        json!({"role": "assistant", "content": "premature done"})
    )));
    // Upstream returns False for non-dicts; the empty map is the
    // closest representable shape.
    assert!(!is_ephemeral_scaffolding(&Map::new()));
}

#[test]
fn falsy_flag_values_are_not_ephemeral() {
    // Source-derived: mirrors Python truthiness of `msg.get(flag)`.
    for falsy in [json!(false), json!(0), json!(""), json!([]), json!({})] {
        assert!(
            !is_ephemeral_scaffolding(&msg(json!({"role": "user", "_thinking_prefill": falsy}))),
            "{falsy} must read as unset"
        );
    }
    assert!(is_ephemeral_scaffolding(&msg(
        json!({"role": "user", "_thinking_prefill": "partial content"})
    )));
    // `null` reads exactly like a missing key (Python `msg.get(flag)`
    // returns `None`), and nonzero numbers are truthy.
    assert!(!is_ephemeral_scaffolding(&msg(
        json!({"role": "user", "_thinking_prefill": null})
    )));
    assert!(is_ephemeral_scaffolding(&msg(
        json!({"role": "user", "_thinking_prefill": 1})
    )));
    assert!(!is_ephemeral_scaffolding(&msg(
        json!({"role": "user", "_thinking_prefill": 0.0})
    )));
}

// ── _safe_session_filename_component (mirrored) ──────────────────────

#[test]
fn traversal_ids_stay_in_one_segment() {
    // tests/run_agent/test_run_agent.py:527-534
    let f = safe_session_filename_component;
    for raw in ["../../etc/passwd", "/abs/path", "..\\win\\trav", "a/b/c"] {
        let out = f(raw);
        assert!(!out.contains('/'), "{out}");
        assert!(!out.contains('\\'), "{out}");
        assert!(!out.contains(".."), "{out}");
    }
}

#[test]
fn legit_ids_pass_through_and_collisions_resolve() {
    // tests/run_agent/test_run_agent.py:536-537
    let f = safe_session_filename_component;
    assert_eq!(f("api-abc123def456"), "api-abc123def456");
    assert_ne!(f("../a"), f("../b"));
    // Exact digests from the Python oracle (sha256[:12] of the raw ID).
    assert_eq!(f("../a"), "a_61b4c98bfb92");
    assert_eq!(f("../../etc/passwd"), "etc_passwd_3754d6cb3a38");
}

#[test]
fn filename_class_matches_python_word_set() {
    // Calibrated against the live Python oracle (`re` `\w` keeps exactly
    // letters + numbers + `_`): Join_Control, marks, and symbols collapse.
    let f = safe_session_filename_component;
    assert_eq!(f("caf\u{e9}abc-123_X"), "caf\u{e9}abc-123_X");
    assert_eq!(f("\u{4e2d}\u{00b2}"), "\u{4e2d}\u{00b2}");
    assert_eq!(f("a\u{200c}b"), "a_b_b350b9e65eef"); // ZWNJ collapses
    assert_eq!(f("x😀y"), "x_y_d506f8eb09b3");
    // Python `str.strip()` also strips FS/GS/RS/US; Rust `trim` does not.
    assert_eq!(f("\x1cid\x1c"), "id");
}

#[test]
fn empty_and_long_ids_have_stable_bounds() {
    // Source-derived bounds: empty never yields "" (downstream joins it
    // into a filename); over-long IDs truncate to 96 chars + hash.
    let empty = safe_session_filename_component("");
    assert!(empty.starts_with("session_"), "{empty}");
    assert_eq!(empty, "session_e3b0c44298fc");
    let long = safe_session_filename_component(&"a".repeat(200));
    assert_eq!(long.len(), 96 + 1 + 12, "{long}");
    assert!(long.starts_with(&"a".repeat(96)));
}

// ── _qwen_portal_headers (source-derived grammar) ────────────────────

#[test]
fn qwen_headers_match_cli_shape_on_linux() {
    let headers = qwen_portal_headers_for("linux", "x86_64");
    assert_eq!(
        headers.get("User-Agent").map(String::as_str),
        Some("QwenCode/0.14.1 (linux; x86_64)")
    );
    assert_eq!(
        headers.get("X-DashScope-CacheControl").map(String::as_str),
        Some("enable")
    );
    assert_eq!(
        headers.get("X-DashScope-UserAgent"),
        headers.get("User-Agent")
    );
    assert_eq!(
        headers.get("X-DashScope-AuthType").map(String::as_str),
        Some("qwen-oauth")
    );
}

#[test]
fn qwen_platform_tokens_reproduce_cpython_spellings() {
    // `platform.system().lower()` says darwin; `platform.machine()`
    // says arm64 on Apple Silicon. Rust reports macos/aarch64.
    assert_eq!(
        qwen_platform_tokens("macos", "aarch64"),
        ("darwin".to_string(), "arm64".to_string())
    );
    assert_eq!(
        qwen_platform_tokens("linux", "x86_64"),
        ("linux".to_string(), "x86_64".to_string())
    );
    // Linux ARM passes through verbatim (CPython reports `aarch64` there).
    assert_eq!(
        qwen_platform_tokens("linux", "aarch64"),
        ("linux".to_string(), "aarch64".to_string())
    );
    // CPython reports kernel arch names on Windows; system is lowercased
    // exactly as upstream `.lower()`s `platform.system()`.
    assert_eq!(
        qwen_platform_tokens("windows", "x86_64"),
        ("windows".to_string(), "AMD64".to_string())
    );
    assert_eq!(
        qwen_platform_tokens("windows", "aarch64"),
        ("windows".to_string(), "ARM64".to_string())
    );
    assert_eq!(
        qwen_platform_tokens("Darwin", "x86_64"),
        ("darwin".to_string(), "x86_64".to_string())
    );
    assert!(qwen_portal_headers_for("macos", "aarch64")["User-Agent"].contains("(darwin; arm64)"));
}

#[test]
fn qwen_live_wrapper_reports_this_process() {
    // The live wrapper takes `std::env::consts`; pin shape only, never
    // platform spellings (those belong to `qwen_platform_tokens` above).
    let headers = qwen_portal_headers();
    assert_eq!(headers.len(), 4);
    let ua = &headers["User-Agent"];
    assert!(ua.starts_with(&format!("QwenCode/{QWEN_CODE_VERSION} (")));
    assert!(ua.contains("; "));
    assert_eq!(headers.get("X-DashScope-UserAgent"), Some(ua));
}

// ── _pool_may_recover_from_rate_limit ─────────────────────────────────

const POOL_NOW: f64 = 1_700_000_000.0;

fn pool_entry(id: &str, key: &str, priority: i32) -> PooledCredential {
    PooledCredential::new("openrouter", id, key, priority)
}

fn fresh_pool(size: i32) -> CredentialPool {
    CredentialPool::new(
        "openrouter",
        (0..size)
            .map(|i| pool_entry(&format!("key-{i}"), &format!("sk-test-{i}"), i))
            .collect(),
        PoolStrategy::LeastUsed,
    )
}

#[test]
fn none_pool_returns_false() {
    // tests/run_agent/test_provider_fallback.py:247-248
    assert!(!pool_may_recover_from_rate_limit(None, POOL_NOW));
}

#[test]
fn multi_entry_pool_recovers() {
    // tests/agent/test_gemini_fast_fallback.py:23-24 — the MagicMock
    // (`has_available=True`, 3 entries) becomes a real 3-entry pool.
    let pool = fresh_pool(3);
    assert!(pool_may_recover_from_rate_limit(Some(&pool), POOL_NOW));
}

#[test]
fn exhausted_pool_skips_rotation() {
    // tests/agent/test_gemini_fast_fallback.py:29-32 — both entries
    // benched through the real exhaustion path, so nothing is available.
    let mut pool = fresh_pool(2);
    for id in ["key-0", "key-1"] {
        pool.mark_exhausted_and_rotate(Some(429), None, None, Some(id), None, POOL_NOW);
    }
    assert!(!pool.has_available(POOL_NOW));
    assert!(!pool_may_recover_from_rate_limit(Some(&pool), POOL_NOW));
}

#[test]
fn single_entry_pool_has_nowhere_to_rotate() {
    // Source-derived: the upstream docstring demands "more than one
    // entry to rotate to"; no upstream case pins the single-entry arm.
    let pool = fresh_pool(1);
    assert!(pool.has_available(POOL_NOW));
    assert!(!pool_may_recover_from_rate_limit(Some(&pool), POOL_NOW));
}

// ── session establishment (mirrored + source-derived) ────────────────

// Env vars are process-global: serialize the env-mutating tests.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct EnvGuard {
    _guard: std::sync::MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn lock() -> Self {
        EnvGuard {
            _guard: ENV_LOCK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            saved: Vec::new(),
        }
    }
    fn set(mut self, key: &'static str, value: &str) -> Self {
        self.saved.push((key, std::env::var(key).ok()));
        std::env::set_var(key, value);
        self
    }
    fn unset(mut self, key: &'static str) -> Self {
        self.saved.push((key, std::env::var(key).ok()));
        std::env::remove_var(key);
        self
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[test]
fn session_source_context_overrides_platform() {
    // tests/run_agent/test_session_source.py:15-22 — the gateway
    // contextvar value arrives as `context_source` (gateway layer seam).
    let _guard = EnvGuard::lock().unset("HERMES_SESSION_SOURCE");
    assert_eq!(session_source_for_agent(Some("tui"), Some("tool")), "tool");
}

#[test]
fn session_source_falls_back_to_platform() {
    // tests/run_agent/test_session_source.py:25-28
    let _guard = EnvGuard::lock().unset("HERMES_SESSION_SOURCE");
    assert_eq!(session_source_for_agent(Some("tui"), None), "tui");
}

#[test]
fn session_source_env_beats_platform_and_defaults_cli() {
    // Source-derived: the env arm has no upstream case; platform/"" and
    // None follow `platform or "cli"`.
    {
        let _guard = EnvGuard::lock().set("HERMES_SESSION_SOURCE", "cron");
        assert_eq!(session_source_for_agent(Some("tui"), None), "cron");
    }
    {
        let _guard = EnvGuard::lock().unset("HERMES_SESSION_SOURCE");
        assert_eq!(session_source_for_agent(None, None), "cli");
        assert_eq!(session_source_for_agent(Some(""), None), "cli");
        // Blank context counts as unset, exactly like `str(source or "")`.
        assert_eq!(session_source_for_agent(Some("tui"), Some("  ")), "tui");
    }
}

#[test]
fn launch_cwd_only_for_local_cli() {
    // Source-derived: no upstream case pins `_launch_cwd_for_session`.
    {
        let _guard = EnvGuard::lock().unset("TERMINAL_ENV");
        assert_eq!(launch_cwd_for_session("gateway"), None);
        assert_eq!(launch_cwd_for_session("cron"), None);
        let cwd = launch_cwd_for_session("cli").expect("local cli records cwd");
        assert_eq!(cwd, std::env::current_dir().unwrap().to_string_lossy());
    }
    {
        let _guard = EnvGuard::lock().set("TERMINAL_ENV", "docker");
        assert_eq!(launch_cwd_for_session("cli"), None);
    }
    {
        let _guard = EnvGuard::lock().set("TERMINAL_ENV", " LOCAL ");
        assert!(launch_cwd_for_session("cli").is_some());
    }
}

// ── _StreamErrorEvent ────────────────────────────────────────────────

#[test]
fn stream_error_event_carries_sdk_shaped_body() {
    // Mirrors the oracle assertions in
    // tests/run_agent/test_codex_xai_oauth_recovery.py:98-102 (message in
    // the error string, provider message in body["error"]["message"]);
    // the streaming harness itself needs the unported AIAgent loop.
    let event = StreamErrorEvent::new(
        "do not have an active Grok subscription",
        Some("forbidden"),
        None::<String>,
        None,
    );
    let rendered = format!("{event}");
    assert!(rendered.contains("do not have an active Grok subscription"));
    assert_eq!(
        event.body["error"]["message"],
        serde_json::json!("do not have an active Grok subscription")
    );
    assert_eq!(event.body["error"]["code"], serde_json::json!("forbidden"));
    assert_eq!(event.body["error"]["param"], serde_json::Value::Null);
    assert_eq!(event.body["error"]["type"], serde_json::json!("error"));
    assert_eq!(event.code.as_deref(), Some("forbidden"));
    assert_eq!(event.status_code, None);
}

// ── _routermint_headers ──────────────────────────────────────────────

#[test]
fn routermint_user_agent_carries_caller_version() {
    // Upstream interpolates `hermes_cli.__version__`; the version is a
    // caller argument here (see module docs), so pin the interpolation.
    let headers: HashMap<String, String> = routermint_headers("0.20.0");
    assert_eq!(
        headers.get("User-Agent").map(String::as_str),
        Some("HermesAgent/0.20.0")
    );
    assert_eq!(headers.len(), 1);
}
