//! Read the in-app terminal pane in the Hermes desktop GUI.
//!
//! PARITY: tools/read_terminal_tool.py @ b9aa928 (89 LOC, ported 1:1).
//!
//! The embedded terminal's buffer lives in the desktop renderer (xterm.js),
//! so this tool round-trips through the gateway's blocking-prompt bridge.
//! This module is just schema + a thin dispatcher over the
//! platform-injected callback.
//!
//! PORT SEAMS:
//! - The Python signature coerces `start_line`/`count` with `int()` at call
//!   time; the Rust public function keeps `Option<serde_json::Value>`
//!   parameters so the same error paths are reachable from dispatch.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::Lazy;
use serde_json::{json, Value};

use crate::registry::{registry, ToolHandler, ToolResult};

/// Platform-injected renderer read callback: `callback(**window)` where
/// window is `{"start_line": ..., "count": ...}` (only present keys).
pub type ReadTerminalCallback = dyn Fn(&HashMap<String, i64>) -> String + Send + Sync;

thread_local! {
    static READ_TERMINAL_CALLBACK: RefCell<Option<Arc<ReadTerminalCallback>>> =
        const { RefCell::new(None) };
}

/// Set the platform read callback for this thread (the desktop gateway
/// injects this before dispatch, matching `kwargs["callback"]`).
pub fn set_read_terminal_callback<F>(cb: F)
where
    F: Fn(&HashMap<String, i64>) -> String + Send + Sync + 'static,
{
    READ_TERMINAL_CALLBACK.with(|slot| *slot.borrow_mut() = Some(Arc::new(cb)));
}

pub fn clear_read_terminal_callback() {
    READ_TERMINAL_CALLBACK.with(|slot| *slot.borrow_mut() = None);
}

/// Python `int()`-style coercion for window parameters (see
/// read_preview_tool for details).
fn coerce_int(value: &Value) -> Result<i64, ()> {
    match value {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Ok(i);
            }
            if let Some(f) = n.as_f64() {
                if f.is_finite() && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
                    return Ok(f.trunc() as i64);
                }
            }
            Err(())
        }
        Value::String(s) => python_int(s).ok_or(()),
        _ => Err(()),
    }
}

fn python_int(s: &str) -> Option<i64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let bytes: Vec<char> = t.chars().collect();
    let (sign, rest) = match bytes[0] {
        '+' => (1i64, &bytes[1..]),
        '-' => (-1i64, &bytes[1..]),
        _ => (1i64, &bytes[..]),
    };
    if rest.is_empty() {
        return None;
    }
    let mut digits = String::with_capacity(rest.len());
    let n = rest.len();
    for (i, ch) in rest.iter().enumerate() {
        if *ch == '_' {
            if i == 0
                || i == n - 1
                || !rest[i - 1].is_ascii_digit()
                || !rest[i + 1].is_ascii_digit()
            {
                return None;
            }
        } else if !ch.is_ascii_digit() {
            return None;
        } else {
            digits.push(*ch);
        }
    }
    digits.parse::<i64>().ok().map(|d| d * sign)
}

fn build_window(
    start_line: Option<&Value>,
    count: Option<&Value>,
) -> Result<HashMap<String, i64>, ()> {
    let mut window = HashMap::new();
    if let Some(start_line) = start_line {
        window.insert(
            "start_line".to_string(),
            (0i64).max(coerce_int(start_line)?),
        );
    }
    if let Some(count) = count {
        window.insert("count".to_string(), (1i64).max(coerce_int(count)?));
    }
    Ok(window)
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown error".to_string()
    }
}

/// Return the in-app terminal's contents (+ line metadata) as a JSON
/// string.
///
/// `callback` mirrors the upstream `callback=` kwarg; when `None` the tool
/// reports it is desktop-only.
///
/// PARITY: tools/read_terminal_tool.py `read_terminal_tool` @ b9aa928.
pub fn read_terminal_tool(
    start_line: Option<Value>,
    count: Option<Value>,
    callback: Option<Arc<ReadTerminalCallback>>,
) -> String {
    let Some(callback) = callback else {
        return crate::registry::tool_error(
            "read_terminal is only available in the Hermes desktop app.",
            &[],
        );
    };

    let window = match build_window(start_line.as_ref(), count.as_ref()) {
        Ok(window) => window,
        Err(()) => {
            return crate::registry::tool_error("start_line and count must be integers.", &[]);
        }
    };

    let raw = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(&window))) {
        Ok(raw) => raw,
        Err(panic) => {
            return crate::registry::tool_error(
                format!("Failed to read terminal: {}", panic_message(panic)),
                &[],
            );
        }
    };

    if raw.is_empty() {
        return crate::registry::tool_error(
            "No in-app terminal is open, or the read timed out.",
            &[],
        );
    }

    // Desktop answers with a JSON object; pass it through, else wrap the
    // raw text.
    if let Ok(parsed) = serde_json::from_str::<Value>(&raw) {
        return serde_json::to_string(&parsed).expect("json");
    }
    serde_json::to_string(&json!({"text": raw})).expect("json")
}

pub static READ_TERMINAL_SCHEMA: Lazy<Value> = Lazy::new(|| {
    json!({
        "name": "read_terminal",
        "description": "Read what's currently shown in the in-app terminal pane of the Hermes desktop GUI (the embedded shell beside this chat). Call with no arguments to get the visible screen plus the total line count (`total_lines`). To page through scrollback, pass `start_line` (0 = oldest line) and `count`; valid lines are [0, total_lines). Returns JSON: {total_lines, start, end, viewport_rows, cursor_row, text}.",
        "parameters": {
            "type": "object",
            "properties": {
                "start_line": {
                    "type": "integer",
                    "description": "0-indexed first line (0 = oldest). Omit for the visible screen."
                },
                "count": {
                    "type": "integer",
                    "description": "Lines to read from start_line. Defaults to the visible row count."
                }
            }
        }
    })
});

struct ReadTerminalHandler;
impl ToolHandler for ReadTerminalHandler {
    fn call(&self, args: Value, _: Option<&str>, _: Option<&str>) -> ToolResult {
        let callback = READ_TERMINAL_CALLBACK.with(|slot| slot.borrow().clone());
        ToolResult::Text(read_terminal_tool(
            args.get("start_line").cloned(),
            args.get("count").cloned(),
            callback,
        ))
    }
}

/// Register the `read_terminal` tool into the registry singleton
/// (`desktop_ui` toolset, no check_fn — the GUI gateway gates by toolset).
///
/// PARITY: tools/read_terminal_tool.py module-level `registry.register`
/// @ b9aa928.
pub fn register_read_terminal() {
    registry()
        .register(
            "read_terminal",
            "desktop_ui",
            READ_TERMINAL_SCHEMA.clone(),
            Arc::new(ReadTerminalHandler),
            None,
            None,
            vec![],
            None,
            Some("🖥️".to_string()),
            None,
            None,
            None,
            false,
        )
        .expect("register read_terminal");
}
