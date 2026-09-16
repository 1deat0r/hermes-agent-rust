//! Source-derived parity oracle for
//! `plugins/model-providers/nvidia/__init__.py` @ 5d59366.
//!
//! Declarative fields plus `TestNvidiaProfile::test_prepare_messages_*` in
//! `tests/providers/test_provider_profiles.py` (the oracle for the
//! `NvidiaProviderProfile.prepare_messages` copy-on-write strip).
//! Tier: unit.

use std::sync::Mutex;

use hermes_providers::registry::{get_provider_profile, list_providers, reset_registry_for_tests};

static NVIDIA_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn nvidia_profile_fields_and_aliases_match_upstream() {
    let _guard = NVIDIA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("nvidia").expect("NVIDIA profile must be registered");

    assert_eq!(profile.name, "nvidia");
    assert_eq!(
        profile.aliases,
        ["nvidia-nim", "nim", "build-nvidia", "nemotron"]
    );
    assert_eq!(profile.env_vars, ["NVIDIA_API_KEY"]);
    assert_eq!(profile.display_name, "NVIDIA NIM");
    assert_eq!(profile.description, "NVIDIA NIM — accelerated inference");
    assert_eq!(profile.signup_url, "https://build.nvidia.com/");
    assert_eq!(
        profile.fallback_models,
        [
            "nvidia/llama-3.1-nemotron-70b-instruct",
            "nvidia/llama-3.3-70b-instruct"
        ]
    );
    assert_eq!(profile.base_url, "https://integrate.api.nvidia.com/v1");
    assert_eq!(profile.get_hostname(), "integrate.api.nvidia.com");
    assert_eq!(profile.default_max_tokens, Some(16_384));
    for alias in ["nvidia-nim", "nim", "build-nvidia", "nemotron"] {
        assert_eq!(get_provider_profile(alias).unwrap().name, "nvidia");
    }
}

#[test]
fn nvidia_is_listed_once_by_canonical_name() {
    let _guard = NVIDIA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let names: Vec<_> = list_providers()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names.iter().filter(|name| *name == "nvidia").count(), 1);
}
#[test]
fn nvidia_prepare_messages_strips_tool_result_names() {
    // PARITY: `TestNvidiaProfile::test_prepare_messages_strips_tool_result_names`
    // (`tests/providers/test_provider_profiles.py` @ 5d59366).
    use serde_json::json;
    let _guard = NVIDIA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("nvidia").expect("NVIDIA profile must be registered");
    let msgs = vec![
        json!({"role": "user", "content": "run a command"}),
        json!({"role": "assistant", "content": null, "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "terminal", "arguments": "{}"}}]}),
        json!({"role": "tool", "name": "terminal", "tool_name": "terminal", "tool_call_id": "call_1", "content": "ok"}),
    ];
    let result = profile.prepare_messages(&msgs);
    assert!(!result[2].as_object().unwrap().contains_key("name"));
    assert!(!result[2].as_object().unwrap().contains_key("tool_name"));
    assert_eq!(
        result[2],
        json!({"role": "tool", "tool_call_id": "call_1", "content": "ok"})
    );
    // Copy-on-write: the caller's input is never mutated.
    assert_eq!(msgs[2]["name"], json!("terminal"));
    assert_eq!(msgs[2]["tool_name"], json!("terminal"));
    // Untouched messages keep their value.
    assert_eq!(result[0], msgs[0]);
    assert_eq!(result[1], msgs[1]);
}
#[test]
fn nvidia_prepare_messages_passthrough_without_tool_result_names() {
    // PARITY: `TestNvidiaProfile::test_prepare_messages_passthrough_without_tool_result_names`.
    // Upstream returns the identical list object (`is`); Rust returns an
    // equal Vec (identity has no cross-seam analog), so value equality plus
    // input immutability is the asserted contract.
    use serde_json::json;
    let _guard = NVIDIA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("nvidia").expect("NVIDIA profile must be registered");
    let msgs = vec![json!({"role": "tool", "tool_call_id": "call_1", "content": "ok"})];
    assert_eq!(profile.prepare_messages(&msgs), msgs);
}
#[test]
fn nvidia_prepare_messages_strips_name_only_shapes() {
    // Hardening: a tool message carrying only one of the two stripped keys
    // still strips (upstream `_needs_strip` is an `or` over both keys).
    use serde_json::json;
    let _guard = NVIDIA_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let profile = get_provider_profile("nvidia").expect("NVIDIA profile must be registered");
    let msgs = vec![
        json!({"role": "tool", "name": "terminal", "tool_call_id": "call_1", "content": "ok"}),
    ];
    let result = profile.prepare_messages(&msgs);
    assert_eq!(
        result[0],
        json!({"role": "tool", "tool_call_id": "call_1", "content": "ok"})
    );
}
