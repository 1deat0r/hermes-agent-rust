//! Parity tests for `tools/open_preview_tool.py` @ b9aa928, mirroring
//! upstream `tests/tools/test_open_preview_tool.py` plus source-derived
//! normalization cases (the `_normalize_target` grammar has no dedicated
//! upstream test file — gap noted in the ledger).

use serde_json::Value;

use hermes_tools::desktop_ui;
use hermes_tools::open_preview_tool::{normalize_target, open_preview_tool, register_open_preview};

// One lock per binary; every test in this file takes it, standing in for the
// upstream autouse `_reset_emitter` fixture (each test controls the emitter;
// never leak one across tests).
static EMITTER_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn with_emitter_reset<F: FnOnce()>(f: F) {
    let _guard = EMITTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    desktop_ui::set_emitter(None);
    f();
    desktop_ui::set_emitter(None);
}

#[test]
fn lives_in_the_gui_surface_toolset() {
    // Reaches a desktop client on ANY backend, including one with no
    // HERMES_DESKTOP in its environment (URL / cloud gateways).
    with_emitter_reset(|| {
        unsafe { std::env::remove_var("HERMES_DESKTOP") };
        register_open_preview();
        let entry = hermes_tools::registry::registry().get_entry("open_preview");
        let entry = entry.expect("open_preview registered");
        assert_eq!(entry.toolset, "desktop_ui");
        assert!(entry.check_fn.is_none());
    });
}

#[test]
fn emitter_failure_is_reported() {
    with_emitter_reset(|| {
        desktop_ui::set_emitter(Some(Box::new(|_, _, _| panic!("no window"))));
        let result = open_preview_tool("https://x.example", "");
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(
            parsed["error"].as_str().unwrap().contains("no window"),
            "{result}"
        );
    });
}

#[test]
fn no_emitter_means_desktop_only() {
    with_emitter_reset(|| {
        let result = open_preview_tool("https://x.example", "");
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            parsed["error"],
            "The preview pane is only available in the Hermes desktop app."
        );
    });
}

#[test]
fn success_payload_round_trips_target_and_label() {
    with_emitter_reset(|| {
        let sent = std::sync::Arc::new(std::sync::Mutex::new(None));
        let sink = sent.clone();
        desktop_ui::set_emitter(Some(Box::new(move |sid, event, payload| {
            *sink.lock().unwrap() = Some((sid, event, payload));
        })));
        let result = open_preview_tool("https://x.example", "Docs");
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["url"], "https://x.example");
        assert_eq!(parsed["label"], "Docs");
        let emitted = sent.lock().unwrap().clone().unwrap();
        assert_eq!(emitted.1, "preview.open");
        assert_eq!(emitted.2["url"], "https://x.example");
        assert_eq!(emitted.2["label"], "Docs");
    });
}

#[test]
fn missing_url_yields_the_required_error() {
    with_emitter_reset(|| {
        let result = open_preview_tool("", "");
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(
            parsed["error"]
                .as_str()
                .unwrap()
                .starts_with("url is required"),
            "{result}"
        );
    });
}

#[test]
fn normalize_target_coaxes_bare_hosts() {
    // Bare domains gain https, localhost hosts gain http.
    assert_eq!(normalize_target("www.cnn.com"), "https://www.cnn.com");
    assert_eq!(normalize_target("localhost:3000"), "http://localhost:3000");
    assert_eq!(
        normalize_target("127.0.0.1:8080/app"),
        "http://127.0.0.1:8080/app"
    );
    assert_eq!(normalize_target("0.0.0.0"), "http://0.0.0.0");
    assert_eq!(normalize_target("[::1]:5173"), "http://[::1]:5173");
    // Case-insensitive host grammar.
    assert_eq!(normalize_target("LOCALHOST:3000"), "http://LOCALHOST:3000");
    assert_eq!(
        normalize_target("WWW.Example.COM"),
        "https://WWW.Example.COM"
    );
    // TLD needs at least two characters.
    assert_eq!(normalize_target("example.c"), "example.c");
    assert_eq!(
        normalize_target("example.io/x?y=1"),
        "https://example.io/x?y=1"
    );
}

#[test]
fn normalize_target_leaves_paths_and_schemes_alone() {
    assert_eq!(normalize_target("https://x.example"), "https://x.example");
    assert_eq!(normalize_target("file:///tmp/a.html"), "file:///tmp/a.html");
    assert_eq!(normalize_target("/abs/path.html"), "/abs/path.html");
    assert_eq!(normalize_target("./rel.html"), "./rel.html");
    assert_eq!(normalize_target("../up.html"), "../up.html");
    assert_eq!(normalize_target("~/notes.md"), "~/notes.md");
    // Whitespace and backtick fences are stripped around the decision.
    assert_eq!(normalize_target("  `www.cnn.com`  "), "https://www.cnn.com");
    // Empty stays empty (the caller's required-url error).
    assert_eq!(normalize_target("   "), "");
    assert_eq!(normalize_target("`"), "");
}
