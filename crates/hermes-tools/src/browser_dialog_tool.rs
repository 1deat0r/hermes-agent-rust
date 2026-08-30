//! Agent-facing tool: respond to a native JS dialog captured by the CDP
//! supervisor.
//!
//! PARITY: `tools/browser_dialog_tool.py` @ b9aa928 (whole module).
//!
//! This tool is response-only — the agent first reads `pending_dialogs`
//! from `browser_snapshot` output, then calls `browser_dialog(action=...)`
//! to accept or dismiss.
//!
//! PORT SEAMS: upstream resolves the live supervisor via
//! `SUPERVISOR_REGISTRY.get(task_id)` from `tools/browser_supervisor.py`
//! (1,518 LOC, PENDING). The Rust port injects the same lookup as a
//! settable sink returning a `DialogResponder` trait object whose
//! `respond_to_dialog` returns the upstream result dict shape
//! (`{"ok": …, "dialog": …}` / `{"ok": false, "error": …}`). The
//! `_browser_dialog_check` CDP-reachability gate lives with
//! `browser_cdp_tool` (PENDING) — the Rust port exports the toolset name
//! and registration for the same surface.

use std::sync::{Arc, Mutex, OnceLock};

use once_cell::sync::Lazy;
use serde_json::{json, Value};

use crate::registry::{registry, ToolHandler, ToolResult};

/// PARITY: `BROWSER_DIALOG_SCHEMA` (upstream lines 28-76).
pub static BROWSER_DIALOG_SCHEMA: Lazy<Value> = Lazy::new(|| {
    json!({
        "name": "browser_dialog",
        "description": (
            "Respond to a native JavaScript dialog (alert / confirm / prompt / \
             beforeunload) that is currently blocking the page.\n\n\
             **Workflow:** call `browser_snapshot` first — if a dialog is open, \
             it appears in the `pending_dialogs` field with `id`, `type`, \
             and `message`. Then call this tool with `action='accept'` or \
             `action='dismiss'`.\n\n\
             **Prompt dialogs:** pass `prompt_text` to supply the response \
             string. Ignored for alert/confirm/beforeunload.\n\n\
             **Multiple dialogs:** if more than one dialog is queued (rare — \
             happens when a second dialog fires while the first is still open), \
             pass `dialog_id` from the snapshot to disambiguate.\n\n\
             **Availability:** only present when a CDP-capable backend is \
             attached — Browserbase sessions, local Chromium-family browser via \
             `/browser connect`, or `browser.cdp_url` in config.yaml. \
             Not available on Camofox (REST-only) or the default Playwright \
             local browser (CDP port is hidden)."
        ),
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["accept", "dismiss"],
                    "description": (
                        "'accept' clicks OK / returns the prompt text. \
                         'dismiss' clicks Cancel / returns null from prompt(). \
                         For `beforeunload` dialogs: 'accept' allows the \
                         navigation, 'dismiss' keeps the page."
                    ),
                },
                "prompt_text": {
                    "type": "string",
                    "description": (
                        "Response string for a `prompt()` dialog. Ignored for \
                         other dialog types. Defaults to empty string."
                    ),
                },
                "dialog_id": {
                    "type": "string",
                    "description": (
                        "Specific dialog to respond to, from \
                         `browser_snapshot.pending_dialogs[].id`. Required \
                         only when multiple dialogs are queued."
                    ),
                },
            },
            "required": ["action"],
        },
    })
});

/// The supervisor surface this tool needs: respond to a pending dialog.
/// Implementations return the upstream result dict
/// (`{"ok": true, "dialog": {...}}` / `{"ok": false, "error": "..."}`).
pub trait DialogResponder: Send + Sync {
    fn respond_to_dialog(
        &self,
        action: &str,
        prompt_text: Option<&str>,
        dialog_id: Option<&str>,
    ) -> Value;
}

/// Lookup seam: `SUPERVISOR_REGISTRY.get(task_id)` — `None` when no
/// supervisor is attached to the task.
pub type SupervisorLookup = dyn Fn(&str) -> Option<Arc<dyn DialogResponder>> + Send + Sync;

static SUPERVISOR_LOOKUP: OnceLock<Mutex<Option<Arc<SupervisorLookup>>>> = OnceLock::new();

fn supervisor_slot() -> &'static Mutex<Option<Arc<SupervisorLookup>>> {
    SUPERVISOR_LOOKUP.get_or_init(|| Mutex::new(None))
}

/// Install the supervisor lookup (the desktop gateway wires this once the
/// CDP supervisor registry exists).
pub fn set_supervisor_lookup(lookup: Option<Arc<SupervisorLookup>>) {
    *supervisor_slot().lock().unwrap_or_else(|e| e.into_inner()) = lookup;
}

/// Respond to a pending dialog on the active task's CDP supervisor.
///
/// PARITY: `browser_dialog` (upstream lines 79-104). `task_id` defaults to
/// "default" (`task_id or "default"`); a missing supervisor returns the
/// "No CDP supervisor is attached" error object; `result.ok` maps to the
/// success object carrying `action` + the returned `dialog`, else the
/// result's error (defaulting to "unknown error").
pub fn browser_dialog(
    action: &str,
    prompt_text: Option<&str>,
    dialog_id: Option<&str>,
    task_id: Option<&str>,
) -> String {
    let effective_task_id = match task_id {
        Some(task_id) if !task_id.is_empty() => task_id.to_string(),
        _ => "default".to_string(),
    };
    let lookup = supervisor_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let supervisor = lookup
        .as_ref()
        .and_then(|lookup| lookup(&effective_task_id));
    let Some(supervisor) = supervisor else {
        return serde_json::to_string(&json!({
            "success": false,
            "error": (
                "No CDP supervisor is attached to this task. Either the \
                 browser backend doesn't expose CDP (Camofox, default \
                 Playwright) or no browser session has been started yet. \
                 Call browser_navigate or /browser connect first."
            )
        }))
        .unwrap_or_default();
    };

    let result = supervisor.respond_to_dialog(action, prompt_text, dialog_id);
    if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let dialog = result.get("dialog").cloned().unwrap_or_else(|| json!({}));
        serde_json::to_string(&json!({
            "success": true,
            "action": action,
            "dialog": dialog,
        }))
        .unwrap_or_default()
    } else {
        let error = result
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        serde_json::to_string(&json!({"success": false, "error": error})).unwrap_or_default()
    }
}

struct BrowserDialogHandler;

impl ToolHandler for BrowserDialogHandler {
    fn call(&self, args: Value, task_id: Option<&str>, _: Option<&str>) -> ToolResult {
        // Upstream: action=args.get("action", ""), task_id from kwargs.
        let action = args.get("action").and_then(Value::as_str).unwrap_or("");
        let prompt_text = args.get("prompt_text").and_then(Value::as_str);
        let dialog_id = args.get("dialog_id").and_then(Value::as_str);
        ToolResult::Text(browser_dialog(action, prompt_text, dialog_id, task_id))
    }
}

/// Register the `browser_dialog` tool into the registry singleton
/// (`browser-cdp` toolset, emoji 💬). The `_browser_dialog_check` gate is
/// wired when `browser_cdp_tool` ports.
///
/// PARITY: `registry.register(name="browser_dialog", toolset="browser-cdp",
/// ...)` (upstream lines 106-116).
pub fn register_browser_dialog() {
    registry()
        .register(
            "browser_dialog",
            "browser-cdp",
            BROWSER_DIALOG_SCHEMA.clone(),
            Arc::new(BrowserDialogHandler),
            None,
            None,
            vec![],
            None,
            Some("💬".to_string()),
            None,
            None,
            None,
            false,
        )
        .expect("register browser_dialog");
}
