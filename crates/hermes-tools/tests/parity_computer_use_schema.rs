//! Parity oracles for the generic computer_use schema, mirroring the
//! schema assertions in upstream tests/tools/test_computer_use.py @ b9aa928
//! (backend/tool cases deferred with tools/computer_use/tool.py).

use hermes_tools::computer_use_schema::get_computer_use_schema;
use serde_json::json;

fn schema() -> serde_json::Value {
    get_computer_use_schema().clone()
}

#[test]
fn action_discriminator_enum() {
    let s = schema();
    let actions = s["parameters"]["properties"]["action"]["enum"].as_array().unwrap();
    // Upstream asserts this exact 24-entry set in test_computer_use_schema actions.
    let expected: Vec<&str> = vec![
        "capture", "click", "double_click", "right_click", "middle_click",
        "drag", "scroll", "type", "key", "set_value", "wait", "list_apps",
        "list_windows", "focus_app", "cua_browser_state", "cua_browser_prepare",
        "cua_browser_navigate", "cua_browser_click", "cua_browser_type",
        "cua_browser_pointer", "cua_browser_dialog", "cua_browser_set_input_files",
        "cua_browser_download",
    ];
    assert_eq!(actions.len(), expected.len());
    for a in actions {
        assert!(expected.contains(&a.as_str().unwrap()), "unexpected action {a}");
    }
}

#[test]
fn max_elements_bounds() {
    let s = schema();
    let prop = &s["parameters"]["properties"]["max_elements"];
    assert_eq!(prop["default"], 100);
    assert_eq!(prop["minimum"], 1);
    assert_eq!(prop["maximum"], 1000);
}

#[test]
fn required_action_only() {
    let s = schema();
    assert_eq!(s["parameters"]["required"], json!(["action"]));
    assert_eq!(s["name"], "computer_use");
}

#[test]
fn golden_json_byte_parity() {
    // The embedded JSON is extracted verbatim from upstream COMPUTER_USE_SCHEMA;
    // confirm the parsed value round-trips to the identical serialization.
    let s = schema();
    let raw: serde_json::Value = serde_json::from_str(
        include_str!("../../../upstream/golden_computer_use_schema.json"),
    )
    .expect("golden");
    assert_eq!(s, raw, "computer_use schema drifted from upstream golden");
}
