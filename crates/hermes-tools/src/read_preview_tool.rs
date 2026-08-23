//! Read the in-app browser / preview pane in the Hermes desktop GUI.
//!
//! PARITY: tools/read_preview_tool.py @ b9aa928 (94 LOC, ported 1:1).
//!
//! The preview's content lives in the desktop renderer (a sandboxed
//! `<webview>` for URL tabs), so this tool round-trips through the
//! gateway's blocking-prompt bridge.  This module is just schema + a thin
//! dispatcher over the platform-injected callback.
//!
//! The callback is injected per-thread (mirroring the Python agent runner
//! passing `callback=` in dispatch kwargs), the same seam clarify.rs uses.
//! Without a wired callback the tool reports "desktop app only".
//!
//! PORT SEAMS:
//! - The Python signature coerces `start`/`count` with `int()` at call
//!   time; the Rust public function keeps `Option<serde_json::Value>`
//!   parameters so the same error paths are reachable from dispatch.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::Lazy;
use serde_json::{json, Value};

use crate::registry::{registry, ToolHandler, ToolResult};

/// Platform-injected renderer read callback: `callback(**window)` where
/// window is `{"start": ..., "count": ...}` (only present keys).
pub type ReadPreviewCallback = dyn Fn(&HashMap<String, i64>) -> String + Send + Sync;

thread_local! {
    static READ_PREVIEW_CALLBACK: RefCell<Option<Arc<ReadPreviewCallback>>> =
        const { RefCell::new(None) };
}

/// Set the platform read callback for this thread (the desktop gateway
/// injects this before dispatch, matching `kwargs["callback"]`).
pub fn set_read_preview_callback<F>(cb: F)
where
    F: Fn(&HashMap<String, i64>) -> String + Send + Sync + 'static,
{
    READ_PREVIEW_CALLBACK.with(|slot| *slot.borrow_mut() = Some(Arc::new(cb)));
}

pub fn clear_read_preview_callback() {
    READ_PREVIEW_CALLBACK.with(|slot| *slot.borrow_mut() = None);
}

/// Python `int()`-style coercion for window parameters: numbers pass
/// through (floats truncate toward zero), numeric strings parse (sign,
/// underscores between digits accepted, like CPython), anything else — or
/// an out-of-range value — is an error.
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
            // Python allows underscores only between digits.
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

/// Build the callback window dict with the upstream floor semantics
/// (`start >= 0`, `count >= 1`).
fn build_window(start: Option<&Value>, count: Option<&Value>) -> Result<HashMap<String, i64>, ()> {
    let mut window = HashMap::new();
    if let Some(start) = start {
        window.insert("start".to_string(), (0i64).max(coerce_int(start)?));
    }
    if let Some(count) = count {
        window.insert("count".to_string(), (1i64).max(coerce_int(count)?));
    }
    Ok(window)
}

/// Best-effort panic message extraction (`str(exc)`-equivalent for
/// callback failures).
fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown error".to_string()
    }
}

/// Return the active preview tab's contents (+ metadata) as a JSON string.
///
/// `callback` mirrors the upstream `callback=` kwarg; when `None` the tool
/// reports it is desktop-only.
///
/// PARITY: tools/read_preview_tool.py `read_preview_tool` @ b9aa928.
pub fn read_preview_tool(
    start: Option<Value>,
    count: Option<Value>,
    callback: Option<Arc<ReadPreviewCallback>>,
) -> String {
    let Some(callback) = callback else {
        return crate::registry::tool_error(
            "read_preview is only available in the Hermes desktop app.",
            &[],
        );
    };

    let window = match build_window(start.as_ref(), count.as_ref()) {
        Ok(window) => window,
        Err(()) => {
            return crate::registry::tool_error("start and count must be integers.", &[]);
        }
    };

    let raw = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(&window))) {
        Ok(raw) => raw,
        Err(panic) => {
            return crate::registry::tool_error(
                format!("Failed to read the preview pane: {}", panic_message(panic)),
                &[],
            );
        }
    };

    if raw.is_empty() {
        return crate::registry::tool_error("No preview tab is open, or the read timed out.", &[]);
    }

    // Desktop answers with a JSON object; pass it through, else wrap the
    // raw text.
    if let Ok(parsed) = serde_json::from_str::<Value>(&raw) {
        return serde_json::to_string(&parsed).expect("json");
    }
    serde_json::to_string(&json!({"text": raw})).expect("json")
}

pub static READ_PREVIEW_SCHEMA: Lazy<Value> = Lazy::new(|| {
    json!({
        "name": "read_preview",
        "description": "Read what's currently shown in the in-app browser / preview pane of the Hermes desktop GUI (the pane open_preview opens beside this chat). Call with no arguments for the first window of the active tab's content. Returns JSON {kind, url, title, text, start, end, total_chars, note?}: a URL (Browser) tab's text is the rendered page's visible text — page through longer pages with `start`/`count` (character offsets, capped per read); a file tab answers identity only (read the file with read_file); an artifact tab points back at the conversation. Use after open_preview, or whenever the user refers to what's on screen in the browser ('what does this page say?').",
        "parameters": {
            "type": "object",
            "properties": {
                "start": {
                    "type": "integer",
                    "description": "0-indexed character offset into the page text. Omit for the start."
                },
                "count": {
                    "type": "integer",
                    "description": "Characters to return from start. Defaults to (and is capped at) the per-read maximum."
                }
            }
        }
    })
});

struct ReadPreviewHandler;
impl ToolHandler for ReadPreviewHandler {
    fn call(&self, args: Value, _: Option<&str>, _: Option<&str>) -> ToolResult {
        let callback = READ_PREVIEW_CALLBACK.with(|slot| slot.borrow().clone());
        ToolResult::Text(read_preview_tool(
            args.get("start").cloned(),
            args.get("count").cloned(),
            callback,
        ))
    }
}

/// Register the `read_preview` tool into the registry singleton
/// (`desktop_ui` toolset, no check_fn — the GUI gateway gates by toolset).
///
/// PARITY: tools/read_preview_tool.py module-level `registry.register`
/// @ b9aa928.
pub fn register_read_preview() {
    registry()
        .register(
            "read_preview",
            "desktop_ui",
            READ_PREVIEW_SCHEMA.clone(),
            Arc::new(ReadPreviewHandler),
            None,
            None,
            vec![],
            None,
            Some("🔍".to_string()),
            None,
            None,
            None,
            false,
        )
        .expect("register read_preview");
}
