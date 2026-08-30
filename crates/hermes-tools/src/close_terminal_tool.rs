//! Close a read-only agent terminal tab in the Hermes desktop GUI.
//!
//! PARITY: `tools/close_terminal_tool.py` @ b9aa928 (whole module) plus the
//! `process_registry.request_close_terminal` contract it dispatches to
//! (`tools/process_registry.py` lines 2143-2173) — the registry mega-module
//! itself stays PENDING, so the `on_close` sink is injected as a seam
//! (the pattern used by `read_terminal_tool` / `desktop_ui`).
//!
//! Each `terminal(background=true)` process is mirrored as a read-only tab
//! in the desktop's terminal pane. This tool lets the agent drop a tab it
//! no longer needs to show — WITHOUT killing the process (use
//! `process(action='kill')` for that). The output keeps buffering and the
//! user can reopen the tab from the status stack.
//!
//! It routes through the process registry's `on_close` sink, which the
//! desktop gateway wires to emit a `terminal.close` event the renderer
//! handles. Like `read_terminal` it lives in the `desktop_ui` toolset,
//! which the GUI gateway enables only for desktop-sourced sessions, so it
//! never appears outside the GUI.

use std::sync::{Arc, Mutex, OnceLock};

use once_cell::sync::Lazy;
use serde_json::{json, Value};

use crate::registry::{registry, tool_error, ToolHandler, ToolResult};

/// The desktop gateway's `on_close` sink: receives the session id of the
/// background process whose tab to drop. Upstream's sink also receives the
/// live `ProcessSession` object; that parameter is dropped until
/// `tools.process_registry` ports (the desktop handler keys off the id).
pub type CloseSink = dyn Fn(&str) -> Result<(), String> + Send + Sync;

static CLOSE_SINK: OnceLock<Mutex<Option<Arc<CloseSink>>>> = OnceLock::new();

fn close_sink_slot() -> &'static Mutex<Option<Arc<CloseSink>>> {
    CLOSE_SINK.get_or_init(|| Mutex::new(None))
}

/// Wire (or clear) the desktop `terminal.close` sink — the
/// `process_registry.on_close` attribute.
pub fn set_close_terminal_sink(sink: Option<Arc<CloseSink>>) {
    *close_sink_slot().lock().unwrap_or_else(|e| e.into_inner()) = sink;
}

/// Ask the registry to close a background process's read-only tab.
///
/// PARITY: `ProcessRegistry.request_close_terminal` (upstream lines
/// 2143-2173): no wired sink → the desktop-only error dict; the sink call
/// is best-effort (`Exception` → `{"status": "error", "error": str(e)}`);
/// success returns `{"status": "ok", "closed": <id>, "note": …}`. A
/// missing/finished session is NOT an error — the tab can still linger and
/// be closed, so the sink is invoked regardless.
pub fn request_close_terminal(session_id: &str) -> Value {
    let sink = close_sink_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let Some(sink) = sink else {
        return json!({
            "status": "error",
            "error": "close_terminal is only available in the Hermes desktop app."
        });
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sink(session_id))) {
        Ok(Ok(())) => json!({
            "status": "ok",
            "closed": session_id,
            "note": (
                "Closed the read-only terminal tab. The process was not killed; \
                 its output remains available and the user can reopen the tab \
                 from the status stack."
            )
        }),
        Ok(Err(message)) => json!({"status": "error", "error": message}),
        // The sink raised (except arm) — surface the panic payload text.
        Err(panic) => {
            let detail = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "close sink panicked".to_string());
            json!({"status": "error", "error": detail})
        }
    }
}

/// Ask the desktop GUI to close a background process's read-only tab.
///
/// PARITY: `close_terminal_tool` (upstream lines 21-27).
pub fn close_terminal_tool(process_id: &str) -> String {
    let pid = process_id.trim();
    if pid.is_empty() {
        return tool_error(
            "process_id is required (the background process whose tab to close).",
            &[],
        );
    }
    serde_json::to_string(&request_close_terminal(pid)).unwrap_or_default()
}

/// PARITY: `CLOSE_TERMINAL_SCHEMA` (upstream lines 30-49).
pub static CLOSE_TERMINAL_SCHEMA: Lazy<Value> = Lazy::new(|| {
    json!({
        "name": "close_terminal",
        "description": (
            "Close the read-only terminal tab for one of your background processes in \
             the Hermes desktop GUI (the tabs mirroring terminal(background=true) runs). \
             This does NOT kill the process — it only drops the tab/view; the output \
             keeps buffering and the user can reopen it from the status stack. Use it \
             to tidy up when a background process's live terminal is no longer worth \
             showing. To actually stop the process, use process(action='kill') instead."
        ),
        "parameters": {
            "type": "object",
            "properties": {
                "process_id": {
                    "type": "string",
                    "description": (
                        "The background process's session id (from terminal(background=true) \
                         output or process(action='list')) whose tab should be closed."
                    ),
                },
            },
            "required": ["process_id"],
        },
    })
});

struct CloseTerminalHandler;

impl ToolHandler for CloseTerminalHandler {
    fn call(&self, args: Value, _: Option<&str>, _: Option<&str>) -> ToolResult {
        // Upstream: `process_id=args.get("process_id", "")`.
        let process_id = args.get("process_id").and_then(Value::as_str).unwrap_or("");
        ToolResult::Text(close_terminal_tool(process_id))
    }
}

/// Register the `close_terminal` tool into the registry singleton
/// (`desktop_ui` toolset, emoji 🖥️).
///
/// PARITY: `registry.register(name="close_terminal", toolset="desktop_ui",
/// ...)` (upstream lines 52-59).
pub fn register_close_terminal() {
    registry()
        .register(
            "close_terminal",
            "desktop_ui",
            CLOSE_TERMINAL_SCHEMA.clone(),
            Arc::new(CloseTerminalHandler),
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
        .expect("register close_terminal");
}
