//! Todo tool — planning & task management for the agent loop.
//!
//! PARITY: tools/todo_tool.py @ b9aa928 (335 LOC, ported 1:1). The state
//! lives on the AIAgent instance (one per session) and is re-injected into
//! the conversation after context-compression events. Bounds on persisted
//! state (GHSA-5g4g-6jrg-mw3g hardening) are part of the contract.
//!
//! Divergence note: Python `str(value)` coercion of non-string todo fields is
//! approximated with the JSON text form (e.g. JSON `true` → "true" vs Python
//! "True"); upstream tests do not exercise those edges.

use once_cell::sync::Lazy;
use serde_json::{json, Value};

use crate::registry::{registry, tool_error, CheckFn, ToolHandler, ToolResult};
use std::cell::RefCell;
use std::sync::Arc;

// Thread-local TodoStore for the registered tool handler. The agent runner
// (AIAgent in Python) owns one store per session and passes it through the
// `store=` kwarg; this seam mirrors that injection. When unset the handler
// reports the upstream "TodoStore not initialized" error exactly like a
// dispatch without the kwarg.
thread_local! {
    static TODO_STORE: RefCell<Option<TodoStore>> = const { RefCell::new(None) };
}

/// Install the session todo store for this thread.
pub fn set_todo_store(store: Option<TodoStore>) {
    TODO_STORE.with(|slot| *slot.borrow_mut() = store);
}

pub const VALID_STATUSES: [&str; 4] = ["pending", "in_progress", "completed", "cancelled"];
pub const MAX_TODO_CONTENT_CHARS: usize = 4000;
pub const MAX_TODO_ITEMS: usize = 256;
pub const MAX_TODO_RESULT_CHARS: usize = 512_000;
const TRUNCATION_MARKER: &str = "… [truncated]";
pub const TODO_INJECTION_HEADER: &str =
    "[Your active task list was preserved across context compression]";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: String,
}

impl TodoItem {
    fn from_parts(id: String, content: String, status: String) -> Self {
        TodoItem { id, content, status }
    }
}

/// In-memory todo list. One instance per session (AIAgent).
#[derive(Default)]
pub struct TodoStore {
    items: Vec<TodoItem>,
}

impl TodoStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Write todos. `merge=false` replaces the list; `merge=true` updates
    /// existing items by id and appends new ones. Returns the full list.
    pub fn write(&mut self, todos: &[Value], merge: bool) -> Vec<TodoItem> {
        if !merge {
            self.items = Self::dedupe_by_id(todos)
                .iter()
                .map(Self::validate)
                .collect();
        } else {
            let mut existing: std::collections::HashMap<String, TodoItem> = self
                .items
                .iter()
                .map(|i| (i.id.clone(), i.clone()))
                .collect();
            for t in Self::dedupe_by_id(todos) {
                let item_id = json_str(&t, "id").trim().to_string();
                if item_id.is_empty() {
                    continue; // Can't merge without an id
                }
                if let Some(cur) = existing.get_mut(&item_id) {
                    // Update only the fields the LLM actually provided.
                    if let Some(v) = t.get("content") {
                        if json_truthy(v) {
                            cur.content = Self::cap_content(value_str(v).trim());
                        }
                    }
                    if let Some(v) = t.get("status") {
                        if json_truthy(v) {
                            let status = value_str(v).trim().to_lowercase();
                            if VALID_STATUSES.contains(&status.as_str()) {
                                cur.status = status;
                            }
                        }
                    }
                } else {
                    // New item — validate fully and append to end.
                    let validated = Self::validate(&t);
                    existing.insert(validated.id.clone(), validated.clone());
                    self.items.push(validated);
                }
            }
            // Rebuild _items preserving order for existing items.
            let mut seen = std::collections::HashSet::new();
            let mut rebuilt = Vec::with_capacity(self.items.len());
            for item in &self.items {
                let current = existing.get(&item.id).cloned().unwrap_or_else(|| item.clone());
                if seen.insert(current.id.clone()) {
                    rebuilt.push(current);
                }
            }
            self.items = rebuilt;
        }
        // Bound total item count; keep the highest-priority head.
        if self.items.len() > MAX_TODO_ITEMS {
            self.items.truncate(MAX_TODO_ITEMS);
        }
        self.read()
    }

    /// Return a copy of the current list.
    pub fn read(&self) -> Vec<TodoItem> {
        self.items.clone()
    }

    pub fn has_items(&self) -> bool {
        !self.items.is_empty()
    }

    /// Render the todo list for post-compression injection; `None` when there
    /// is nothing active to say.
    pub fn format_for_injection(&self) -> Option<String> {
        if self.items.is_empty() {
            return None;
        }
        let active_items: Vec<&TodoItem> = self
            .items
            .iter()
            .filter(|i| i.status == "pending" || i.status == "in_progress")
            .collect();
        if active_items.is_empty() {
            return None;
        }
        let mut lines = vec![TODO_INJECTION_HEADER.to_string()];
        for item in active_items {
            let marker = match item.status.as_str() {
                "completed" => "[x]",
                "in_progress" => "[>]",
                "pending" => "[ ]",
                "cancelled" => "[~]",
                _ => "[?]",
            };
            lines.push(format!(
                "- {marker} {}. {} ({})",
                item.id, item.content, item.status
            ));
        }
        Some(lines.join("\n"))
    }

    /// Truncate oversized todo content, keeping the head plus a marker.
    pub fn cap_content(content: &str) -> String {
        if content.chars().count() > MAX_TODO_CONTENT_CHARS {
            let keep = MAX_TODO_CONTENT_CHARS - TRUNCATION_MARKER.chars().count();
            let truncated: String = content.chars().take(keep).collect();
            return truncated + TRUNCATION_MARKER;
        }
        content.to_string()
    }

    /// Validate and normalize a todo item.
    fn validate(item: &Value) -> TodoItem {
        if !item.is_object() {
            return TodoItem::from_parts(
                "?".to_string(),
                "(invalid item)".to_string(),
                "pending".to_string(),
            );
        }
        let item_id = json_str(item, "id").trim().to_string();
        let item_id = if item_id.is_empty() { "?".to_string() } else { item_id };

        let mut content = json_str(item, "content").trim().to_string();
        if content.is_empty() {
            content = "(no description)".to_string();
        } else {
            content = Self::cap_content(&content);
        }

        let status = json_str(item, "status").trim().to_lowercase();
        let status = if VALID_STATUSES.contains(&status.as_str()) {
            status
        } else {
            "pending".to_string()
        };

        TodoItem::from_parts(item_id, content, status)
    }

    /// Collapse duplicate ids, keeping the last occurrence in its position.
    fn dedupe_by_id(todos: &[Value]) -> Vec<Value> {
        let mut last_index: Vec<(String, usize)> = Vec::new();
        for (i, item) in todos.iter().enumerate() {
            if !item.is_object() {
                last_index.push((format!("__invalid_{i}"), i));
                continue;
            }
            let item_id = json_str(item, "id").trim().to_string();
            let key = if item_id.is_empty() { "?".to_string() } else { item_id };
            last_index.push((key, i));
        }
        // Keep the last index per key; then restore order.
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut final_last: Vec<(String, usize)> = Vec::new();
        for (k, idx) in last_index.iter().rev() {
            if seen.insert(k.as_str()) {
                final_last.push((k.clone(), *idx));
            }
        }
        final_last.reverse();
        // Sort by original index (keeps the last occurrence in its position).
        final_last.sort_by_key(|(_, i)| *i);
        final_last.iter().map(|(_, i)| todos[*i].clone()).collect()
    }
}

/// Single entry point for the todo tool: reads or writes depending on params.
pub fn todo_tool(
    todos: Option<Value>,
    merge: bool,
    store: Option<&mut TodoStore>,
) -> String {
    let Some(store) = store else {
        return tool_error("TodoStore not initialized", &[]);
    };

    if let Some(todos) = todos {
        // Guard: LLM sometimes sends todos as a JSON string instead of a list.
        if let Value::String(s) = &todos {
            let parsed: Result<Value, _> = serde_json::from_str(s);
            let todos = match parsed {
                Ok(v) => v,
                Err(_) => {
                    return tool_error(
                        "todos must be a list of objects, got unparseable string",
                        &[],
                    );
                }
            };
            if !todos.is_array() {
                return tool_error(
                    format!("todos must be a list, got {}", py_type_name(&todos)),
                    &[],
                );
            }
            let items = store.write(todos.as_array().unwrap(), merge);
            return todos_json(&items);
        }
        if !todos.is_array() {
            return tool_error(
                format!("todos must be a list, got {}", py_type_name(&todos)),
                &[],
            );
        }
        let items = store.write(todos.as_array().unwrap(), merge);
        return todos_json(&items);
    }
    let items = store.read();
    todos_json(&items)
}

fn todos_json(items: &[TodoItem]) -> String {
    let pending = items.iter().filter(|i| i.status == "pending").count();
    let in_progress = items.iter().filter(|i| i.status == "in_progress").count();
    let completed = items.iter().filter(|i| i.status == "completed").count();
    let cancelled = items.iter().filter(|i| i.status == "cancelled").count();
    serde_json::to_string(&json!({
        "todos": items.iter().map(|i| json!({"id": i.id, "content": i.content, "status": i.status})).collect::<Vec<_>>(),
        "summary": {
            "total": items.len(),
            "pending": pending,
            "in_progress": in_progress,
            "completed": completed,
            "cancelled": cancelled,
        },
    }))
    .unwrap_or_default()
}

/// Todo tool has no external requirements -- always available.
pub fn check_todo_requirements() -> bool {
    true
}

/// Read a string field from a JSON object; missing/null produce "".
fn json_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(other) if other.is_null() => String::new(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Python `str(value)` for an arbitrary JSON value (upstream `str(t["x"])`).
fn value_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "None".to_string(),
        other => other.to_string(),
    }
}

/// Python-style truthiness for JSON values (upstream `if t["content"]`).
fn json_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Approximate Python `type(x).__name__` for the error message.
fn py_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "NoneType",
        Value::Bool(_) => "bool",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "int"
            } else {
                "float"
            }
        }
        Value::String(_) => "str",
        Value::Array(_) => "list",
        Value::Object(_) => "dict",
    }
}

/// The upstream TODO_SCHEMA, extracted verbatim from
/// `tools/todo_tool.py` TODO_SCHEMA @ b9aa928 (golden: upstream/golden_todo_schema.json).
pub fn todo_schema() -> &'static Value {
    static SCHEMA: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../upstream/golden_todo_schema.json"))
            .expect("todo schema")
    });
    &SCHEMA
}

/// Register the `todo` tool (mirrors upstream module-level registry.register;
/// the agent loop calls this when the todo toolset is enabled).
pub fn register_todo() {
    registry()
        .register(
            "todo",
            "todo",
            todo_schema().clone(),
            Arc::new(TodoHandler),
            Some(Arc::new(TodoCheck)),
            Some("check_todo_requirements"),
            vec![],
            None,
            Some("📋".to_string()),
            None,
            None,
            None,
            false,
        )
        .expect("register todo");
}

struct TodoHandler;
impl ToolHandler for TodoHandler {
    fn call(&self, args: Value, _task_id: Option<&str>, _user_task: Option<&str>) -> ToolResult {
        let merge = args.get("merge").and_then(Value::as_bool).unwrap_or(false);
        let todos = args.get("todos").cloned();
        // Dispatch with the thread-local store, mirroring the upstream
        // `store=kw.get("store")` injection from the agent loop.
        let result = TODO_STORE.with(|slot| {
            let mut borrow = slot.borrow_mut();
            todo_tool(todos, merge, borrow.as_mut())
        });
        ToolResult::Text(result)
    }
}

struct TodoCheck;
impl CheckFn for TodoCheck {
    fn check(&self) -> bool {
        check_todo_requirements()
    }
}
