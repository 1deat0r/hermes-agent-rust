//! Parity oracles for delegate_task structured-output schema helpers,
//! mirroring upstream tests/tools/test_delegate_output_schema.py @ b9aa928.
//!
//! Tier: unit.
//!
//! Mirrored here are the helper-module test classes (TestValidateOutput,
//! TestCoerceOutputSchema, TestPromptPlumbing) that exercise this module
//! directly.
//!
//! DEFERRED (not ported here — they exercise `tools/delegate_tool.py`,
//! a different module not in this agent's assignment):
//! - TestToolSchemaSurface (DELEGATE_TASK_SCHEMA static field)
//! - TestRunSingleChildSchemaValidation (_run_single_child retry loop)
//! - TestDelegateTaskDispatch (delegate_task dispatch-time coercion)
//! Those land with the delegate_tool port.
//!
//! JSON-SCHEMA SEAM: the module currently ships the upstream "jsonschema
//! unavailable" degradation path (accept schema / accept parsed JSON
//! unvalidated). Two upstream cases
//! (`test_invalid_json_schema_is_rejected`, `test_json_violating_schema_
//! reports_errors`) depend on real validation and are present as `#[ignore]`
//! tests — flip them on once hermes-tools gets `jsonschema = "0.33"` and the
//! `jsonschema_compat` seam in delegation_output_schema.rs is wired.

use hermes_tools::delegation_output_schema::{
    MAX_SCHEMA_RETRIES, append_output_contract, build_retry_message,
    coerce_output_schema, extract_json_candidate, validate_output,
};
use serde_json::{json, Value};

fn address_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "city": {"type": "string"},
            "zip": {"type": "string"},
        },
        "required": ["city"],
    })
}

// --- TestValidateOutput (validation-engine cases are #[ignore] until the
//     jsonschema crate is wired) -------------------------------------------

#[test]
fn valid_json_matching_schema() {
    let ok_err = validate_output("{\"city\": \"Berlin\"}", &address_schema());
    let (ok, errors) = ok_err;
    assert!(ok);
    assert!(errors.is_empty());
}

#[test]
#[ignore = "requires jsonschema crate (dependency report pending): instance validation currently degrades to accept"]
fn json_violating_schema_reports_errors() {
    let (ok, errors) = validate_output("{\"zip\": \"10115\"}", &address_schema());
    assert!(!ok);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.contains("city")), "errors: {errors:?}");
}

#[test]
fn non_json_text_reports_parse_error() {
    let (ok, errors) = validate_output("I could not produce JSON, sorry.", &address_schema());
    assert!(!ok);
    assert!(!errors.is_empty());
}

#[test]
fn code_fenced_json_is_accepted() {
    let text = "```json\n{\"city\": \"Oslo\"}\n```";
    let (ok, errors) = validate_output(text, &address_schema());
    assert!(ok);
    assert!(errors.is_empty());
}

#[test]
fn json_embedded_in_prose_is_extracted() {
    let text = "Here is the result:\n{\"city\": \"Lima\"}\nHope that helps!";
    let (ok, _) = validate_output(text, &address_schema());
    assert!(ok);
}

#[test]
fn empty_text_is_invalid() {
    let (ok, errors) = validate_output("", &address_schema());
    assert!(!ok);
    assert!(!errors.is_empty());
}

// --- TestCoerceOutputSchema -------------------------------------------------

#[test]
fn valid_schema_passes() {
    let schema = address_schema();
    let (out, err) = coerce_output_schema(schema.clone());
    assert_eq!(out, Some(schema));
    assert!(err.is_none());
}

#[test]
fn none_passes_through() {
    let (out, err) = coerce_output_schema(Value::Null);
    assert!(out.is_none());
    assert!(err.is_none());
}

#[test]
fn non_dict_is_rejected() {
    let (out, err) = coerce_output_schema(json!("not a schema"));
    assert!(out.is_none());
    assert!(err.is_some());
}

#[test]
fn list_is_rejected_with_python_type_name() {
    let (out, err) = coerce_output_schema(json!([1, 2]));
    assert!(out.is_none());
    let err = err.unwrap_or_default();
    assert!(err.contains("list"), "err: {err}");
}

#[test]
fn json_string_double_encode_is_unwrapped() {
    let schema = address_schema();
    let encoded = serde_json::to_string(&schema).unwrap();
    let (out, err) = coerce_output_schema(json!(encoded));
    assert!(err.is_none());
    assert_eq!(out, Some(schema));
}

#[test]
#[ignore = "requires jsonschema crate (dependency report pending): meta-validation currently degrades to accept"]
fn invalid_json_schema_is_rejected() {
    let (out, err) = coerce_output_schema(json!({"type": 42}));
    assert!(out.is_none());
    assert!(err.is_some());
}

// --- TestPromptPlumbing -----------------------------------------------------

#[test]
fn contract_block_carries_schema() {
    let out = append_output_contract(Some("base context"), &address_schema());
    assert!(out.contains("base context"));
    assert!(out.contains("OUTPUT CONTRACT"));
    assert!(out.contains("\"city\""));
    // Exact block layout (byte parity with upstream append_output_contract).
    assert!(out.starts_with("base context\n\nOUTPUT CONTRACT (machine-validated):\n"));
}

#[test]
fn contract_block_without_prior_context() {
    let out = append_output_contract(None, &address_schema());
    assert!(out.contains("OUTPUT CONTRACT"));
    assert!(out.starts_with("OUTPUT CONTRACT (machine-validated):\n"));
    assert!(out.contains("```json code fence is acceptable but not required."));
}

#[test]
fn retry_message_carries_verbatim_errors() {
    let msg = build_retry_message(&["'city' is a required property".to_string()]);
    assert!(msg.contains("'city' is a required property"));
    assert!(msg.contains("JSON"));
}

// --- module constants (upstream MAX_SCHEMA_RETRIES = 1) ----------------------

#[test]
fn retry_budget_is_exactly_one() {
    assert_eq!(MAX_SCHEMA_RETRIES, 1);
}

// --- extract_json_candidate edge cases (upstream code oracle) ----------------

#[test]
fn extract_strips_fence_and_lang_tag() {
    assert_eq!(
        extract_json_candidate("```json\n{\"a\": 1}\n```"),
        "{\"a\": 1}"
    );
}

#[test]
fn extract_finds_outermost_bracket_span() {
    let text = "prefix text {\"a\": {\"nested\": true}} suffix";
    assert_eq!(
        extract_json_candidate(text),
        "{\"a\": {\"nested\": true}}"
    );
}

#[test]
fn extract_passthrough_when_no_delimiters() {
    assert_eq!(extract_json_candidate("no json here"), "no json here");
}
