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

use hermes_agent::run_agent::{
    is_ephemeral_scaffolding, qwen_platform_tokens, qwen_portal_headers, qwen_portal_headers_for,
    routermint_headers, safe_session_filename_component, DB_PERSISTED_MARKER,
    EPHEMERAL_SCAFFOLDING_FLAGS, MAX_TOOL_WORKERS, QWEN_CODE_VERSION,
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
