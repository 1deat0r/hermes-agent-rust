//! Parity tests for `gateway/platforms/qqbot/keyboards.py` @ 5d59366.
//!
//! Oracle: `tests/gateway/test_qqbot.py` (`TestApprovalButtonData`,
//! `TestUpdatePromptButtonData`, `TestBuildApprovalKeyboard`,
//! `TestBuildUpdatePromptKeyboard`, `TestBuildApprovalText`,
//! `TestInteractionEventParsing`) plus coercion edge cases verified live
//! against the oracle interpreter. Tier: `unit`.

use serde_json::json;

use std::sync::{Arc, Mutex};

use hermes_gateway::qqbot::keyboards::{
    build_approval_keyboard, build_approval_text, build_update_prompt_keyboard,
    parse_approval_button_data, parse_interaction_event, parse_update_prompt_button_data,
    ApprovalRequest, ApprovalSender, APPROVAL_BUTTON_PREFIX, UPDATE_PROMPT_PREFIX,
};

// ── button_data parsing ──────────────────────────────────────────────────

#[test]
fn approval_button_data_parses_with_greedy_session_key() {
    // ORACLE: `TestApprovalButtonData::test_parse_allow_once`
    // `("agent:main:qqbot:c2c:UID", "allow-once")`.
    let (key, decision) =
        parse_approval_button_data("approve:agent:main:qqbot:c2c:UID:allow-once").unwrap();
    assert_eq!(key, "agent:main:qqbot:c2c:UID");
    assert_eq!(decision, "allow-once");
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
    // ORACLE: `TestApprovalButtonData::test_parse_empty_returns_none`
    // (`""` and `None` both → `None`; `button_data or ""`).
    assert_eq!(parse_approval_button_data("update_prompt:y"), None);
    assert_eq!(parse_approval_button_data("approve:s:bogus"), None);
    // `.+` needs at least one session-key char before the decision colon.
    assert_eq!(parse_approval_button_data("approve:allow-once"), None);
    assert_eq!(parse_approval_button_data(""), None);
    assert_eq!(parse_approval_button_data("approve:"), None);
}

#[test]
fn approval_button_data_suffix_mimicry_resolves_like_the_greedy_regex() {
    // Oracle-verified: `approve:x-allow-once:deny` → `("x-allow-once", "deny")`
    // (greedy `.+` backtracks past the embedded decision-like text).
    let (key, decision) = parse_approval_button_data("approve:x-allow-once:deny").unwrap();
    assert_eq!(key, "x-allow-once");
    assert_eq!(decision, "deny");
    // Oracle-verified: empty session key never matches
    // (`approve::allow-once` → None), but punctuation-only keys do
    // (`approve:-:deny` → `("-", "deny")`).
    assert_eq!(parse_approval_button_data("approve::allow-once"), None);
    let (key, decision) = parse_approval_button_data("approve:-:deny").unwrap();
    assert_eq!(key, "-");
    assert_eq!(decision, "deny");
}

#[test]
fn button_data_prefix_constants_match_upstream() {
    // ORACLE: `APPROVAL_BUTTON_PREFIX` / `UPDATE_PROMPT_PREFIX` literals.
    assert_eq!(APPROVAL_BUTTON_PREFIX, "approve:");
    assert_eq!(UPDATE_PROMPT_PREFIX, "update_prompt:");
}

#[test]
fn update_prompt_button_data_parses_y_or_n_only() {
    // ORACLE: `TestUpdatePromptButtonData::test_parse_yes`.
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
fn approval_keyboard_matches_upstream_defaults() {
    // ORACLE: `TestBuildApprovalKeyboard` — default call
    // `build_approval_keyboard("session-1")` carries `allow_permanent=True`.
    let dict = build_approval_keyboard("session-1", true).to_dict();
    let rows = dict["content"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["buttons"].as_array().unwrap().len(), 3);
}

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
fn approval_keyboard_session_key_embedding_is_exact() {
    // ORACLE: `TestBuildApprovalKeyboard::test_button_data_embeds_session_key`.
    let dict = build_approval_keyboard("agent:main:qqbot:c2c:UID", true).to_dict();
    let buttons = dict["content"]["rows"][0]["buttons"].as_array().unwrap();
    let datas: Vec<&str> = buttons
        .iter()
        .map(|b| b["action"]["data"].as_str().unwrap())
        .collect();
    assert_eq!(
        datas,
        [
            "approve:agent:main:qqbot:c2c:UID:allow-once",
            "approve:agent:main:qqbot:c2c:UID:allow-always",
            "approve:agent:main:qqbot:c2c:UID:deny",
        ]
    );
}

#[test]
fn update_prompt_keyboard_is_yes_no() {
    // ORACLE: `TestBuildUpdatePromptKeyboard::test_two_buttons` + the
    // `TestSendUpdatePrompt` contract
    // (`datas == ["update_prompt:y", "update_prompt:n"]`).
    let dict = build_update_prompt_keyboard().to_dict();
    let buttons = dict["content"]["rows"][0]["buttons"].as_array().unwrap();
    assert_eq!(buttons.len(), 2);
    let datas: Vec<&str> = buttons
        .iter()
        .map(|b| b["action"]["data"].as_str().unwrap())
        .collect();
    assert_eq!(datas, ["update_prompt:y", "update_prompt:n"]);
    assert!(buttons.iter().all(|b| b["group_id"] == "update_prompt"));
}

#[test]
fn keyboard_wire_shape_matches_upstream_serialization() {
    // ORACLE: `_to_dict` recursion in field-declaration order over the full
    // nested payload (button → render_data/action → permission).
    let keyboard = build_approval_keyboard("s", true);
    let dict = keyboard.to_dict();
    let button = &dict["content"]["rows"][0]["buttons"][0];
    assert_eq!(
        button.as_object().unwrap().keys().collect::<Vec<_>>(),
        ["id", "render_data", "action", "group_id"]
    );
    assert_eq!(
        button["render_data"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["label", "visited_label", "style"]
    );
    assert_eq!(
        button["action"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["type", "data", "permission", "click_limit"]
    );
    assert_eq!(
        button["action"]["permission"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["type"]
    );
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
    // ORACLE: `TestBuildApprovalText::test_truncates_long_commands`
    // (1000-char preview capped inline; timeout line adds no `x`s).
    // Oracle-verified: the capped body holds exactly 301 `x`s
    // (300 preview + the "超时" line contributes none — `cwd="/x"` adds one,
    // so the oracle probe used `cwd="/x"` → 301 total).
    let req = ApprovalRequest {
        session_key: "s".to_string(),
        title: "t".to_string(),
        command_preview: "x".repeat(1000),
        cwd: "/x".to_string(),
        ..ApprovalRequest::default()
    };
    let text = build_approval_text(&req);
    assert!(text.contains(&"x".repeat(300)));
    assert!(!text.contains(&"x".repeat(302)));
    assert_eq!(text.matches('x').count(), 301, "{text}");
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
    // ORACLE: `TestInteractionEventParsing::test_parse_c2c_interaction`.
    let raw = json!({
        "id": "interaction-42",
        "chat_type": 2,
        "user_openid": "user-1",
        "data": {
            "type": 11,
            "resolved": {
                "button_data": "approve:sess:allow-once",
                "button_id": "allow",
            }
        }
    });
    let event = parse_interaction_event(&raw);
    assert_eq!(event.id, "interaction-42");
    assert_eq!(event.kind, 11);
    assert_eq!(event.chat_type, 2);
    assert_eq!(event.scene, "c2c");
    assert_eq!(event.user_openid, "user-1");
    assert_eq!(event.button_data, "approve:sess:allow-once");
    assert_eq!(event.button_id, "allow");
    assert_eq!(event.operator_openid(), "user-1");
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

#[test]
fn interaction_event_coercions_match_upstream_int_and_str() {
    // Oracle-verified against the interpreter (`int(...)` / `str(...)`):
    // - numeric strings coerce (`"2"` → 2, `"11"` → 11);
    // - bools are ints (`True` → `id "True"`, `chat_type` 1);
    // - `None` payloads stringify (`id None` → `"None"`), missing `data`
    //   maps (`None`/`False`) fall back to empty objects.
    let event = parse_interaction_event(&json!({
        "id": 42, "chat_type": "2",
        "data": {"type": "11", "resolved": {"button_data": "x"}},
    }));
    assert_eq!(event.id, "42");
    assert_eq!(event.chat_type, 2);
    assert_eq!(event.scene, "c2c");
    assert_eq!(event.kind, 11);

    let event = parse_interaction_event(&json!({"id": true, "chat_type": true}));
    assert_eq!(event.id, "True");
    assert_eq!(event.chat_type, 1);
    assert_eq!(event.scene, "group");

    let event = parse_interaction_event(&json!({"id": null}));
    assert_eq!(event.id, "None");
    let event = parse_interaction_event(&json!({"data": null}));
    assert_eq!(event.button_data, "");
    let event = parse_interaction_event(&json!({"data": false}));
    assert_eq!(event.button_data, "");
    assert_eq!(event.kind, 0);
}

#[test]
fn interaction_event_scalar_stringification_matches_python_repr() {
    // Oracle-verified `str(...)` shapes: floats, dicts, lists, resolver ints.
    let event = parse_interaction_event(&json!({"id": 3.7}));
    assert_eq!(event.id, "3.7");
    let event = parse_interaction_event(&json!({"id": {"a": 1}}));
    assert_eq!(event.id, "{'a': 1}");
    let event = parse_interaction_event(&json!({"id": ["x"]}));
    assert_eq!(event.id, "['x']");
    let event = parse_interaction_event(&json!({
        "data": {"type": 11, "resolved": {"button_data": 5}},
    }));
    assert_eq!(event.kind, 11);
    assert_eq!(event.button_data, "5");
    let event = parse_interaction_event(&json!({"data": {"type": 11.9}}));
    assert_eq!(event.kind, 11, "float codes truncate like int()");
    let event = parse_interaction_event(&json!({"chat_type": 2.9}));
    assert_eq!(event.chat_type, 2);
    assert_eq!(event.scene, "c2c");
}

#[test]
fn interaction_event_malformed_shapes_fail_open_to_zero() {
    // DOCUMENTED DIVERGENCE (see `py_int`): upstream `int()` raises on
    // non-numeric strings / wrong-type payloads / float-shaped strings;
    // the port returns 0 (same drop-the-event outcome at the adapter's
    // try/except boundary). Pinned as 0 so strictness changes fail loudly.
    assert_eq!(
        parse_interaction_event(&json!({"chat_type": "abc"})).chat_type,
        0
    );
    assert_eq!(
        parse_interaction_event(&json!({"chat_type": "2.7"})).chat_type,
        2,
        "float-shaped strings coerce via float()"
    );
    assert_eq!(
        parse_interaction_event(&json!({"data": {"type": "x"}})).kind,
        0
    );
    // DOCUMENTED DIVERGENCE: truthy non-dict `data`/`resolved` raise
    // AttributeError upstream; the port falls back to empty.
    let event = parse_interaction_event(&json!({"data": "x"}));
    assert_eq!(event.button_data, "");
    assert_eq!(event.kind, 0);
    let event = parse_interaction_event(&json!({"id": [1, "a", null, true]}));
    assert_eq!(event.id, "[1, 'a', None, True]");
    let event = parse_interaction_event(&json!({"id": {"a": [1, {"b": null}]}}));
    assert_eq!(event.id, "{'a': [1, {'b': None}]}");
    // Falsy-but-present values take the `or 0` path upstream (no raise).
    assert_eq!(
        parse_interaction_event(&json!({"chat_type": ""})).chat_type,
        0
    );
    assert_eq!(
        parse_interaction_event(&json!({"data": {"type": ""}})).kind,
        0
    );
    let event = parse_interaction_event(&json!({"data": {"resolved": "x"}}));
    assert_eq!(event.button_data, "");
}

#[test]
fn button_data_parsers_are_case_sensitive() {
    // Oracle-verified: decisions/answers are lowercase literals only.
    assert_eq!(parse_approval_button_data("approve:ALLOW-ONCE:x"), None);
    assert_eq!(parse_approval_button_data("approve:x:Allow-Once"), None);
    assert_eq!(parse_update_prompt_button_data("update_prompt:Y"), None);
    assert_eq!(parse_update_prompt_button_data("update_prompt:"), None);
    assert_eq!(parse_update_prompt_button_data("update_prompt:y "), None);
}

#[test]
fn approval_text_edge_cases_match_upstream_branches() {
    // Oracle-verified: `title != command_preview` with empty cwd still
    // takes the exec branch and still renders the title line.
    let req = ApprovalRequest {
        session_key: "s".to_string(),
        title: "t".to_string(),
        command_preview: "same".to_string(),
        timeout_sec: 5,
        ..ApprovalRequest::default()
    };
    let text = build_approval_text(&req);
    assert!(text.starts_with("🔐 **命令执行审批**"));
    assert!(text.contains("📋 t"));
    // Oracle-verified: empty title renders the bare `📋 ` line; unknown
    // severity falls back to 🟡.
    let req = ApprovalRequest {
        session_key: "s".to_string(),
        description: "d".to_string(),
        tool_name: "tn".to_string(),
        severity: "weird".to_string(),
        ..ApprovalRequest::default()
    };
    let text = build_approval_text(&req);
    assert!(text.starts_with("🟡 **审批请求**"));
    assert!(text.contains("📋 \n"));
}

// ── ApprovalSender ─────────────────────────────────────────────────────
// ORACLE: `ApprovalSender.send` (no dedicated upstream test — behavior
// pinned from the implementation: c2c/group dispatch, fail-closed unknown
// chat_type, fail-closed post errors). Tier: `unit`.

#[derive(Debug, Default)]
struct CapturedPost {
    chat_id: String,
    text: String,
    reply_to: Option<String>,
    buttons: usize,
    first_data: String,
}

fn recording_sender(captured: Arc<Mutex<CapturedPost>>, fail: bool) -> ApprovalSender {
    let store = captured.clone();
    let fail_flag = fail;
    let post = move |chat_id: String,
                     text: String,
                     reply_to: Option<String>,
                     keyboard: hermes_gateway::qqbot::keyboards::InlineKeyboard|
          -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let store = store.clone();
        Box::pin(async move {
            if fail_flag {
                panic!("post transport failed");
            }
            let dict = keyboard.to_dict();
            let buttons = dict["content"]["rows"][0]["buttons"].as_array().unwrap();
            *store.lock().unwrap() = CapturedPost {
                chat_id,
                text,
                reply_to,
                buttons: buttons.len(),
                first_data: buttons[0]["action"]["data"].as_str().unwrap().to_string(),
            };
        })
    };
    let store = captured.clone();
    let post_group = move |chat_id: String,
                           text: String,
                           reply_to: Option<String>,
                           keyboard: hermes_gateway::qqbot::keyboards::InlineKeyboard|
          -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let store = store.clone();
        Box::pin(async move {
            if fail {
                panic!("post transport failed");
            }
            let dict = keyboard.to_dict();
            let buttons = dict["content"]["rows"][0]["buttons"].as_array().unwrap();
            *store.lock().unwrap() = CapturedPost {
                chat_id,
                text,
                reply_to,
                buttons: buttons.len(),
                first_data: buttons[0]["action"]["data"].as_str().unwrap().to_string(),
            };
        })
    };
    // Silence the unused binding while keeping symmetric construction.
    let _ = &post;
    ApprovalSender::new(post, post_group)
}

fn approval_req() -> ApprovalRequest {
    ApprovalRequest {
        session_key: "sess-9".to_string(),
        title: "Run build".to_string(),
        command_preview: "cargo test".to_string(),
        ..ApprovalRequest::default()
    }
}

#[test]
fn approval_sender_routes_c2c_and_returns_true() {
    let captured = Arc::new(Mutex::new(CapturedPost::default()));
    let sender = recording_sender(captured.clone(), false);
    assert_eq!(sender.log_tag(), "QQBot");
    assert!(sender.send("c2c", "U1", &approval_req(), Some("m-1")));
    let got = captured.lock().unwrap();
    assert_eq!(got.chat_id, "U1");
    assert_eq!(got.reply_to.as_deref(), Some("m-1"));
    assert!(got.text.contains("🔐 **命令执行审批**"));
    assert!(got.text.contains("cargo test"));
    assert_eq!(got.buttons, 3);
    assert_eq!(got.first_data, "approve:sess-9:allow-once");
}

#[test]
fn approval_sender_routes_group_without_reply_to() {
    let captured = Arc::new(Mutex::new(CapturedPost::default()));
    let sender = recording_sender(captured.clone(), false);
    assert!(sender.send("group", "G1", &approval_req(), None));
    let got = captured.lock().unwrap();
    assert_eq!(got.chat_id, "G1");
    assert_eq!(got.reply_to, None);
    assert_eq!(got.buttons, 3);
}

#[test]
fn approval_sender_rejects_unknown_chat_type_fail_closed() {
    // Upstream logs a warning and returns False; the post callable must
    // never run.
    let captured = Arc::new(Mutex::new(CapturedPost::default()));
    let sender = recording_sender(captured.clone(), false);
    assert!(!sender.send("guild", "G1", &approval_req(), None));
    let got = captured.lock().unwrap();
    assert_eq!(got.chat_id, "");
}

#[test]
fn approval_sender_post_error_returns_false() {
    // Upstream `try/except Exception → False`.
    let captured = Arc::new(Mutex::new(CapturedPost::default()));
    let sender = recording_sender(captured.clone(), true);
    assert!(!sender.send("c2c", "U1", &approval_req(), None));
}
