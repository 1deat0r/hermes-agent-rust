//! Open a URL, dev server, or file in the Hermes desktop GUI's preview
//! pane.
//!
//! PARITY: `tools/open_preview_tool.py` @ b9aa928 (whole module).
//!
//! Lives in the `desktop_ui` toolset, which the GUI gateway enables only for
//! a session whose source is the desktop app — so the schema never reaches a
//! CLI, messaging, or cron agent, and it DOES reach a desktop client on a
//! remote/cloud backend. Emits `preview.open` through the shared
//! `desktop_ui` bridge; the renderer opens the pane beside the chat for the
//! window that asked and never steals focus for a background session.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, LazyLock};

use regex::Regex;
use serde_json::{json, Value};

use crate::desktop_ui;
use crate::registry::{registry, tool_error, ToolHandler, ToolResult};

/// PARITY: `_normalize_target` host regex (upstream line 23) —
/// `^(localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1\])(:\d+)?(/|$)` with re.I.
static LOCALHOST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1\])(:\d+)?(/|$)")
        .expect("localhost re")
});

/// PARITY: `_normalize_target` domain regex (upstream line 26) —
/// `^[\w.-]+\.[a-z]{2,}(:\d+)?(/.*)?$` with re.I (`\w` is unicode-aware
/// exactly like Python's default).
static DOMAIN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^[\w.-]+\.[a-z]{2,}(:\d+)?(/.*)?$").expect("domain re"));

/// Coax a bare host/domain into a fetchable URL; leave paths + schemes
/// alone.
///
/// `www.cnn.com` → `https://www.cnn.com`; `localhost:3000` →
/// `http://localhost:3000`. File paths and explicit schemes pass through
/// for the renderer's preview normalizer to classify.
///
/// PARITY: `_normalize_target` (upstream lines 19-29). The Python
/// `raw.strip().strip("`").strip()` chain strips whitespace, then
/// backticks, then whitespace again.
pub fn normalize_target(raw: &str) -> String {
    let v = raw.trim().trim_matches('`').trim();
    if v.is_empty()
        || v.contains("://")
        || v.starts_with('/')
        || v.starts_with("./")
        || v.starts_with("../")
        || v.starts_with('~')
        || v.starts_with("file:")
    {
        return v.to_string();
    }
    if LOCALHOST_RE.is_match(v) {
        return format!("http://{v}");
    }
    if DOMAIN_RE.is_match(v) {
        return format!("https://{v}");
    }
    v.to_string()
}

/// Ask the desktop GUI to show `url` in the preview pane beside the chat.
///
/// PARITY: `open_preview_tool` (upstream lines 32-47). The except arm
/// around `desktop_ui.emit` is unreachable for error *returns* in Rust (the
/// emitter is infallible) and is reproduced for panics via `catch_unwind` +
/// a str-of-exception-style label, matching `read_preview_tool`; the
/// `if not ok` arm covers the no-emitter ("not the desktop app") case.
pub fn open_preview_tool(url: &str, label: &str) -> String {
    let target = normalize_target(url);
    if target.is_empty() {
        return tool_error(
            "url is required — a web URL (https://…), a localhost dev server, or a \
             file path to show in the preview pane.",
            &[],
        );
    }

    let label = label.trim();
    let payload = json!({ "url": target, "label": label });
    let ok = catch_unwind(AssertUnwindSafe(|| {
        desktop_ui::emit("preview.open", payload)
    }));
    let ok = match ok {
        Ok(ok) => ok,
        Err(panic) => {
            let detail = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "emitter panic".to_string());
            return tool_error(&format!("Failed to open the preview pane: {detail}"), &[]);
        }
    };
    if !ok {
        return tool_error(
            "The preview pane is only available in the Hermes desktop app.",
            &[],
        );
    }

    // `json.dumps(..., ensure_ascii=False)`.
    serde_json::to_string(&json!({ "success": true, "url": target, "label": label })).expect("json")
}

/// PARITY: `OPEN_PREVIEW_SCHEMA` (upstream lines 50-65).
pub static OPEN_PREVIEW_SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    json!({
        "name": "open_preview",
        "description": (
            "Open something in the preview pane beside the chat in the Hermes desktop \
             app. Use this when the user asks to see a page, dev server, or file in the \
             preview pane — e.g. \"open cnn.com in the preview pane\" or \"preview \
             localhost:3000\". Accepts a web URL (a bare domain like www.cnn.com is fine), \
             a localhost dev-server URL, or a file path (HTML renders live; other files \
             show their contents). The pane opens for the current window only."
        ),
        "parameters": {
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": (
                        "What to preview: a web URL (https://… or a bare domain), a \
                         localhost URL (localhost:3000), or a file path."
                    ),
                },
                "label": {
                    "type": "string",
                    "description": "Optional tab label; defaults to the target's name.",
                },
            },
            "required": ["url"],
        },
    })
});

struct OpenPreviewHandler;

impl ToolHandler for OpenPreviewHandler {
    fn call(&self, args: Value, _: Option<&str>, _: Option<&str>) -> ToolResult {
        // Upstream: `url=args.get("url", ""), label=args.get("label", "")`.
        let url = args.get("url").and_then(Value::as_str).unwrap_or("");
        let label = args.get("label").and_then(Value::as_str).unwrap_or("");
        ToolResult::Text(open_preview_tool(url, label))
    }
}

/// Register the `open_preview` tool into the registry singleton
/// (`desktop_ui` toolset, no check_fn — the GUI gateway gates by toolset).
///
/// PARITY: `tools/open_preview_tool.py` module-level `registry.register`
/// (upstream lines 68-74).
pub fn register_open_preview() {
    registry()
        .register(
            "open_preview",
            "desktop_ui",
            OPEN_PREVIEW_SCHEMA.clone(),
            Arc::new(OpenPreviewHandler),
            None,
            None,
            vec![],
            None,
            Some("🖼️".to_string()),
            None,
            None,
            None,
            false,
        )
        .expect("register open_preview");
}
