//! Reveal/focus a pane in the Hermes desktop GUI.
//!
//! PARITY: `tools/focus_pane_tool.py` @ b9aa928 (whole module, lines 1-65).
//!
//! Lives in the `desktop_ui` toolset (like the other GUI affordances), which
//! the GUI gateway enables only for desktop-sourced sessions. Emits
//! `pane.reveal` through the shared [`crate::desktop_ui`] bridge; the
//! renderer runs each pane's own reveal path and only acts on the active
//! window (a background turn never moves the user's focus). To show a
//! URL/file, use `open_preview`.
//!
//! PORT SEAMS:
//! - Upstream wraps `desktop_ui.emit` in `try`/`except Exception`; the
//!   Rust emitter is an infallible closure, so the only reachable failure is
//!   a panic inside the sink — caught with `catch_unwind` and rendered as
//!   `str(exc)` like the sibling `read_preview_tool` does.

use serde_json::{json, Value};
use std::sync::{Arc, LazyLock};

use crate::desktop_ui;
use crate::registry::{registry, tool_error, ToolHandler, ToolResult};

/// The panes the renderer can reveal, in upstream order.
///
/// PARITY: `PANES` (upstream line 16).
pub const PANES: [&str; 5] = ["chat", "files", "terminal", "review", "sessions"];

/// The toolset this tool registers into (the GUI gateway enables it only
/// for desktop-sourced sessions).
pub const FOCUS_PANE_TOOLSET: &str = "desktop_ui";

/// Ask the desktop GUI to reveal and focus `pane`.
///
/// PARITY: `focus_pane_tool` (upstream lines 19-32). The pane name is
/// normalized (`(pane or "").strip().lower()`); an unknown or blank pane is
/// a tool error naming the allowed panes. A wired emitter answers
/// `{"success": true, "pane": name}`; no emitter reports desktop-only.
pub fn focus_pane_tool(pane: &str) -> String {
    let name = pane.trim().to_lowercase();
    if !PANES.contains(&name.as_str()) {
        return tool_error(format!("pane must be one of: {}.", PANES.join(", ")), &[]);
    }

    let emitted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        desktop_ui::emit("pane.reveal", json!({ "pane": name }))
    }));
    match emitted {
        Err(panic) => tool_error(
            format!(
                "Failed to focus the {name} pane: {}",
                crate::read_preview_tool::panic_message(panic)
            ),
            &[],
        ),
        Ok(false) => tool_error(
            "Pane focus is only available in the Hermes desktop app.",
            &[],
        ),
        Ok(true) => serde_json::to_string(&json!({ "success": true, "pane": name })).expect("json"),
    }
}

/// PARITY: `FOCUS_PANE_SCHEMA` (upstream lines 35-55).
pub static FOCUS_PANE_SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    json!({
        "name": "focus_pane",
        "description": (
            "Reveal and focus a pane in the Hermes desktop app when the user asks to \
             see it — e.g. \"show me the terminal\", \"open the file browser\", \"show \
             the diff\". Panes: chat (the conversation), files (project file browser), \
             terminal (embedded shell), review (git diff), sessions (the session list). \
             To show a URL or file in the preview pane, use open_preview instead."
        ),
        "parameters": {
            "type": "object",
            "properties": {
                "pane": {
                    "type": "string",
                    "enum": PANES,
                    "description": "Which pane to reveal.",
                },
            },
            "required": ["pane"],
        },
    })
});

struct FocusPaneHandler;

impl ToolHandler for FocusPaneHandler {
    fn call(&self, args: Value, _: Option<&str>, _: Option<&str>) -> ToolResult {
        // Upstream: `pane=args.get("pane", "")`.
        let pane = args.get("pane").and_then(Value::as_str).unwrap_or("");
        ToolResult::Text(focus_pane_tool(pane))
    }
}

/// Register the `focus_pane` tool into the registry singleton (`desktop_ui`
/// toolset, no check_fn — the GUI gateway gates by toolset).
///
/// PARITY: `tools/focus_pane_tool.py` module-level `registry.register`
/// (upstream lines 58-64).
pub fn register_focus_pane() {
    registry()
        .register(
            "focus_pane",
            FOCUS_PANE_TOOLSET,
            FOCUS_PANE_SCHEMA.clone(),
            Arc::new(FocusPaneHandler),
            None,
            None,
            vec![],
            None,
            Some("🪟".to_string()),
            None,
            None,
            None,
            false,
        )
        .expect("register focus_pane");
}
