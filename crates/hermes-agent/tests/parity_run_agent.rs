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
    clean_error_message, codex_silent_hang_hint, coerce_api_error_detail,
    copilot_requires_responses_api, decorate_xai_entitlement_error, is_azure_openai_url,
    is_codex_backend, is_copilot_provider, is_copilot_url, is_direct_openai_url,
    is_entitlement_failure, is_ephemeral_scaffolding, is_github_copilot_url, is_openrouter_url,
    launch_cwd_for_session, mask_api_key_for_logs, max_tokens_param, model_requires_responses_api,
    pool_may_recover_from_rate_limit, provider_model_requires_responses_api, qwen_platform_tokens,
    qwen_portal_headers, qwen_portal_headers_for, requested_output_cap_from_api_kwargs,
    routermint_headers, safe_session_filename_component, session_source_for_agent,
    summarize_api_error, ApiErrorShape, StreamErrorEvent, DB_PERSISTED_MARKER,
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

// ── provider/URL predicates (mirrored + source-derived) ───────────────

#[test]
fn direct_openai_url_matches_native_host_only() {
    // tests/agent/test_direct_provider_url_detection.py
    assert!(!is_direct_openai_url("https://api.openai.com.example/v1"));
    assert!(is_direct_openai_url("https://api.openai.com/v1"));
}

#[test]
fn azure_url_uses_substring_detection() {
    // tests/run_agent/test_run_agent.py::TestGpt5ApiModeRouting::test_is_azure_openai_url_detection
    assert!(is_azure_openai_url(
        "https://foo.openai.azure.com/openai/v1"
    ));
    assert!(!is_azure_openai_url("https://api.openai.com/v1"));
    assert!(!is_azure_openai_url("https://openrouter.ai/api/v1"));
    assert!(is_azure_openai_url(
        "https://my-resource.openai.azure.com/openai/v1"
    ));
}

#[test]
fn github_copilot_url_matches_host_and_subdomains() {
    // Source-derived: no upstream case pins this predicate.
    assert!(is_github_copilot_url("https://api.githubcopilot.com/v1"));
    assert!(is_github_copilot_url("https://proxy.githubcopilot.com/v1"));
    assert!(!is_github_copilot_url(
        "https://api.githubcopilot.com.example/v1"
    ));
    assert!(!is_github_copilot_url(""));
    assert!(!is_github_copilot_url("https://api.openai.com/v1"));
}

#[test]
fn openrouter_and_copilot_url_forms() {
    // Source-derived: explicit-URL forms of the self-reading predicates.
    assert!(is_openrouter_url("https://openrouter.ai/api/v1"));
    assert!(!is_openrouter_url("https://api.openai.com/v1"));
    assert!(is_copilot_url("https://api.githubcopilot.com/v1"));
    assert!(is_copilot_url("https://models.github.ai/v1"));
    assert!(is_copilot_url("HTTPS://API.GITHUBCOPILOT.COM/V1"));
    assert!(!is_copilot_url("https://openrouter.ai/api/v1"));
}

#[test]
fn copilot_provider_covers_alias_spellings() {
    // Source-derived, overlapping the existing oracle
    // tests/agent/test_turn_retry_state.py::test_copilot_provider_check_accepts_alias_spellings
    // (aliases + URL-fallback + openrouter negative); the whitespace
    // variant is source-consistent but oracle-unpinned.
    for alias in ["copilot", "github-copilot", "github", " Copilot "] {
        assert!(is_copilot_provider(
            Some(alias),
            "https://openrouter.ai/api/v1"
        ));
    }
    assert!(!is_copilot_provider(
        Some("openai"),
        "https://openrouter.ai/api/v1"
    ));
    assert!(is_copilot_provider(
        Some("openai"),
        "https://api.githubcopilot.com/v1"
    ));
    assert!(is_copilot_provider(None, "https://models.github.ai/v1"));
}

#[test]
fn codex_backend_needs_mode_host_and_path() {
    // Source-derived: explicit-argument form of the field-reading check.
    assert!(is_codex_backend(
        "codex_responses",
        "https://chatgpt.com/backend-api/codex"
    ));
    assert!(!is_codex_backend(
        "chat_completions",
        "https://chatgpt.com/backend-api/codex"
    ));
    assert!(!is_codex_backend(
        "codex_responses",
        "https://api.openai.com/v1"
    ));
    assert!(!is_codex_backend(
        "codex_responses",
        "https://chatgpt.com/backend-api/other"
    ));
}

#[test]
fn responses_api_routing_per_provider() {
    // tests/run_agent/test_run_agent.py::TestGpt5ApiModeRouting (nous arm):
    // Nous serves GPT-5.x on chat completions.
    assert!(!provider_model_requires_responses_api(
        "openai/gpt-5.5",
        Some("nous")
    ));
    // Generic GPT-5 models upgrade (the rule the routing tests rely on
    // when the provider is not Nous/Azure).
    assert!(provider_model_requires_responses_api(
        "openai/gpt-5.5",
        None
    ));
    assert!(provider_model_requires_responses_api("gpt-5.4-mini", None));
    // Generic custom endpoints stay conservative; non-GPT-5 never upgrades.
    assert!(!provider_model_requires_responses_api(
        "openai/gpt-5.5",
        Some("custom")
    ));
    assert!(!provider_model_requires_responses_api("gpt-4o", None));
    assert!(!provider_model_requires_responses_api(
        "claude-opus-4-6",
        None
    ));
    // Copilot applies the ported hermes_cli rule (pure `re`, no layer
    // violation): GPT-5+ except mini.
    assert!(provider_model_requires_responses_api(
        "gpt-5.5",
        Some("copilot")
    ));
    assert!(!provider_model_requires_responses_api(
        "gpt-5-mini",
        Some("copilot")
    ));
    assert!(!provider_model_requires_responses_api(
        "claude-opus-4-6",
        Some("copilot")
    ));
}

#[test]
fn copilot_rule_matches_opencode_logic() {
    // Direct oracle: hermes_cli/models.py::_should_use_copilot_responses_api.
    // Case-sensitive `re.match` — `GPT-5` does not match.
    assert!(copilot_requires_responses_api("gpt-5.5"));
    assert!(copilot_requires_responses_api("gpt-5.5-codex"));
    assert!(copilot_requires_responses_api("gpt-6"));
    assert!(!copilot_requires_responses_api("gpt-5-mini"));
    assert!(!copilot_requires_responses_api("claude-opus-4-6"));
    assert!(!copilot_requires_responses_api("GPT-5.5"));
    assert!(!copilot_requires_responses_api(""));
    assert!(model_requires_responses_api("openai/gpt-5.4"));
    assert!(!model_requires_responses_api("gpt-4.1"));
}

#[test]
fn hang_hint_fires_only_for_gpt55_on_codex() {
    // tests/run_agent/test_codex_silent_hang_hint.py (positives).
    let hint = codex_silent_hang_hint(
        "codex_responses",
        "openai-codex",
        "https://chatgpt.com/backend-api/codex",
        Some("gpt-5.5"),
        "gpt-5.5",
    )
    .expect("hint fires for bare gpt-5.5 on codex");
    assert!(hint.contains("gpt-5.4"));
    assert!(hint.contains("gpt-5.3-codex"));
    assert!(hint.contains("gpt-5.4-codex"));
    assert!(hint.contains("fallback chain"));
    assert!(hint.contains("'gpt-5.5'"));
    // Quote-containing model that still matches renders with Python
    // double quotes (oracle `repr()`; verified pattern fires).
    let quoted = codex_silent_hang_hint(
        "codex_responses",
        "openai-codex",
        "https://chatgpt.com/backend-api/codex",
        Some("it's-gpt-5.5-x"),
        "gpt-5.5",
    )
    .expect("quote-adjacent token still matches");
    assert!(quoted.contains("\"it's-gpt-5.5-x\""));
    assert!(codex_silent_hang_hint(
        "codex_responses",
        "openai-codex",
        "https://chatgpt.com/backend-api/codex",
        Some("openai/gpt-5.5"),
        "gpt-5.5",
    )
    .is_some());
    // Source-derived negatives (the oracle's negative section is empty):
    // wrong mode, non-5.5 model, gpt-5.50 boundary, model-fallback arm.
    assert_eq!(
        codex_silent_hang_hint(
            "chat_completions",
            "openai-codex",
            "https://chatgpt.com/backend-api/codex",
            Some("gpt-5.5"),
            "gpt-5.5",
        ),
        None
    );
    assert_eq!(
        codex_silent_hang_hint(
            "codex_responses",
            "openai-codex",
            "https://chatgpt.com/backend-api/codex",
            Some("gpt-5.4"),
            "gpt-5.4",
        ),
        None
    );
    assert_eq!(
        codex_silent_hang_hint(
            "codex_responses",
            "openai-codex",
            "https://chatgpt.com/backend-api/codex",
            Some("gpt-5.50"),
            "gpt-5.50",
        ),
        None
    );
    assert!(codex_silent_hang_hint(
        "codex_responses",
        "openai-codex",
        "https://chatgpt.com/backend-api/codex",
        None,
        "gpt-5.5-codex",
    )
    .is_some());
    assert_eq!(
        codex_silent_hang_hint(
            "codex_responses",
            "openai",
            "https://api.openai.com/v1",
            Some("gpt-5.5"),
            "gpt-5.5",
        ),
        None
    );
}

#[test]
fn max_tokens_key_prefers_new_kwarg_for_new_families() {
    // tests/run_agent/test_run_agent.py::TestMaxTokensParam (direct arm).
    assert_eq!(
        max_tokens_param(4096, "https://api.openai.com/v1", "gpt-4o-mini"),
        ("max_completion_tokens", 4096)
    );
    // Source-derived: azure/copilot URLs, model-name fallback, legacy.
    assert_eq!(
        max_tokens_param(
            4096,
            "https://foo.openai.azure.com/openai/v1",
            "gpt-4o-mini"
        ),
        ("max_completion_tokens", 4096)
    );
    assert_eq!(
        max_tokens_param(4096, "https://api.githubcopilot.com/v1", "gpt-4o-mini"),
        ("max_completion_tokens", 4096)
    );
    assert_eq!(
        max_tokens_param(4096, "https://openrouter.ai/api/v1", "openai/gpt-5.5"),
        ("max_completion_tokens", 4096)
    );
    assert_eq!(
        max_tokens_param(4096, "https://openrouter.ai/api/v1", "claude-opus-4-6"),
        ("max_tokens", 4096)
    );
}

#[test]
fn output_cap_reads_first_positive_kwarg() {
    // Source-derived: no upstream case pins this staticmethod.
    let cap = |pairs: &[(&str, Value)]| {
        let map: Map<String, Value> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        requested_output_cap_from_api_kwargs(&map)
    };
    assert_eq!(cap(&[("max_tokens", json!(4096))]), Some(4096));
    assert_eq!(
        cap(&[
            ("max_output_tokens", json!(100)),
            ("max_tokens", json!(4096))
        ]),
        Some(100)
    );
    assert_eq!(
        cap(&[("max_output_tokens", json!(0)), ("max_tokens", json!(5))]),
        Some(5)
    );
    assert_eq!(cap(&[("max_tokens", json!("abc"))]), None);
    assert_eq!(cap(&[("max_tokens", json!(-3))]), None);
    assert_eq!(cap(&[("max_tokens", json!(3.7))]), Some(3));
    assert_eq!(cap(&[("max_tokens", json!(" 64 "))]), Some(64));
    assert_eq!(cap(&[("max_tokens", json!(true))]), Some(1));
    // CPython underscore separators: single between digits only.
    assert_eq!(cap(&[("max_tokens", json!("1_0"))]), Some(10));
    assert_eq!(cap(&[("max_tokens", json!("_1"))]), None);
    assert_eq!(cap(&[("max_tokens", json!("1__0"))]), None);
    assert_eq!(cap(&[]), None);
    assert_eq!(cap(&[("other", json!(9))]), None);
}

// ── entitlement / error-text helpers (mirrored + source-derived) ─────

fn body_map(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

#[test]
fn entitlement_matches_real_xai_bodies() {
    // tests/run_agent/test_codex_xai_oauth_recovery.py Fix D parametrize:
    // both captured wire bodies classify True at 403.
    for message in [
        "You have either run out of available resources or do not have an \
         active Grok subscription. Manage at https://grok.com",
        "The caller does not have permission to execute the specified \
         operation for grok-4.3",
    ] {
        assert!(
            is_entitlement_failure(
                Some(&body_map(&[
                    ("message", Value::String(message.to_string())),
                    ("reason", Value::String("permission_denied".to_string())),
                ])),
                Some(403)
            ),
            "{message}"
        );
    }
}

#[test]
fn entitlement_rejects_other_statuses() {
    // tests/run_agent/test_codex_xai_oauth_recovery.py:369-378
    let body = body_map(&[(
        "message",
        Value::String("do not have an active Grok subscription".to_string()),
    )]);
    assert!(!is_entitlement_failure(Some(&body), Some(500)));
    assert!(!is_entitlement_failure(Some(&body), Some(429)));
    assert!(!is_entitlement_failure(Some(&body), Some(200)));
}

#[test]
fn entitlement_disambiguator_prefers_refresh() {
    // Source-derived from the #29344 docstring: stale-token signals
    // route to credential refresh, never surface as entitlement.
    let stale = body_map(&[(
        "error",
        Value::String(
            "The caller does not have permission [WKE=unauthenticated:expired]".to_string(),
        ),
    )]);
    assert!(!is_entitlement_failure(Some(&stale), Some(403)));
    let stale_phrase = body_map(&[(
        "message",
        Value::String("OAuth2 access token could not be validated".to_string()),
    )]);
    assert!(!is_entitlement_failure(Some(&stale_phrase), Some(401)));
    // Status None is allowed; empty and non-mapping bodies are not.
    let body = body_map(&[(
        "message",
        Value::String("do not have an active Grok subscription".to_string()),
    )]);
    assert!(is_entitlement_failure(Some(&body), None));
    assert!(!is_entitlement_failure(Some(&body_map(&[])), Some(403)));
    assert!(!is_entitlement_failure(None, Some(403)));
    // Conjunction arms need both halves.
    let half = body_map(&[(
        "message",
        Value::String("out of available resources".to_string()),
    )]);
    assert!(!is_entitlement_failure(Some(&half), Some(403)));
    // List bodies render with Python `repr()` separators: a phrase split
    // across elements must NOT fuse into a match (executed oracle vector).
    let split = body_map(&[
        ("message", json!(["out of available", "resources"])),
        ("reason", Value::String("grok".to_string())),
    ]);
    assert!(!is_entitlement_failure(Some(&split), Some(403)));
    // ...but the joined phrase in one element still matches.
    let joined = body_map(&[(
        "message",
        Value::String("out of available resources for grok".to_string()),
    )]);
    assert!(is_entitlement_failure(Some(&joined), Some(403)));
}

#[test]
fn xai_decorate_appends_hint_once() {
    // Partial mirror of oracle Fix B (`test_codex_xai_oauth_recovery.py`):
    // the pinned substrings below plus original-text survival; the
    // non-accusatory-tone constraints are prose, not asserts.
    let detail = "The caller does not have permission for grok-4.3";
    let decorated = decorate_xai_entitlement_error(detail);
    assert!(decorated.starts_with(detail));
    assert!(decorated.contains("X Premium+ does NOT include"));
    assert!(decorated.contains("standalone SuperGrok subscribers"));
    assert!(decorated.contains("no Grok subscription"));
    assert!(decorated.contains("tier doesn't include this model"));
    assert!(decorated.contains("quota is exhausted"));
    assert!(decorated.contains("https://grok.com/?_s=usage"));
    assert!(decorated.contains("`/model`"));
    assert_eq!(decorate_xai_entitlement_error(&decorated), decorated);
    assert_eq!(decorate_xai_entitlement_error(""), "");
    assert_eq!(
        decorate_xai_entitlement_error("plain 429 rate limit"),
        "plain 429 rate limit"
    );
}

#[test]
fn coerce_detail_prefers_message_then_recurses() {
    // Source-derived: no upstream case pins the coercion table.
    assert_eq!(coerce_api_error_detail(&json!("raw text")), "raw text");
    assert_eq!(
        coerce_api_error_detail(&json!({"message": "  keep  ", "code": "drop"})),
        "  keep  "
    );
    assert_eq!(
        coerce_api_error_detail(&json!({"code": {"message": "nested"}})),
        "nested"
    );
    assert_eq!(
        coerce_api_error_detail(&json!(["a", "", {"message": "b"}])),
        "a; b"
    );
    assert_eq!(coerce_api_error_detail(&json!(null)), "");
    assert_eq!(coerce_api_error_detail(&json!(true)), "True");
    assert_eq!(coerce_api_error_detail(&json!(3.0)), "3.0");
    assert_eq!(
        coerce_api_error_detail(&json!({"b": 1, "a": {"y": 2}})),
        "{\"a\": {\"y\": 2}, \"b\": 1}"
    );
    assert_eq!(
        coerce_api_error_detail(&json!({"f": 1e28})),
        "{\"f\": 1e+28}"
    );
}

#[test]
fn mask_api_key_keeps_head_and_tail() {
    // `TestMaskApiKey` intent for the long-key shape (the pin's literal
    // key is redacted to 13 chars and contradicts its own startswith
    // assert); thresholds and the exact string below are
    // implementation-derived from the source arms.
    assert_eq!(mask_api_key_for_logs(None), None);
    assert_eq!(mask_api_key_for_logs(Some("")), None);
    assert_eq!(
        mask_api_key_for_logs(Some("short-key")),
        Some("***".to_string())
    );
    assert_eq!(
        mask_api_key_for_logs(Some("123456789012")),
        Some("***".to_string())
    );
    let masked = mask_api_key_for_logs(Some("sk-or-v1-abcdefghijklmnop")).unwrap();
    assert!(masked.starts_with("sk-or-v1"));
    assert!(masked.ends_with("mnop"));
    assert!(masked.contains("..."));
    assert_eq!(masked, "sk-or-v1...mnop");
}

#[test]
fn clean_error_message_collapses_and_caps() {
    // HTML-collapse behavior overlaps `_summarize_api_error`'s no-raw-HTML
    // rule; the exact notice string, 150-cap, and whitespace table are
    // implementation-derived from the source arms.
    assert_eq!(clean_error_message(""), "Unknown error");
    assert_eq!(
        clean_error_message("<!DOCTYPE html><html>oops</html>"),
        "Service temporarily unavailable (HTML error page returned)"
    );
    assert_eq!(
        clean_error_message("  <html>proxy</html>  "),
        "Service temporarily unavailable (HTML error page returned)"
    );
    assert_eq!(clean_error_message("a\n\n  b\tc"), "a b c");
    let long = "x".repeat(200);
    let capped = clean_error_message(&long);
    assert_eq!(capped.len(), 153);
    assert!(capped.ends_with("..."));
    assert_eq!(clean_error_message("fine"), "fine");
}

// ── _summarize_api_error ─────────────────────────────────────────────

fn error_shape<'a>(
    message: &'a str,
    status_code: Option<i64>,
    response_text: Option<&'a str>,
) -> ApiErrorShape<'a> {
    ApiErrorShape {
        message,
        is_value_error: false,
        body: None,
        response_text,
        status_code,
        type_name: "Exception",
    }
}

#[test]
fn empty_body_falls_back_to_response_json() {
    // tests/run_agent/test_summarize_api_error.py: empty SDK body ({})
    // + httpx response.text with error.message surfaces both. The empty
    // dict (not `None`) is the real #36109 trigger shape.
    let empty_body: Map<String, Value> = Map::new();
    let shape = ApiErrorShape {
        message: "",
        body: Some(&empty_body),
        status_code: Some(400),
        response_text: Some(
            "{\"error\": {\"message\": \"model `foo` does not exist\", \"type\": \"invalid_request_error\"}}",
        ),
        ..error_shape("", None, None)
    };
    let summary = summarize_api_error(&shape);
    assert!(summary.contains("HTTP 400"), "{summary}");
    assert!(summary.contains("model `foo` does not exist"), "{summary}");
}

#[test]
fn unreadable_response_falls_back_to_message() {
    // tests/run_agent/test_summarize_api_error.py: raising response.text
    // maps to None (upstream swallows read failures).
    let summary = summarize_api_error(&error_shape(
        "Gemini HTTP 429: quota exceeded",
        Some(429),
        None,
    ));
    assert!(summary.contains("HTTP 429"), "{summary}");
    assert!(
        summary.contains("Gemini HTTP 429: quota exceeded"),
        "{summary}"
    );
}

#[test]
fn cloudflare_challenge_collapses_to_one_liner() {
    // tests/run_agent/test_nonretryable_error_html_summary.py: a padded,
    // title-less challenge page carrying the `_cf_chl_opt` marker must
    // collapse short with no marker leakage (the reported 31-message
    // Discord flood). The marker text makes `len<200` meaningful.
    let html = format!(
        "<!DOCTYPE html>\n<html>\n  <head>\n    <meta http-equiv=\"refresh\" content=\"360\"></head>\n  <body>\n    <div><noscript>Enable JavaScript and cookies to continue</noscript><script>(function(){{window._cf_chl_opt = {{cRay: 'a0ca002c4f91769c', cZone: 'chatgpt.com', md: '{}'}};}})();</script></div>\n  </body>\n</html>\n",
        "x".repeat(400)
    );
    let summary = summarize_api_error(&error_shape(&html, Some(403), None));
    assert!(!summary.to_lowercase().contains("<html"), "{summary}");
    assert!(!summary.to_lowercase().contains("<!doctype"), "{summary}");
    assert!(!summary.contains("_cf_chl_opt"), "{summary}");
    assert!(summary.contains("403"), "{summary}");
    assert!(
        summary.contains("HTML error page (title not found)"),
        "{summary}"
    );
    assert!(summary.len() < 200, "{summary}");
    // The loop-integration second half of that oracle file needs a live
    // turn and stays explicitly out of scope for this unit.
}

#[test]
fn html_title_and_ray_id_survive() {
    // Source-derived: title + Ray ID grammar from the source arms.
    let html = "<html><head><title>Example Domain</title></head><body>Cloudflare Ray ID: <strong>abc123</strong></body></html>";
    let summary = summarize_api_error(&error_shape(html, Some(503), None));
    assert_eq!(summary, "HTTP 503 — Example Domain — Ray abc123");
    // Whitespace-only title stays empty (no placeholder substitution);
    // NBSP after `Ray ID:` still matches (Python `\s` semantics).
    let blank_title = "<html><head><title>   </title></head><body>x</body></html>";
    assert_eq!(
        summarize_api_error(&error_shape(blank_title, Some(500), None)),
        "HTTP 500 — "
    );
    let nbsp_ray = "<html><head><title>T</title></head><body>Cloudflare Ray ID: <strong>r4y</strong></body></html>";
    assert_eq!(
        summarize_api_error(&error_shape(nbsp_ray, Some(500), None)),
        "HTTP 500 — T — Ray r4y"
    );
}

#[test]
fn value_error_and_gemini_arms() {
    // Source-derived: no upstream case pins these arms directly.
    let shape = ApiErrorShape {
        message: "expected ident at line 1 column 5",
        is_value_error: true,
        body: None,
        response_text: None,
        status_code: None,
        type_name: "ValueError",
    };
    let summary = summarize_api_error(&shape);
    assert!(
        summary.starts_with("Malformed provider streaming response: "),
        "{summary}"
    );
    let gemini = ApiErrorShape {
        message: "quota exceeded",
        type_name: "GeminiAPIError",
        ..shape
    };
    // Pre-composed message survives (redact passes secret-free text through).
    assert_eq!(summarize_api_error(&gemini), "quota exceeded");
}

#[test]
fn body_dict_arm_coerces_and_decorates() {
    // Source-derived: SDK body extraction + coerce + xAI decoration.
    let body = body_map(&[(
        "error",
        json!({"message": "do not have an active Grok subscription"}),
    )]);
    let shape = ApiErrorShape {
        message: "ignored",
        body: Some(&body),
        status_code: Some(401),
        ..error_shape("", None, None)
    };
    let summary = summarize_api_error(&shape);
    assert!(
        summary.starts_with("HTTP 401: do not have an active Grok subscription"),
        "{summary}"
    );
    assert!(summary.contains("X Premium+ does NOT include"), "{summary}");
    // Executed oracle vectors: string `error` + top `message` routes to
    // the top message; empty `error.message` falls through to it.
    let routed = body_map(&[("error", json!("boom")), ("message", json!("top"))]);
    let shape = ApiErrorShape {
        message: "ignored",
        body: Some(&routed),
        ..error_shape("", None, None)
    };
    assert_eq!(summarize_api_error(&shape), "top");
    let skipped = body_map(&[("error", json!({"message": ""})), ("message", json!("top"))]);
    let shape = ApiErrorShape {
        message: "ignored",
        body: Some(&skipped),
        ..error_shape("", None, None)
    };
    // No second lookup: a dict `error` with falsy `message` skips the
    // whole arm (verified against the oracle) and falls to the raw
    // fallback — unlike a non-dict `error`, which routes to the top
    // `message` as above.
    assert_eq!(summarize_api_error(&shape), "ignored");
    // Zero status reads as absent, exactly like falsy upstream.
    let shape = ApiErrorShape {
        message: "plain failure",
        status_code: Some(0),
        ..error_shape("", None, None)
    };
    assert_eq!(summarize_api_error(&shape), "plain failure");
    // Whitespace-only response text skips to the fallback arm.
    let shape = ApiErrorShape {
        message: "plain failure",
        response_text: Some("   "),
        ..error_shape("", None, None)
    };
    assert_eq!(summarize_api_error(&shape), "plain failure");
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
    }
    {
        // Explicitly cleared context masks env entirely (gateway
        // `get_session_env` returns set-"" with no os.environ fallback),
        // so the blank falls through to platform, not the set env var.
        let _guard = EnvGuard::lock().set("HERMES_SESSION_SOURCE", "cron");
        assert_eq!(session_source_for_agent(Some("tui"), Some("  ")), "tui");
        assert_eq!(session_source_for_agent(None, Some("")), "cli");
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
    {
        // Empty and whitespace-only backend names mean "local" upstream
        // (`(… or "local")`), so they record the cwd too.
        let _guard = EnvGuard::lock().set("TERMINAL_ENV", "");
        assert!(launch_cwd_for_session("cli").is_some());
    }
    {
        let _guard = EnvGuard::lock().set("TERMINAL_ENV", "   ");
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
    // Remaining constructor arms: `param: Some`, `code: None`,
    // `status_code: Some` all propagate into fields and body.
    let full = StreamErrorEvent::new("boom", None::<String>, Some("reasoning"), Some(429));
    assert_eq!(full.body["error"]["code"], serde_json::Value::Null);
    assert_eq!(full.body["error"]["param"], serde_json::json!("reasoning"));
    assert_eq!(full.param.as_deref(), Some("reasoning"));
    assert_eq!(full.status_code, Some(429));
    assert_eq!(format!("{full}"), "boom");
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
