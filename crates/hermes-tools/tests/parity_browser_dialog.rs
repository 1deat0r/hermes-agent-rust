//! Parity tests for `tools/browser_dialog_tool.py` @ b9aa928, mirroring
//! the tool-side surface (the supervisor's dialog queue lives in
//! browser_supervisor.py, PENDING — injected here as a lookup seam).

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use hermes_tools::browser_dialog_tool::{
    browser_dialog, register_browser_dialog, set_supervisor_lookup, DialogResponder,
};
use hermes_tools::registry::registry;

struct FakeSupervisor {
    calls: Mutex<Vec<(String, Option<String>, Option<String>)>>,
    fail_with: Option<String>,
}

impl DialogResponder for FakeSupervisor {
    fn respond_to_dialog(
        &self,
        action: &str,
        prompt_text: Option<&str>,
        dialog_id: Option<&str>,
    ) -> Value {
        self.calls.lock().unwrap().push((
            action.to_string(),
            prompt_text.map(str::to_string),
            dialog_id.map(str::to_string),
        ));
        if let Some(error) = &self.fail_with {
            return json!({"ok": false, "error": error});
        }
        json!({"ok": true, "dialog": {"id": "d1", "type": "prompt", "message": "name?"}})
    }
}

fn install_default() -> Arc<FakeSupervisor> {
    let supervisor = Arc::new(FakeSupervisor {
        calls: Mutex::new(Vec::new()),
        fail_with: None,
    });
    let supervisor_for_lookup = Arc::clone(&supervisor) as Arc<dyn DialogResponder>;
    set_supervisor_lookup(Some(Arc::new(move |task_id: &str| {
        if task_id == "default" {
            Some(Arc::clone(&supervisor_for_lookup))
        } else {
            None
        }
    })));
    supervisor
}

#[test]
fn lives_in_the_browser_cdp_toolset() {
    register_browser_dialog();
    let entry = registry().get_entry("browser_dialog").unwrap();
    assert_eq!(entry.toolset, "browser-cdp");
}

#[test]
fn successful_accept_carries_action_and_dialog() {
    let supervisor = install_default();
    let result = browser_dialog("accept", Some("my name"), None, None);
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["action"], "accept");
    assert_eq!(parsed["dialog"]["id"], "d1");
    // task_id defaulted to "default".
    assert_eq!(supervisor.calls.lock().unwrap()[0].0, "accept");
    assert_eq!(
        supervisor.calls.lock().unwrap()[0].1.as_deref(),
        Some("my name")
    );
}

#[test]
fn no_supervisor_yields_the_desktop_only_error() {
    set_supervisor_lookup(Some(Arc::new(|_: &str| None)));
    let result = browser_dialog("accept", None, None, None);
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["success"], false);
    assert!(parsed["error"]
        .as_str()
        .unwrap()
        .starts_with("No CDP supervisor is attached to this task"));
}

#[test]
fn custom_task_id_selects_its_supervisor() {
    let hit = Arc::new(FakeSupervisor {
        calls: Mutex::new(Vec::new()),
        fail_with: None,
    });
    set_supervisor_lookup(Some(Arc::new(move |task_id: &str| {
        if task_id == "task-7" {
            Some(Arc::new(FakeSupervisor {
                calls: Mutex::new(Vec::new()),
                fail_with: None,
            }) as Arc<dyn DialogResponder>)
        } else {
            // `hit` captured to satisfy move; not used for lookup.
            let _ = &hit;
            None
        }
    })));
    let result = browser_dialog("dismiss", None, None, Some("task-7"));
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["success"], true);
    // Unknown task id has no supervisor -> error object.
    let result = browser_dialog("dismiss", None, None, Some("task-8"));
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["success"], false);
}

#[test]
fn responder_error_passes_through_verbatim() {
    let supervisor = Arc::new(FakeSupervisor {
        calls: Mutex::new(Vec::new()),
        fail_with: Some("no dialog is currently open".to_string()),
    });
    set_supervisor_lookup(Some(Arc::new(move |_: &str| {
        Some(Arc::clone(&supervisor) as Arc<dyn DialogResponder>)
    })));
    let result = browser_dialog("dismiss", None, None, None);
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["error"], "no dialog is currently open");
}

#[test]
fn dialog_id_and_prompt_text_forward_verbatim() {
    let supervisor = install_default();
    browser_dialog("accept", Some("typed answer"), Some("dlg-9"), None);
    let calls = supervisor.calls.lock().unwrap();
    assert_eq!(calls[0].2.as_deref(), Some("dlg-9"));
}
