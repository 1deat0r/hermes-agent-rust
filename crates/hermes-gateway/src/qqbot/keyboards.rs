//! QQ Bot inline keyboards + approval / update-prompt builders and parsers.
//!
//! PARITY: `gateway/platforms/qqbot/keyboards.py` @ b9aa928 — PARTIAL:
//! the keyboard dataclasses, INTERACTION_CREATE parsing, keyboard builders,
//! and the approval text renderers are ported; `ApprovalSender` (async
//! HTTP orchestration over adapter callables) stays PENDING with the
//! adapter transport.
//!
//! `button_data` formats:
//!
//! ```text
//! approve:<session_key>:<decision>      # decision = allow-once|allow-always|deny
//! update_prompt:<answer>                # answer = y|n
//! ```
//!
//! Ported from WideLee's qqbot-agent-sdk v1.2.2 (`approval.py` + `dto.py`
//! keyboard types). Authorship preserved via Co-authored-by.

use serde_json::{json, Map, Value};

// ── button_data prefixes ─────────────────────────────────────────────

/// PARITY: `APPROVAL_BUTTON_PREFIX` (upstream line 33).
pub const APPROVAL_BUTTON_PREFIX: &str = "approve:";
/// PARITY: `UPDATE_PROMPT_PREFIX` (upstream line 34).
pub const UPDATE_PROMPT_PREFIX: &str = "update_prompt:";

/// Pattern: `approve:<session_key>:<decision>` — session_key may itself
/// contain colons (e.g. `agent:main:qqbot:c2c:OPENID`), so the session_key
/// group is greedy but trails the decision.
///
/// PARITY: `_APPROVAL_DATA_RE` (upstream lines 37-39).
fn parse_approval(button_data: &str) -> Option<(String, String)> {
    let rest = button_data.strip_prefix("approve:")?;
    let decision = ["allow-once", "allow-always", "deny"]
        .iter()
        .find(|decision| rest.ends_with(*decision))?;
    // Greedy session key: the longest prefix ending before the trailing
    // decision, requiring at least one character (`.+`).
    let key_len = rest.len() - decision.len();
    if key_len < 2 {
        return None;
    }
    let session_key = &rest[..key_len - 1];
    if session_key.is_empty() {
        return None;
    }
    Some((session_key.to_string(), (*decision).to_string()))
}

/// Pattern: `update_prompt:y` | `update_prompt:n`.
///
/// PARITY: `_UPDATE_PROMPT_RE` / `parse_update_prompt_button_data`
/// (upstream lines 42-43, 174-180).
pub fn parse_update_prompt_button_data(button_data: &str) -> Option<String> {
    let answer = button_data.strip_prefix("update_prompt:")?;
    if answer == "y" || answer == "n" {
        Some(answer.to_string())
    } else {
        None
    }
}

/// PARITY: `parse_approval_button_data` (upstream lines 161-172).
pub fn parse_approval_button_data(button_data: &str) -> Option<(String, String)> {
    parse_approval(button_data)
}

// ── keyboard dataclasses ─────────────────────────────────────────────

/// Button permission metadata. `type = 2` means all users can click.
///
/// PARITY: `KeyboardButtonPermission` (upstream lines 48-53).
#[derive(Debug, Clone)]
pub struct KeyboardButtonPermission {
    pub kind: i64,
}

impl Default for KeyboardButtonPermission {
    fn default() -> Self {
        Self { kind: 2 }
    }
}

impl KeyboardButtonPermission {
    pub fn to_dict(&self) -> Value {
        json!({ "type": self.kind })
    }
}

/// What happens when the button is clicked.
///
/// `kind = 1` (Callback — triggers INTERACTION_CREATE) or `2` (Link).
///
/// PARITY: `KeyboardButtonAction` (upstream lines 56-76).
#[derive(Debug, Clone)]
pub struct KeyboardButtonAction {
    pub kind: i64,
    pub data: String,
    pub permission: KeyboardButtonPermission,
    pub click_limit: i64,
}

impl Default for KeyboardButtonAction {
    fn default() -> Self {
        Self {
            kind: 0,
            data: String::new(),
            permission: KeyboardButtonPermission::default(),
            click_limit: 1,
        }
    }
}

impl KeyboardButtonAction {
    pub fn to_dict(&self) -> Value {
        json!({
            "type": self.kind,
            "data": self.data,
            "permission": self.permission.to_dict(),
            "click_limit": self.click_limit,
        })
    }
}

/// Visual rendering of a button.
///
/// PARITY: `KeyboardButtonRenderData` (upstream lines 79-95). `style`:
/// `0` = grey, `1` = blue.
#[derive(Debug, Clone)]
pub struct KeyboardButtonRenderData {
    pub label: String,
    pub visited_label: String,
    pub style: i64,
}

impl KeyboardButtonRenderData {
    pub fn to_dict(&self) -> Value {
        json!({
            "label": self.label,
            "visited_label": self.visited_label,
            "style": self.style,
        })
    }
}

/// One button in a keyboard. Buttons sharing a `group_id` are mutually
/// exclusive — clicking one greys the rest.
///
/// PARITY: `KeyboardButton` (upstream lines 98-118).
#[derive(Debug, Clone)]
pub struct KeyboardButton {
    pub id: String,
    pub render_data: KeyboardButtonRenderData,
    pub action: KeyboardButtonAction,
    pub group_id: String,
}

impl KeyboardButton {
    pub fn to_dict(&self) -> Value {
        json!({
            "id": self.id,
            "render_data": self.render_data.to_dict(),
            "action": self.action.to_dict(),
            "group_id": self.group_id,
        })
    }
}

/// PARITY: `KeyboardRow` (upstream lines 121-126).
#[derive(Debug, Clone, Default)]
pub struct KeyboardRow {
    pub buttons: Vec<KeyboardButton>,
}

impl KeyboardRow {
    pub fn to_dict(&self) -> Value {
        json!({ "buttons": self.buttons.iter().map(|b| b.to_dict()).collect::<Vec<_>>() })
    }
}

/// PARITY: `KeyboardContent` (upstream lines 129-134).
#[derive(Debug, Clone, Default)]
pub struct KeyboardContent {
    pub rows: Vec<KeyboardRow>,
}

impl KeyboardContent {
    pub fn to_dict(&self) -> Value {
        json!({ "rows": self.rows.iter().map(|r| r.to_dict()).collect::<Vec<_>>() })
    }
}

/// Top-level keyboard payload — goes into `MessageToCreate.keyboard`.
///
/// PARITY: `InlineKeyboard` (upstream lines 137-142).
#[derive(Debug, Clone, Default)]
pub struct InlineKeyboard {
    pub content: KeyboardContent,
}

impl InlineKeyboard {
    pub fn to_dict(&self) -> Value {
        json!({ "content": self.content.to_dict() })
    }
}

// ── keyboard builders ────────────────────────────────────────────────

/// PARITY: `_make_callback_button` (upstream lines 184-201).
fn make_callback_button(
    btn_id: &str,
    label: &str,
    visited_label: &str,
    data: String,
    style: i64,
    group_id: &str,
) -> KeyboardButton {
    KeyboardButton {
        id: btn_id.to_string(),
        render_data: KeyboardButtonRenderData {
            label: label.to_string(),
            visited_label: visited_label.to_string(),
            style,
        },
        action: KeyboardButtonAction {
            kind: 1,
            data,
            ..KeyboardButtonAction::default()
        },
        group_id: group_id.to_string(),
    }
}

/// Build the approval keyboard, hiding persistent scope when unavailable.
///
/// Layout: `[✅ 允许一次] [⭐ 始终允许] [❌ 拒绝]` — all three share
/// `group_id = "approval"` so clicking one greys out the rest.
///
/// PARITY: `build_approval_keyboard` (upstream lines 204-232).
pub fn build_approval_keyboard(session_key: &str, allow_permanent: bool) -> InlineKeyboard {
    let mut buttons = vec![make_callback_button(
        "allow",
        "✅ 允许一次",
        "已允许",
        format!("{APPROVAL_BUTTON_PREFIX}{session_key}:allow-once"),
        1,
        "approval",
    )];
    if allow_permanent {
        buttons.push(make_callback_button(
            "always",
            "⭐ 始终允许",
            "已始终允许",
            format!("{APPROVAL_BUTTON_PREFIX}{session_key}:allow-always"),
            1,
            "approval",
        ));
    }
    buttons.push(make_callback_button(
        "deny",
        "❌ 拒绝",
        "已拒绝",
        format!("{APPROVAL_BUTTON_PREFIX}{session_key}:deny"),
        0,
        "approval",
    ));
    InlineKeyboard {
        content: KeyboardContent {
            rows: vec![KeyboardRow { buttons }],
        },
    }
}

/// Build a Yes/No keyboard for update confirmation prompts.
///
/// PARITY: `build_update_prompt_keyboard` (upstream lines 235-253).
pub fn build_update_prompt_keyboard() -> InlineKeyboard {
    InlineKeyboard {
        content: KeyboardContent {
            rows: vec![KeyboardRow {
                buttons: vec![
                    make_callback_button(
                        "yes",
                        "✓ 确认",
                        "已确认",
                        format!("{UPDATE_PROMPT_PREFIX}y"),
                        1,
                        "update_prompt",
                    ),
                    make_callback_button(
                        "no",
                        "✗ 取消",
                        "已取消",
                        format!("{UPDATE_PROMPT_PREFIX}n"),
                        0,
                        "update_prompt",
                    ),
                ],
            }],
        },
    }
}

// ── ApprovalRequest + text builder ───────────────────────────────────

/// Structured approval-request display data.
///
/// PARITY: `ApprovalRequest` (upstream lines 256-270).
#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub session_key: String,
    pub title: String,
    pub description: String,
    pub command_preview: String,
    pub cwd: String,
    pub tool_name: String,
    /// `'critical' | 'info' | ''`.
    pub severity: String,
    pub timeout_sec: i64,
    pub allow_permanent: bool,
}

impl Default for ApprovalRequest {
    fn default() -> Self {
        Self {
            session_key: String::new(),
            title: String::new(),
            description: String::new(),
            command_preview: String::new(),
            cwd: String::new(),
            tool_name: String::new(),
            severity: String::new(),
            timeout_sec: 120,
            allow_permanent: true,
        }
    }
}

/// Render an [`ApprovalRequest`] into the message body (markdown).
///
/// PARITY: `build_approval_text` (upstream lines 273-276).
pub fn build_approval_text(req: &ApprovalRequest) -> String {
    if !req.command_preview.is_empty() || !req.cwd.is_empty() {
        return build_exec_text(req);
    }
    build_plugin_text(req)
}

/// PARITY: `_build_exec_text` (upstream lines 279-293).
fn build_exec_text(req: &ApprovalRequest) -> String {
    let mut lines: Vec<String> = vec!["🔐 **命令执行审批**".to_string(), String::new()];
    if !req.command_preview.is_empty() {
        let preview: String = req.command_preview.chars().take(300).collect();
        lines.push(format!("```\n{preview}\n```"));
    }
    if !req.cwd.is_empty() {
        lines.push(format!("📁 目录: {}", req.cwd));
    }
    if !req.title.is_empty() && req.title != req.command_preview {
        lines.push(format!("📋 {}", req.title));
    }
    if !req.description.is_empty() {
        lines.push(format!("📝 {}", req.description));
    }
    lines.push(String::new());
    lines.push(format!("⏱️ 超时: {} 秒", req.timeout_sec));
    lines.join("\n")
}

/// PARITY: `_build_plugin_text` (upstream lines 296-310).
fn build_plugin_text(req: &ApprovalRequest) -> String {
    let icon = if req.severity == "critical" {
        "🔴"
    } else if req.severity == "info" {
        "🔵"
    } else {
        "🟡"
    };
    let mut lines: Vec<String> = vec![format!("{icon} **审批请求**"), String::new()];
    lines.push(format!("📋 {}", req.title));
    if !req.description.is_empty() {
        lines.push(format!("📝 {}", req.description));
    }
    if !req.tool_name.is_empty() {
        lines.push(format!("🔧 工具: {}", req.tool_name));
    }
    lines.push(String::new());
    lines.push(format!("⏱️ 超时: {} 秒", req.timeout_sec));
    lines.join("\n")
}

// ── INTERACTION_CREATE event shape ───────────────────────────────────

/// Parsed `INTERACTION_CREATE` event payload.
///
/// PARITY: `InteractionEvent` (upstream lines 405-437).
#[derive(Debug, Clone, Default)]
pub struct InteractionEvent {
    /// Interaction event id — required for the `PUT /interactions/{id}` ACK.
    pub id: String,
    /// Event type code (`11` = message button).
    pub kind: i64,
    /// `0` = guild, `1` = group, `2` = c2c.
    pub chat_type: i64,
    /// `'guild'` | `'group'` | `'c2c'` — human-readable scene.
    pub scene: String,
    pub group_openid: String,
    pub group_member_openid: String,
    pub user_openid: String,
    pub channel_id: String,
    pub guild_id: String,
    pub button_data: String,
    pub button_id: String,
    pub resolver_user_id: String,
}

impl InteractionEvent {
    /// Best available operator openid (group → member; c2c → user).
    ///
    /// PARITY: the `operator_openid` property (upstream lines 429-434) —
    /// Python `or`-falsiness over the three slots.
    pub fn operator_openid(&self) -> &str {
        if !self.group_member_openid.is_empty() {
            &self.group_member_openid
        } else if !self.user_openid.is_empty() {
            &self.user_openid
        } else {
            &self.resolver_user_id
        }
    }
}

/// Parse a raw `INTERACTION_CREATE` dispatch payload (`d`).
///
/// PARITY: `parse_interaction_event` (upstream lines 440-461).
pub fn parse_interaction_event(raw: &Value) -> InteractionEvent {
    let empty = Value::Object(Map::new());
    let data_raw = raw.get("data").unwrap_or(&empty);
    let resolved = data_raw.get("resolved").unwrap_or(&empty);
    let get_str = |obj: &Value, key: &str| -> String {
        obj.get(key)
            .map(|v| match v {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => other.to_string(),
            })
            .unwrap_or_default()
    };
    let scene_code = raw.get("chat_type").and_then(Value::as_i64).unwrap_or(0);
    let scene = match scene_code {
        0 => "guild",
        1 => "group",
        2 => "c2c",
        _ => "",
    };
    InteractionEvent {
        id: get_str(raw, "id"),
        kind: data_raw.get("type").and_then(Value::as_i64).unwrap_or(0),
        chat_type: scene_code,
        scene: scene.to_string(),
        group_openid: get_str(raw, "group_openid"),
        group_member_openid: get_str(raw, "group_member_openid"),
        user_openid: get_str(raw, "user_openid"),
        channel_id: get_str(raw, "channel_id"),
        guild_id: get_str(raw, "guild_id"),
        button_data: get_str(resolved, "button_data"),
        button_id: get_str(resolved, "button_id"),
        resolver_user_id: get_str(resolved, "user_id"),
    }
}
