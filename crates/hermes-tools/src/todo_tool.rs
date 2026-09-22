//! Todo tool: in-memory, revisioned task list for multi-step work.
//!
//! PARITY: `tools/todo_tool.py` @ 5d59366 (whole module, 284 lines).
//! State lives on the AIAgent (one per session), is re-injected after
//! context compression, and every write bumps a monotonic revision so
//! UI clients can reject stale updates. One `todo_list` tool: pass
//! `todos` to write, omit to read; every call returns the full list.
//!
//! Divergence note: Python `str(value)` coercion of non-string todo
//! fields is approximated with the JSON text form (e.g. JSON `true` →
//! "true" vs Python "True"); upstream tests do not exercise those edges.

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
/// The list is re-read after every compression, so unbounded content
/// would defeat the compression it rides through. Caps apply equally
/// to model-authored items and caller-replayed API history.
pub const MAX_TODO_CONTENT_CHARS: usize = 4000;
pub const MAX_TODO_ITEMS: usize = 256;
/// Max single todo tool-result payload accepted during history
/// hydration, so a forged oversized result is dropped before parsing
/// (AIAgent._hydrate_todo_store).
pub const MAX_TODO_RESULT_CHARS: usize = 512_000;
const TRUNCATION_MARKER: &str = "… [truncated]";
/// Persisted as ordinary message content; the ContextCompressor keys on
/// this stable header to tell the synthetic post-compaction row from a
/// real user message.
pub const TODO_INJECTION_HEADER: &str =
    "[Your active task list was preserved across context compression]";

/// Status markers for injection rendering.
fn status_marker(status: &str) -> &'static str {
    match status {
        "completed" => "[x]",
        "in_progress" => "[>]",
        "pending" => "[ ]",
        "cancelled" => "[~]",
        _ => "[?]",
    }
}

/// One task item. List position is priority; `parent` nests a subtask.
///
/// PARITY: the `{id, content, status, parent?}` item shape (upstream
/// lines 26-28).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: String,
    pub parent: Option<String>,
}

impl TodoItem {
    fn from_parts(id: String, content: String, status: String, parent: Option<String>) -> Self {
        TodoItem {
            id,
            content,
            status,
            parent,
        }
    }

    fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("id".to_string(), Value::String(self.id.clone()));
        map.insert("content".to_string(), Value::String(self.content.clone()));
        map.insert("status".to_string(), Value::String(self.status.clone()));
        if let Some(parent) = &self.parent {
            map.insert("parent".to_string(), Value::String(parent.clone()));
        }
        Value::Object(map)
    }
}

/// In-memory todo list, one per AIAgent.
///
/// PARITY: `TodoStore` (upstream lines 26-188).
#[derive(Default)]
pub struct TodoStore {
    items: Vec<TodoItem>,
    revision: u64,
}

impl TodoStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate, dedupe and order a whole new list (replace / restore).
    fn fresh_items(&self, todos: &[Value]) -> Vec<TodoItem> {
        Self::normalize_order(
            &Self::dedupe_by_id(todos)
                .iter()
                .map(Self::validate)
                .collect::<Vec<_>>(),
        )
    }

    /// Replace the list (default) or merge by id; returns the full list
    /// after writing. Every state-changing write bumps the revision.
    ///
    /// PARITY: `write` (upstream lines 38-49).
    pub fn write(&mut self, todos: &[Value], merge: bool) -> Vec<TodoItem> {
        let before = self.read();
        if merge {
            self.merge(todos);
        } else {
            self.items = self.fresh_items(todos);
        }
        // Keep the priority head; replays can't grow unbounded.
        self.items.truncate(MAX_TODO_ITEMS);
        Self::sanitize_parents(&mut self.items);
        if self.items != before {
            self.revision += 1;
        }
        self.read()
    }

    /// Update existing items only in the fields provided; append new
    /// ones (validated).
    ///
    /// PARITY: `_merge` (upstream lines 51-76).
    fn merge(&mut self, todos: &[Value]) {
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
                if let Some(v) = t.get("parent") {
                    // Upstream `str(t["parent"] or "")`: null counts as
                    // absent (clears), never the literal "None".
                    let parent = match v {
                        Value::Null => String::new(),
                        _ => value_str(v).trim().to_string(),
                    };
                    if parent.is_empty() {
                        cur.parent = None;
                    } else {
                        cur.parent = Some(parent);
                    }
                }
            } else {
                // New item — validate fully and append to end.
                let validated = Self::validate(&t);
                existing.insert(validated.id.clone(), validated.clone());
                self.items.push(validated);
            }
        }
        // Rebuild preserving original order for existing items (first
        // occurrence wins).
        let mut seen = std::collections::HashSet::new();
        let mut rebuilt = Vec::with_capacity(self.items.len());
        for item in &self.items {
            let current = existing
                .get(&item.id)
                .cloned()
                .unwrap_or_else(|| item.clone());
            if seen.insert(current.id.clone()) {
                rebuilt.push(current);
            }
        }
        self.items = Self::normalize_order(&rebuilt);
    }

    /// Return a copy of the current list.
    pub fn read(&self) -> Vec<TodoItem> {
        self.items.clone()
    }

    pub fn has_items(&self) -> bool {
        !self.items.is_empty()
    }

    /// Full state clients can reconcile atomically.
    ///
    /// PARITY: `snapshot` (upstream lines 84-86).
    pub fn snapshot(&self) -> Value {
        json!({
            "todos": self.items.iter().map(TodoItem::to_json).collect::<Vec<_>>(),
            "revision": self.revision,
        })
    }

    /// Current monotonic revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Restore a trusted snapshot without manufacturing a new revision.
    ///
    /// PARITY: `restore` (upstream lines 88-95).
    pub fn restore(&mut self, todos: &[Value], revision: Option<&Value>) -> Vec<TodoItem> {
        let mut items = self.fresh_items(todos);
        items.truncate(MAX_TODO_ITEMS);
        self.items = items;
        self.revision = match revision {
            Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
            Some(Value::String(s)) => s.trim().parse::<u64>().unwrap_or(0),
            Some(Value::Bool(true)) => 1,
            _ => 0,
        };
        self.read()
    }

    /// Render the list for post-compression injection, or None if
    /// nothing active. Only pending/in_progress items are injected —
    /// finished ones make the model re-do work after compression. A
    /// parent is kept (with its real status marker) when any descendant
    /// is active so subtasks keep context.
    ///
    /// PARITY: `format_for_injection` (upstream lines 97-126).
    pub fn format_for_injection(&self) -> Option<String> {
        if self.items.is_empty() {
            return None;
        }
        let mut children: std::collections::HashMap<&str, Vec<&TodoItem>> =
            std::collections::HashMap::new();
        for item in &self.items {
            if let Some(parent) = &item.parent {
                children.entry(parent.as_str()).or_default().push(item);
            }
        }
        fn render(
            item: &TodoItem,
            depth: usize,
            out: &mut Vec<String>,
            children: &std::collections::HashMap<&str, Vec<&TodoItem>>,
        ) -> bool {
            let mut kid_lines = Vec::new();
            let mut has_active_kid = false;
            for kid in children.get(item.id.as_str()).into_iter().flatten() {
                has_active_kid |= render(kid, depth + 1, &mut kid_lines, children);
            }
            let keep = item.status == "pending" || item.status == "in_progress" || has_active_kid;
            if keep {
                out.push(format!(
                    "{}- {} {}. {} ({})",
                    "  ".repeat(depth),
                    status_marker(&item.status),
                    item.id,
                    item.content,
                    item.status
                ));
                out.extend(kid_lines);
            }
            keep
        }
        let mut lines = vec![TODO_INJECTION_HEADER.to_string()];
        for item in &self.items {
            if item.parent.is_none() {
                render(item, 0, &mut lines, &children);
            }
        }
        if lines.len() > 1 {
            Some(lines.join("\n"))
        } else {
            None
        }
    }

    /// Truncate to MAX_TODO_CONTENT_CHARS keeping the head (the
    /// actionable part) + marker.
    pub fn cap_content(content: &str) -> String {
        if content.chars().count() > MAX_TODO_CONTENT_CHARS {
            let keep = MAX_TODO_CONTENT_CHARS - TRUNCATION_MARKER.chars().count();
            let truncated: String = content.chars().take(keep).collect();
            return truncated + TRUNCATION_MARKER;
        }
        content.to_string()
    }

    /// Normalize one item to `{id, content, status, parent?}`
    /// (placeholders when missing).
    ///
    /// PARITY: `_validate` (upstream lines 135-149).
    fn validate(item: &Value) -> TodoItem {
        if !item.is_object() {
            return TodoItem::from_parts(
                "?".to_string(),
                "(invalid item)".to_string(),
                "pending".to_string(),
                None,
            );
        }
        let item_id = json_str(item, "id").trim().to_string();
        let item_id = if item_id.is_empty() {
            "?".to_string()
        } else {
            item_id
        };

        let mut content = json_str(item, "content").trim().to_string();
        if content.is_empty() {
            content = "(no description)".to_string();
        } else {
            content = Self::cap_content(&content);
        }

        let status = json_str(item, "status").trim().to_lowercase();
        // Upstream defaults a missing status to pending: `str(item.get(
        // "status", "pending"))` — json_str already yields "" when
        // missing, which falls through to pending below.
        let status = if VALID_STATUSES.contains(&status.as_str()) {
            status
        } else {
            "pending".to_string()
        };

        // Upstream `str(item.get("parent") or "")`: any falsy parent
        // (None, "", False, 0, [], {}) drops; self-parent drops.
        let parent_raw = item.get("parent");
        let parent = match parent_raw {
            Some(v) if json_truthy(v) => value_str(v).trim().to_string(),
            _ => String::new(),
        };
        let parent = if parent.is_empty() || parent == item_id {
            None
        } else {
            Some(parent)
        };

        TodoItem::from_parts(item_id, content, status, parent)
    }

    /// Drop dangling parent refs and break cycles in place (such items
    /// become roots).
    ///
    /// PARITY: `_sanitize_parents` (upstream lines 151-165).
    fn sanitize_parents(items: &mut [TodoItem]) {
        let ids: std::collections::HashSet<String> = items.iter().map(|i| i.id.clone()).collect();
        for item in items.iter_mut() {
            if let Some(parent) = &item.parent {
                if !ids.contains(parent) {
                    item.parent = None;
                }
            }
        }
        for i in 0..items.len() {
            let mut seen = std::collections::HashSet::new();
            seen.insert(items[i].id.clone());
            let mut node_parent = items[i].parent.clone();
            // Walk up; a missing parent can't happen post-sanitize, but
            // guard anyway (upstream indexes by_id directly).
            while let Some(parent) = node_parent {
                if !seen.insert(parent.clone()) {
                    items[i].parent = None;
                    break;
                }
                node_parent = items
                    .iter()
                    .find(|it| it.id == parent)
                    .and_then(|it| it.parent.clone());
            }
        }
    }

    /// Collapse duplicate ids, keeping the last occurrence in its position.
    ///
    /// PARITY: `_dedupe_by_id` (upstream lines 167-174). Non-dicts get a
    /// synthetic key; `_validate` handles them downstream.
    fn dedupe_by_id(todos: &[Value]) -> Vec<Value> {
        let mut last_index: Vec<(String, usize)> = Vec::new();
        for (i, item) in todos.iter().enumerate() {
            if !item.is_object() {
                last_index.push((format!("__invalid_{i}"), i));
                continue;
            }
            let item_id = json_str(item, "id").trim().to_string();
            let key = if item_id.is_empty() {
                "?".to_string()
            } else {
                item_id
            };
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

    /// Lift the in_progress step ahead of any earlier pending
    /// placeholder. Nested lists keep authored order — reordering would
    /// tear a subtask from its siblings.
    ///
    /// PARITY: `_normalize_order` (upstream lines 176-188).
    fn normalize_order(items: &[TodoItem]) -> Vec<TodoItem> {
        let has_parent = items.iter().any(|i| i.parent.is_some());
        let active_index = items.iter().position(|i| i.status == "in_progress");
        let Some(active_index) = active_index else {
            return items.to_vec();
        };
        if has_parent {
            return items.to_vec();
        }
        let pending_before = items[..active_index]
            .iter()
            .position(|i| i.status == "pending");
        let Some(pending_pos) = pending_before else {
            return items.to_vec();
        };
        let mut normalized = items.to_vec();
        let active = normalized.remove(active_index);
        normalized.insert(pending_pos, active);
        normalized
    }
}

/// Single entry point for the todo tool: reads or writes depending on params.
///
/// PARITY: `todo_tool` (upstream lines 191-211) — write returns the
/// full list + revision + summary; read returns the same shape.
pub fn todo_tool(todos: Option<Value>, merge: bool, store: Option<&mut TodoStore>) -> String {
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
            return todos_json(store, &items);
        }
        if !todos.is_array() {
            return tool_error(
                format!("todos must be a list, got {}", py_type_name(&todos)),
                &[],
            );
        }
        let items = store.write(todos.as_array().unwrap(), merge);
        return todos_json(store, &items);
    }
    let items = store.read();
    todos_json(store, &items)
}

fn todos_json(store: &TodoStore, items: &[TodoItem]) -> String {
    let pending = items.iter().filter(|i| i.status == "pending").count();
    let in_progress = items.iter().filter(|i| i.status == "in_progress").count();
    let completed = items.iter().filter(|i| i.status == "completed").count();
    let cancelled = items.iter().filter(|i| i.status == "cancelled").count();
    serde_json::to_string(&json!({
        "todos": items.iter().map(TodoItem::to_json).collect::<Vec<_>>(),
        "revision": store.revision(),
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
/// `tools/todo_tool.py` TODO_SCHEMA @ 5d59366 (golden:
/// upstream/golden_todo_schema.json).
pub fn todo_schema() -> &'static Value {
    static SCHEMA: Lazy<Value> = Lazy::new(|| {
        serde_json::from_str(include_str!("../../../upstream/golden_todo_schema.json"))
            .expect("todo schema")
    });
    &SCHEMA
}

/// Register the `todo_list` tool (mirrors upstream module-level
/// registry.register; the agent loop calls this when the todo toolset
/// is enabled).
pub fn register_todo() {
    registry()
        .register(
            "todo_list",
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
