//! QQ Bot inline keyboards + approval / update-prompt builders and parsers.
//!
//! PARITY: `gateway/platforms/qqbot/keyboards.py` @ 5d59366 — the full
//! module surface is ported line by line: the keyboard dataclasses and
//! `_to_dict` wire serialization, both `button_data` parsers, both keyboard
//! builders, `ApprovalRequest` + `build_approval_text`, the
//! `INTERACTION_CREATE` parser, and the `ApprovalSender` orchestration
//! (async post callables; the PLUGIN-COMPAT block is intentionally NOT
//! ported — see below).
//!
//! PORT SEAMS (documented divergences):
//! - The upstream PLUGIN-COMPAT block (`logger` lazy re-export) is an
//!   in-tree compat pointer for external plugins; per repo fidelity rules
//!   ("Compat pointers are OFF LIMITS in-tree") it is not ported.
//! - Upstream `ApprovalSender.send` is `async def` over an asyncio loop.
//!   This crate has no async runtime, so the post callables are boxed
//!   `Fn(String, String, Option<String>, InlineKeyboard) -> Pin<Box<dyn
//!   Future>>` and `send` is a *blocking* function that drives the future
//!   to completion with a minimal no-op-waker executor (same pattern as
//!   `hermes-tools::slash_confirm::resolve`). The observable contract is
//!   identical: text + keyboard are built the same way, `c2c`/`group` route
//!   to the matching callable, anything else returns `false`, exceptions
//!   in the callable return `false` (upstream `try/except` → `False`).
//! - Upstream `operator_openid` is a `@property`; here it is an inherent
//!   method `operator_openid()` (same `or`-chain semantics).
//! - `parse_*` take `&str` (upstream `button_data or ""` means `None` maps
//!   to `""`; callers pass `unwrap_or_default()` — same outcome).
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

use std::future::Future;
use std::pin::Pin;

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
/// Coerce a JSON scalar the way Python `str(value)` does: strings pass
/// through, `None` renders `"None"`, bools render `"True"`/`"False"`
/// (oracle-verified: `id True` → `"True"`), floats render Python-style
/// (`3.7` → `"3.7"`), ints plainly, containers via the Python-repr
/// analogue — single-quoted debug formatting (`{'a': 1}` → `"{'a': 1}"`,
/// `['x']` → `"['x']"`, oracle-verified).
///
/// PARITY: `str(...)` in `parse_interaction_event` (upstream lines 185-191).
fn py_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                // Python `str(3.7)` is `"3.7"`; serde_json agrees here.
                n.as_f64().map(|f| f.to_string()).unwrap_or_default()
            }
        }
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(py_repr_inner).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Object(map) => {
            // Python dict repr single-quotes string keys (`{'a': 1}`).
            let inner: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("'{}': {}", k, py_repr_inner(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
    }
}

/// Inner-repr helper for [`py_str`]: nested containers use the same
/// Python-repr shape; nested strings are single-quoted (`'x'`).
fn py_repr_inner(value: &Value) -> String {
    match value {
        Value::String(s) => format!("{s:?}").replace('"', "'"),
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                n.as_f64().map(|f| f.to_string()).unwrap_or_default()
            }
        }
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(py_repr_inner).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Object(map) => {
            // Python dict repr single-quotes string keys (`{'a': 1}`).
            let inner: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("'{}': {}", k, py_repr_inner(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
    }
}

/// Coerce a JSON value the way Python `int(value or 0)` does: missing/null
/// → `0`, bools → `1`/`0`, numbers truncate toward zero (`11.9` → `11`),
/// numeric strings parse (`"2"` → `2`, whitespace-tolerant like `int()`).
///
/// DIVERGENCE (documented, caller-safe): upstream `int()` *raises* on
/// non-numeric strings (`"abc"` → `ValueError`), wrong-type payloads
/// (lists → `TypeError`), and float-shaped strings (`"2.7"` →
/// `ValueError`); this port returns `0` instead. The sole in-tree caller
/// shape (`QQAdapter._on_interaction`, adapter lines 596-605) wraps the
/// whole parse in `try/except → return`, so both behaviors land in the
/// same "drop the event" outcome — but the port additionally survives
/// where upstream would log-and-drop. Malformed-string cases are pinned
/// in the parity suite as `0` so a future strictness change fails loudly.
///
/// PARITY: `int(raw.get("chat_type", 0) or 0)` and
/// `int(data_raw.get("type", 0) or 0)` (upstream line 184).
fn py_int(value: Option<&Value>) -> i64 {
    match value {
        None | Some(Value::Null) => 0,
        Some(Value::Bool(true)) => 1,
        Some(Value::Bool(false)) => 0,
        Some(Value::Number(n)) => n
            .as_i64()
            .unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64),
        Some(Value::String(s)) => s
            .trim()
            .parse::<i64>()
            .or_else(|_| s.trim().parse::<f64>().map(|f| f as i64))
            .unwrap_or(0),
        Some(_) => 0,
    }
}

/// Parse a raw `INTERACTION_CREATE` dispatch payload (`d`).
///
/// PARITY: `parse_interaction_event` (upstream lines 181-191).
pub fn parse_interaction_event(raw: &Value) -> InteractionEvent {
    let empty = Value::Object(Map::new());
    // Upstream: `raw.get("data") or {}` — falsy non-dict payloads
    // (`None`, `False`, `""`, `[]`, `0`) fall back to the empty mapping just
    // like missing.
    //
    // DIVERGENCE (documented, caller-safe): a *truthy* non-dict `data`
    // (e.g. the string `"x"`) makes upstream raise `AttributeError`
    // (`.get` on `str`); this port falls back to empty (same "drop the
    // event" outcome at the adapter's `try/except` boundary, adapter lines
    // 601-605). Same for a truthy non-dict `resolved`.
    let data_raw = match raw.get("data") {
        Some(Value::Object(_)) => raw.get("data").unwrap(),
        _ => &empty,
    };
    // Upstream: `data_raw.get("resolved") or {}` — same falsy fallback.
    let resolved = match data_raw.get("resolved") {
        Some(Value::Object(_)) => data_raw.get("resolved").unwrap(),
        _ => &empty,
    };
    let get_str =
        |obj: &Value, key: &str| -> String { obj.get(key).map(py_str).unwrap_or_default() };
    let scene_code = py_int(raw.get("chat_type"));
    let scene = match scene_code {
        0 => "guild",
        1 => "group",
        2 => "c2c",
        _ => "",
    };
    InteractionEvent {
        id: get_str(raw, "id"),
        kind: py_int(data_raw.get("type")),
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

// ── ApprovalSender ───────────────────────────────────────────────────

/// Async post callable for [`ApprovalSender`]: `(chat_id, text, reply_to,
/// keyboard)` — mirrors the adapter's `_send_message_with_keyboard` helper
/// the upstream constructor takes as `post_c2c` / `post_group`.
///
/// PARITY: `PostMessageFn` (upstream line 207).
pub type PostMessageFn = dyn Fn(String, String, Option<String>, InlineKeyboard) -> Pin<Box<dyn Future<Output = ()> + Send>>
    + Send
    + Sync;

/// Send an approval-request message with an inline keyboard.
///
/// Decoupled from the adapter via callables so it can be unit-tested in
/// isolation. Pass the adapter's `_send_message_with_keyboard` helper
/// (or any equivalent) as `post_c2c` / `post_group`.
///
/// PARITY: `ApprovalSender` (upstream lines 209-271).
pub struct ApprovalSender {
    post_c2c: Box<PostMessageFn>,
    post_group: Box<PostMessageFn>,
    log_tag: String,
}

impl ApprovalSender {
    /// PARITY: `ApprovalSender.__init__` (upstream lines 217-225);
    /// `log_tag` defaults to `"QQBot"` (upstream line 221).
    pub fn new(
        post_c2c: impl Fn(
                String,
                String,
                Option<String>,
                InlineKeyboard,
            ) -> Pin<Box<dyn Future<Output = ()> + Send>>
            + Send
            + Sync
            + 'static,
        post_group: impl Fn(
                String,
                String,
                Option<String>,
                InlineKeyboard,
            ) -> Pin<Box<dyn Future<Output = ()> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self::with_log_tag(post_c2c, post_group, "QQBot")
    }

    /// Constructor with an explicit log tag (upstream `log_tag="QQBot"`).
    pub fn with_log_tag(
        post_c2c: impl Fn(
                String,
                String,
                Option<String>,
                InlineKeyboard,
            ) -> Pin<Box<dyn Future<Output = ()> + Send>>
            + Send
            + Sync
            + 'static,
        post_group: impl Fn(
                String,
                String,
                Option<String>,
                InlineKeyboard,
            ) -> Pin<Box<dyn Future<Output = ()> + Send>>
            + Send
            + Sync
            + 'static,
        log_tag: &str,
    ) -> Self {
        Self {
            post_c2c: Box::new(post_c2c),
            post_group: Box::new(post_group),
            log_tag: log_tag.to_string(),
        }
    }

    /// Log tag carried from construction (upstream `self._log_tag`).
    pub fn log_tag(&self) -> &str {
        &self.log_tag
    }

    /// Send an approval message to `chat_id`.
    ///
    /// `chat_type`: `"c2c"` or `"group"`. `reply_to`: reply-to message id
    /// (required for passive messages). Returns `true` on success, `false`
    /// on failure.
    ///
    /// PARITY: `ApprovalSender.send` (upstream lines 227-271) — the text
    /// and keyboard are built from `req` identically, the `c2c`/`group`
    /// dispatch and the fail-closed unknown-`chat_type` arm match, and a
    /// throwing post callable returns `false` (upstream `try/except`).
    /// Note: upstream threads `req.allow_permanent` into
    /// `build_approval_keyboard` only via the adapter's
    /// `send_approval_request` path (adapter line 1466), not here — `send`
    /// uses the default keyboard with the permanent button present.
    pub fn send(
        &self,
        chat_type: &str,
        chat_id: &str,
        req: &ApprovalRequest,
        reply_to: Option<&str>,
    ) -> bool {
        let text = build_approval_text(req);
        let keyboard = build_approval_keyboard(&req.session_key, req.allow_permanent);

        log::info!(
            "[{}] Sending approval request to {}:{} (session={:.20}…)",
            self.log_tag,
            chat_type,
            chat_id,
            req.session_key,
        );

        let post = match chat_type {
            "c2c" => &self.post_c2c,
            "group" => &self.post_group,
            _ => {
                log::warn!(
                    "[{}] Approval: unsupported chat_type {:?}",
                    self.log_tag,
                    chat_type,
                );
                return false;
            }
        };
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            block_on(post(
                chat_id.to_string(),
                text,
                reply_to.map(str::to_string),
                keyboard,
            ))
        }));
        match outcome {
            Ok(()) => {
                log::info!(
                    "[{}] Approval message sent to {}:{}",
                    self.log_tag,
                    chat_type,
                    chat_id,
                );
                true
            }
            Err(_) => {
                log::error!(
                    "[{}] Failed to send approval message to {}:{}",
                    self.log_tag,
                    chat_type,
                    chat_id,
                );
                false
            }
        }
    }
}

/// Drive a `Send`-bound future to completion on the calling thread with a
/// no-op waker (same minimal-executor pattern as
/// `hermes-tools::slash_confirm::resolve`; panics inside the future
/// propagate to the caller so `ApprovalSender::send` can map them to
/// `false`).
fn block_on<F>(mut future: F)
where
    F: Future + Send,
    F::Output: Send,
{
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    fn raw_waker() -> RawWaker {
        fn no_op(_: *const ()) {}
        fn clone(_: *const ()) -> RawWaker {
            raw_waker()
        }
        let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
        RawWaker::new(std::ptr::null(), vtable)
    }

    // SAFETY: the waker is a no-op that never dereferences its null data
    // pointer; it only signals "not ready — poll again".
    let waker = unsafe { Waker::from_raw(raw_waker()) };
    let mut cx = Context::from_waker(&waker);
    // SAFETY: `future` is a stack-local we never move after pinning.
    let mut future = unsafe { Pin::new_unchecked(&mut future) };
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(_) => return,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
