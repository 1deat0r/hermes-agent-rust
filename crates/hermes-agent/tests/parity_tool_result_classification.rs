// Tier: unit — mirrors tests/agent/test_tool_result_classification.py.

use hermes_agent::tool_result_classification::{
    file_mutation_result_landed, tool_may_have_side_effect, FILE_MUTATING_TOOL_NAMES,
    NO_EFFECT_TOOL_NAMES,
};
use serde_json::json;

/// The payload contract: `json.dumps(...)` of the tool result, i.e. a JSON
/// *string* handed to `file_mutation_result_landed`.
fn payload(value: serde_json::Value) -> serde_json::Value {
    json!(serde_json::to_string(&value).expect("encode"))
}

#[test]
fn write_file_with_nested_lint_error_counts_as_landed() {
    let result = payload(json!(
        {"bytes_written": 12, "lint": {"status": "error", "output": "SyntaxError: invalid syntax"}}
    ));
    assert!(file_mutation_result_landed("write_file", Some(&result)));
}

#[test]
fn side_effect_classification_keeps_session_mutations() {
    assert!(tool_may_have_side_effect("todo"));
    assert!(tool_may_have_side_effect("memory"));
    assert!(tool_may_have_side_effect("write_file"));
    assert!(tool_may_have_side_effect("mcp_unknown"));
    assert!(!tool_may_have_side_effect("read_file"));
    assert!(!tool_may_have_side_effect("web_search"));
}

// Source-derived cases (upstream has no dedicated test for these branches).
#[test]
fn patch_requires_explicit_success_true() {
    assert!(file_mutation_result_landed(
        "patch",
        Some(&payload(json!({"success": true})))
    ));
    assert!(!file_mutation_result_landed(
        "patch",
        Some(&payload(json!({"success": "true"})))
    ));
    assert!(!file_mutation_result_landed(
        "patch",
        Some(&payload(json!({"success": false})))
    ));
    assert!(!file_mutation_result_landed(
        "patch",
        Some(&payload(json!({})))
    ));
}

#[test]
fn error_payload_or_non_json_or_missing_payload_does_not_land() {
    assert!(!file_mutation_result_landed(
        "write_file",
        Some(&payload(
            json!({"error": "permission denied", "bytes_written": 4})
        ))
    ));
    // Truthy-but-not-a-string errors still discard the proof; falsy ones do not.
    assert!(!file_mutation_result_landed(
        "write_file",
        Some(&payload(json!({"error": true, "bytes_written": 4})))
    ));
    assert!(file_mutation_result_landed(
        "write_file",
        Some(&payload(json!({"error": null, "bytes_written": 4})))
    ));
    // A top-level string is a JSON document but not an object.
    assert!(!file_mutation_result_landed(
        "write_file",
        Some(&json!("\"not-an-object\""))
    ));
    // Unparseable text fails open.
    assert!(!file_mutation_result_landed(
        "write_file",
        Some(&json!("  {oops"))
    ));
    // Non-string payloads (already-parsed dicts) fail the `isinstance(result, str)` guard.
    assert!(!file_mutation_result_landed(
        "write_file",
        Some(&json!({"bytes_written": 4}))
    ));
    assert!(!file_mutation_result_landed("write_file", None));
    // Non-file tools never "land", whatever the payload says.
    assert!(!file_mutation_result_landed(
        "read_file",
        Some(&payload(json!({"bytes_written": 4})))
    ));
}

#[test]
fn name_sets_match_upstream_membership() {
    assert_eq!(FILE_MUTATING_TOOL_NAMES, ["write_file", "patch"]);
    assert_eq!(
        NO_EFFECT_TOOL_NAMES,
        [
            "read_file",
            "search_files",
            "session_search",
            "skill_view",
            "skills_list",
            "web_extract",
            "web_search",
            "vision_analyze",
            "browser_snapshot",
            "browser_get_images",
            "browser_console",
            "read_terminal",
        ]
    );
}
