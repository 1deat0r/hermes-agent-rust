// Tier: unit/mock — mirrors `tests/tools/test_focus_pane_tool.py` (the
// parametrized `pane.reveal` emit oracle) plus the source's error arms. The
// upstream registry-entry test is covered by asserting exactly what this
// module registers (toolset, schema shape, and enum order).

use hermes_tools::desktop_ui;
use hermes_tools::focus_pane_tool::{
    focus_pane_tool, FOCUS_PANE_SCHEMA, FOCUS_PANE_TOOLSET, PANES,
};
use parking_lot::{Mutex, MutexGuard};
use serde_json::{json, Value};
use std::sync::{Arc, LazyLock};

type Calls = Arc<Mutex<Vec<(String, String, Value)>>>;

/// The desktop_ui emitter is process-global; serialize the tests that
/// install it (cargo test runs a binary's tests in parallel).
fn seam_lock() -> MutexGuard<'static, ()> {
    static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
    LOCK.lock()
}

fn reset_emitter() {
    desktop_ui::set_emitter(None);
}

fn install_recording_emitter(calls: Calls) {
    desktop_ui::set_emitter(Some(Box::new(move |sid, event, payload| {
        calls.lock().push((sid, event, payload));
    })));
}

// Oracle: test_emits_pane_reveal — normalized input, `pane.reveal` payload,
// session sid defaulting to "".
#[test]
fn emits_pane_reveal_for_every_pane() {
    let _guard = seam_lock();
    for pane in PANES {
        reset_emitter();
        let calls: Calls = Arc::new(Mutex::new(Vec::new()));
        install_recording_emitter(calls.clone());

        let out = focus_pane_tool(&format!("  {}  ", pane.to_uppercase()));

        assert_eq!(
            serde_json::from_str::<Value>(&out).expect("json"),
            json!({"success": true, "pane": pane})
        );
        assert_eq!(
            *calls.lock(),
            vec![(
                String::new(),
                "pane.reveal".to_string(),
                json!({"pane": pane})
            )]
        );
    }
    reset_emitter();
}

// Source arm: `(pane or "").strip().lower()` not in PANES → tool_error with
// the exact joined pane list.
#[test]
fn unknown_pane_is_a_tool_error_naming_the_panes() {
    let out = focus_pane_tool("storm");
    assert_eq!(
        serde_json::from_str::<Value>(&out).expect("json"),
        json!({"error": "pane must be one of: chat, files, terminal, review, sessions."})
    );
}

#[test]
fn blank_pane_names_the_allowed_panes() {
    for blank in ["", "   "] {
        let out = focus_pane_tool(blank);
        assert_eq!(
            serde_json::from_str::<Value>(&out).expect("json"),
            json!({"error": "pane must be one of: chat, files, terminal, review, sessions."})
        );
    }
}

// Source arm: `desktop_ui.emit` returns False with no emitter wired — the
// tool is desktop-only.
#[test]
fn without_an_emitter_reports_desktop_only() {
    let _guard = seam_lock();
    reset_emitter();
    assert!(!desktop_ui::available());

    let out = focus_pane_tool("chat");
    assert_eq!(
        serde_json::from_str::<Value>(&out).expect("json"),
        json!({"error": "Pane focus is only available in the Hermes desktop app."})
    );
}

// Source arm: `except Exception` around the emit — an emitter panic lands in
// `Failed to focus the {name} pane: {exc}` (str(exc) equivalent).
#[test]
fn emitter_failure_fails_open_to_a_tool_error() {
    let _guard = seam_lock();
    desktop_ui::set_emitter(Some(Box::new(|_, _, _| panic!("renderer gone"))));

    let out = focus_pane_tool("chat");
    assert_eq!(
        serde_json::from_str::<Value>(&out).expect("json"),
        json!({"error": "Failed to focus the chat pane: renderer gone"})
    );
    reset_emitter();
}

// Oracle: test_lives_in_the_gui_surface_toolset — the module registers into
// `desktop_ui` with no check_fn; assert the registration inputs.
#[test]
fn registration_inputs_match_the_gui_surface() {
    assert_eq!(FOCUS_PANE_TOOLSET, "desktop_ui");
    let schema = &*FOCUS_PANE_SCHEMA;
    assert_eq!(
        schema["description"],
        "Reveal and focus a pane in the Hermes desktop app when the user asks to \
         see it — e.g. \"show me the terminal\", \"open the file browser\", \"show \
         the diff\". Panes: chat (the conversation), files (project file browser), \
         terminal (embedded shell), review (git diff), sessions (the session list). \
         To show a URL or file in the preview pane, use open_preview instead."
    );
    assert_eq!(
        schema["parameters"]["properties"]["pane"]["enum"],
        json!(PANES)
    );
    assert_eq!(schema["parameters"]["required"], json!(["pane"]));
    assert_eq!(
        schema["parameters"]["properties"]["pane"]["description"],
        "Which pane to reveal."
    );
}
