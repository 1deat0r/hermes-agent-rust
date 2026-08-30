//! Parity tests for `gateway/platforms/qqbot/keyboards.py` (partial port)
//! @ b9aa928. Upstream has no dedicated test file (missing-test gap, noted
//! in the ledger); cases derive from the upstream code as oracle.

use serde_json::json;

use hermes_gateway::qqbot::keyboards::{
    build_approval_keyboard, build_approval_text, build_update_prompt_keyboard,
    parse_approval_button_data, parse_interaction_event, parse_update_prompt_button_data,
    ApprovalRequest, APPROVAL_BUTTON_PREFIX,
};

// ── button_data parsing ──────────────────────────────────────────────────

#[test]
fn approval_button_data_parses_with_greedy_session_key() {
    let (key, decision) =
        parse_approval_button_data("approve:agent:main:qqbot:c2c:OPENID:allow-once").unwrap();
    assert_eq!(key, "agent:main:qqbot:c2c:OPENID");
    assert_eq!(decision, "allow-once");
    let (key, decision) = parse_approval_button_data("approve:my-session:deny").unwrap();
    assert_eq!(key, "my-session");
    assert_eq!(decision, "deny");
    let (key, decision) = parse_approval_button_data("approve:s:allow-always").unwrap();
    assert_eq!(key, "s");
    assert_eq!(decision, "allow-always");
}

#[test]
fn approval_button_data_rejects_non_approval_and_bad_decisions() {
    assert_eq!(parse_approval_button_data("update_prompt:y"), None);
    assert_eq!(parse_approval_button_data("approve:s:bogus"), None);
    // `.+` needs at least one session-key char before the decision colon.
    assert_eq!(parse_approval_button_data("approve:allow-once"), None);
    assert_eq!(parse_approval_button_data(""), None);
}

#[test]
fn update_prompt_button_data_parses_y_or_n_only() {
    assert_eq!(
        parse_update_prompt_button_data("update_prompt:y").as_deref(),
        Some("y")
    );
    assert_eq!(
        parse_update_prompt_button_data("update_prompt:n").as_deref(),
        Some("n")
    );
    assert_eq!(parse_update_prompt_button_data("update_prompt:maybe"), None);
    assert_eq!(parse_update_prompt_button_data("approve:s:deny"), None);
}

// ── keyboard builders ────────────────────────────────────────────────────

#[test]
fn approval_keyboard_full_layout() {
    let keyboard = build_approval_keyboard("sess-1", true);
    let dict = keyboard.to_dict();
    let rows = dict["content"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    let buttons = rows[0]["buttons"].as_array().unwrap();
    assert_eq!(buttons.len(), 3);
    // allow-once button.
    assert_eq!(buttons[0]["id"], "allow");
    assert_eq!(buttons[0]["render_data"]["label"], "✅ 允许一次");
    assert_eq!(buttons[0]["render_data"]["visited_label"], "已允许");
    assert_eq!(buttons[0]["render_data"]["style"], 1);
    assert_eq!(
        buttons[0]["action"]["data"],
        format!("{APPROVAL_BUTTON_PREFIX}sess-1:allow-once")
    );
    assert_eq!(buttons[0]["action"]["type"], 1);
    assert_eq!(buttons[0]["action"]["click_limit"], 1);
    assert_eq!(buttons[0]["action"]["permission"]["type"], 2);
    // always button.
    assert_eq!(buttons[1]["id"], "always");
    assert_eq!(buttons[1]["action"]["data"], "approve:sess-1:allow-always");
    // deny button is grey-styled.
    assert_eq!(buttons[2]["id"], "deny");
    assert_eq!(buttons[2]["render_data"]["style"], 0);
    assert_eq!(buttons[2]["action"]["data"], "approve:sess-1:deny");
    // All share the approval group (mutual exclusion).
    assert!(buttons.iter().all(|b| b["group_id"] == "approval"));
}

#[test]
fn approval_keyboard_hides_permanent_when_unavailable() {
    let keyboard = build_approval_keyboard("sess-1", false);
    let dict = keyboard.to_dict();
    let buttons = dict["content"]["rows"][0]["buttons"].as_array().unwrap();
    assert_eq!(buttons.len(), 2);
    assert_eq!(buttons[0]["id"], "allow");
    assert_eq!(buttons[1]["id"], "deny");
}

#[test]
fn update_prompt_keyboard_is_yes_no() {
    let dict = build_update_prompt_keyboard().to_dict();
    let buttons = dict["content"]["rows"][0]["buttons"].as_array().unwrap();
    assert_eq!(buttons.len(), 2);
    assert_eq!(buttons[0]["action"]["data"], "update_prompt:y");
    assert_eq!(buttons[1]["action"]["data"], "update_prompt:n");
    assert!(buttons.iter().all(|b| b["group_id"] == "update_prompt"));
}

// ── approval text rendering ──────────────────────────────────────────────

#[test]
fn exec_requests_render_the_command_template() {
    let req = ApprovalRequest {
        session_key: "s".to_string(),
        title: "Run build".to_string(),
        description: "User asked".to_string(),
        command_preview: "cargo test".to_string(),
        cwd: "/workspace".to_string(),
        timeout_sec: 60,
        ..ApprovalRequest::default()
    };
    let text = build_approval_text(&req);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "🔐 **命令执行审批**");
    assert!(text.contains("```\ncargo test\n```"));
    assert!(text.contains("📁 目录: /workspace"));
    assert!(text.contains("📋 Run build"));
    assert!(text.contains("📝 User asked"));
    assert!(text.contains("⏱️ 超时: 60 秒"));
}

#[test]
fn command_preview_is_bounded_to_300_chars() {
    let req = ApprovalRequest {
        session_key: "s".to_string(),
        title: "t".to_string(),
        command_preview: "x".repeat(400),
        ..ApprovalRequest::default()
    };
    let text = build_approval_text(&req);
    assert!(text.contains(&"x".repeat(300)));
    assert!(!text.contains(&"x".repeat(301)));
}

#[test]
fn plugin_requests_render_severity_icons() {
    let base = |severity: &str| ApprovalRequest {
        session_key: "s".to_string(),
        title: "Tool call".to_string(),
        tool_name: "browser_navigate".to_string(),
        severity: severity.to_string(),
        ..ApprovalRequest::default()
    };
    for (severity, icon) in [("critical", "🔴"), ("info", "🔵"), ("", "🟡")] {
        let text = build_approval_text(&base(severity));
        assert!(text.starts_with(&format!("{icon} **审批请求**")), "{text}");
        assert!(text.contains("🔧 工具: browser_navigate"));
    }
}

#[test]
fn title_suppressed_when_equal_to_command_preview() {
    let req = ApprovalRequest {
        session_key: "s".to_string(),
        title: "same".to_string(),
        command_preview: "same".to_string(),
        cwd: "/w".to_string(),
        ..ApprovalRequest::default()
    };
    let text = build_approval_text(&req);
    assert!(!text.contains("📋 same"), "title line is skipped: {text}");
}

// ── INTERACTION_CREATE parsing ───────────────────────────────────────────

#[test]
fn interaction_event_parses_the_dispatch_payload() {
    let raw = json!({
        "id": "inter-1",
        "chat_type": 2,
        "group_openid": "G1",
        "user_openid": "U1",
        "data": {
            "type": 11,
            "resolved": {
                "button_data": "approve:s:allow-once",
                "button_id": "allow",
                "user_id": "R1",
            }
        }
    });
    let event = parse_interaction_event(&raw);
    assert_eq!(event.id, "inter-1");
    assert_eq!(event.kind, 11);
    assert_eq!(event.chat_type, 2);
    assert_eq!(event.scene, "c2c");
    assert_eq!(event.button_data, "approve:s:allow-once");
    assert_eq!(event.button_id, "allow");
    assert_eq!(event.resolver_user_id, "R1");
    // operator_openid: c2c falls through to user openid.
    assert_eq!(event.operator_openid(), "U1");
}

#[test]
fn interaction_event_operator_prefers_group_member_then_user_then_resolver() {
    let mut event = parse_interaction_event(&json!({
        "chat_type": 1,
        "group_member_openid": "M1",
        "user_openid": "U1",
        "data": {"resolved": {"user_id": "R1"}}
    }));
    assert_eq!(event.scene, "group");
    assert_eq!(event.operator_openid(), "M1");
    event.group_member_openid = String::new();
    assert_eq!(event.operator_openid(), "U1");
    event.user_openid = String::new();
    assert_eq!(event.operator_openid(), "R1");
}

#[test]
fn interaction_event_tolerates_missing_fields_and_unknown_scenes() {
    let event = parse_interaction_event(&json!({}));
    assert_eq!(event.id, "");
    assert_eq!(event.kind, 0);
    assert_eq!(event.chat_type, 0);
    assert_eq!(event.scene, "guild", "chat_type 0 = guild");
    assert_eq!(event.operator_openid(), "");

    let event = parse_interaction_event(&json!({"chat_type": 99}));
    assert_eq!(event.scene, "", "unknown scene code maps to empty");
}
