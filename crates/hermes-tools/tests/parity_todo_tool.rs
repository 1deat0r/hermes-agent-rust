//! Parity oracles for tools/todo_tool.py, mirroring upstream
//! tests/tools/test_todo_tool.py + tests/tools/test_todo_tool_type_coercion.py
//! @ b9aa928 (registry registration cases deferred until the agent loop).

use hermes_tools::todo_tool::{check_todo_requirements, todo_tool, TodoStore, MAX_TODO_CONTENT_CHARS, MAX_TODO_ITEMS};
use serde_json::{json, Value};

fn item(id: &str, content: &str, status: &str) -> Value {
    json!({"id": id, "content": content, "status": status})
}

#[test]
fn write_replaces_list() {
    let mut store = TodoStore::new();
    let items = vec![item("1", "First task", "pending"), item("2", "Second task", "in_progress")];
    let result = store.write(&items, false);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].id, "1");
    assert_eq!(result[1].status, "in_progress");
}

#[test]
fn write_deduplicates_duplicate_ids() {
    let mut store = TodoStore::new();
    let result = store.write(&[
        item("1", "First version", "pending"),
        item("2", "Other task", "pending"),
        item("1", "Latest version", "in_progress"),
    ], false);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].id, "2");
    assert_eq!(result[1].id, "1");
    assert_eq!(result[1].content, "Latest version");
    assert_eq!(result[1].status, "in_progress");
}

#[test]
fn empty_store_has_no_items() {
    let store = TodoStore::new();
    assert!(!store.has_items());
}

#[test]
fn non_empty_store_has_items() {
    let mut store = TodoStore::new();
    store.write(&[item("1", "x", "pending")], false);
    assert!(store.has_items());
}

#[test]
fn format_for_injection_empty_returns_none() {
    let store = TodoStore::new();
    assert!(store.format_for_injection().is_none());
}

#[test]
fn format_for_injection_has_markers() {
    let mut store = TodoStore::new();
    store.write(&[
        item("1", "Do thing", "completed"),
        item("2", "Next", "pending"),
        item("3", "Working", "in_progress"),
    ], false);
    let text = store.format_for_injection().unwrap();
    assert!(!text.contains("[x]"));
    assert!(!text.contains("Do thing"));
    assert!(text.contains("[ ]"));
    assert!(text.contains("[>]"));
    assert!(text.contains("Next"));
    assert!(text.contains("Working"));
    assert!(text.to_lowercase().contains("context compression"));
}

#[test]
fn merge_updates_existing_by_id() {
    let mut store = TodoStore::new();
    store.write(&[item("1", "Original", "pending")], false);
    store.write(&[json!({"id": "1", "status": "completed"})], true);
    let items = store.read();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].status, "completed");
    assert_eq!(items[0].content, "Original");
}

#[test]
fn merge_appends_new() {
    let mut store = TodoStore::new();
    store.write(&[item("1", "First", "pending")], false);
    store.write(&[item("2", "Second", "pending")], true);
    assert_eq!(store.read().len(), 2);
}

#[test]
fn tool_read_mode() {
    let mut store = TodoStore::new();
    store.write(&[item("1", "Task", "pending")], false);
    let result: Value = serde_json::from_str(&todo_tool(None, false, Some(&mut store))).unwrap();
    assert_eq!(result["summary"]["total"], 1);
    assert_eq!(result["summary"]["pending"], 1);
}

#[test]
fn tool_no_store_returns_error() {
    let result: Value = serde_json::from_str(&todo_tool(None, false, None)).unwrap();
    assert!(result.get("error").is_some());
}

#[test]
fn oversized_content_is_truncated() {
    let mut store = TodoStore::new();
    let long = "A".repeat(50_001);
    store.write(&[item("1", &long, "pending")], false);
    let item = store.read().remove(0);
    assert!(item.content.chars().count() <= MAX_TODO_CONTENT_CHARS);
    assert!(item.content.ends_with("… [truncated]"));
}

#[test]
fn injection_block_is_bounded() {
    let mut store = TodoStore::new();
    let long = "A".repeat(50_001);
    store.write(&[item("1", &long, "pending")], false);
    let inj = store.format_for_injection().unwrap();
    assert!(inj.chars().count() < MAX_TODO_CONTENT_CHARS + 200);
}

#[test]
fn item_count_is_bounded() {
    let mut store = TodoStore::new();
    let many: Vec<Value> = (0..5000).map(|i| item(&i.to_string(), &format!("task {i}"), "pending")).collect();
    store.write(&many, false);
    assert_eq!(store.read().len(), MAX_TODO_ITEMS);
}

#[test]
fn normal_list_is_unchanged() {
    let mut store = TodoStore::new();
    store.write(&[
        item("1", "write the report", "in_progress"),
        item("2", "review PR", "pending"),
    ], false);
    let items = store.read();
    assert_eq!(items[0].content, "write the report");
    assert_eq!(items[1].content, "review PR");
    assert!(!items[0].content.contains("[truncated]"));
}

// ---- test_todo_tool_type_coercion.py ----

#[test]
fn json_string_is_parsed_into_list() {
    let mut store = TodoStore::new();
    let todos_str = serde_json::to_string(&vec![
        item("t1", "Do A", "pending"),
        item("t2", "Do B", "in_progress"),
    ])
    .unwrap();
    let result: Value = serde_json::from_str(&todo_tool(Some(Value::String(todos_str)), false, Some(&mut store))).unwrap();
    assert!(result.get("error").is_none());
    assert_eq!(result["summary"]["total"], 2);
    assert_eq!(result["todos"][0]["id"], "t1");
    assert_eq!(result["todos"][1]["status"], "in_progress");
}

#[test]
fn non_list_non_string_returns_error() {
    let mut store = TodoStore::new();
    let result: Value = serde_json::from_str(&todo_tool(Some(json!(42)), false, Some(&mut store))).unwrap();
    assert!(result.get("error").is_some());
}

#[test]
fn string_item_in_list_does_not_crash() {
    let mut store = TodoStore::new();
    let result = store.write(&[json!("not-a-dict")], false);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "?");
    assert_eq!(result[0].content, "(invalid item)");
    assert_eq!(result[0].status, "pending");
}

#[test]
fn non_dict_items_via_tool() {
    let mut store = TodoStore::new();
    let result: Value = serde_json::from_str(&todo_tool(
        Some(json!(["bad", "also bad"])), false, Some(&mut store),
    ))
    .unwrap();
    assert!(result.get("error").is_none());
    assert_eq!(result["summary"]["total"], 2);
    assert_eq!(result["summary"]["pending"], 2);
}

#[test]
fn normal_write_and_read() {
    let mut store = TodoStore::new();
    let result: Value = serde_json::from_str(&todo_tool(
        Some(json!([
            {"id": "a", "content": "First", "status": "pending"},
            {"id": "b", "content": "Second", "status": "in_progress"},
        ])),
        false,
        Some(&mut store),
    ))
    .unwrap();
    assert_eq!(result["summary"]["total"], 2);
    assert_eq!(result["summary"]["pending"], 1);
    assert_eq!(result["summary"]["in_progress"], 1);
}

#[test]
fn dedup_still_works() {
    let mut store = TodoStore::new();
    let result = store.write(&[
        item("1", "v1", "pending"),
        item("1", "v2", "in_progress"),
    ], false);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].content, "v2");
}

#[test]
fn cap_content_exact() {
    assert_eq!(TodoStore::cap_content("short"), "short");
    let long = "x".repeat(5000);
    let capped = TodoStore::cap_content(&long);
    assert_eq!(capped.chars().count(), MAX_TODO_CONTENT_CHARS);
    assert!(capped.ends_with("… [truncated]"));
}

#[test]
fn check_requirements_true() {
    assert!(check_todo_requirements());
}

#[test]
fn schema_golden_parity() {
    let schema = hermes_tools::todo_tool::todo_schema();
    let golden: Value = serde_json::from_str(
        include_str!("../../../upstream/golden_todo_schema.json"),
    )
    .unwrap();
    assert_eq!(*schema, golden);
    assert_eq!(golden["name"], "todo");
}
