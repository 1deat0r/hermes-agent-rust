//! Conversation projection surface — OpenAI-format history loading for
//! resume/replay, lineage walks, and rewind restore.
//!
//! Helper subroutines mirror agent/memory_manager.py (sanitize_context),
//! agent/agent_runtime_helpers.py (repair_message_sequence,
//! drop_stale_api_content), and hermes_state.py's module-level
//! sandboxing/strip helpers.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   resolve_resume_session_id              (7481–7568)
//!   get_messages_as_conversation           (7570–7634)
//!   _rows_to_conversation                  (7636–7767)
//!   get_resume_conversations               (7769–7818)
//!   get_ancestor_display_prefix            (7820–7859)
//!   get_conversation_root                  (7861–7873)
//!   _session_lineage_root_to_tip           (7875–7896)
//!   _is_duplicate_replayed_user_message    (7898–7913)
//!   restore_rewound                        (8002–8015)

use std::collections::HashSet;

use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::Connection;
use serde_json::{json, Map, Value};

use crate::crud::CONTENT_JSON_PREFIX;
use crate::state::{SessionDB, WriteError};

// ── sanitize_context (agent/memory_manager.py) ─────────────────────────────

static INTERNAL_CONTEXT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)<\s*memory-context\s*>[\s\S]*?</\s*memory-context\s*>")
        .expect("context re")
});
static INTERNAL_NOTE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?is)\[System note:\s*The following is recalled memory context,\s*NOT new user input\.\s*Treat as (?:informational background data|authoritative reference data[^\]]*)\.\]\s*",
    )
    .expect("note re")
});
static FENCE_TAG_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)</?\s*memory-context\s*>").expect("fence re"));

/// Strip fence tags, injected context blocks, and system notes from
/// provider output.
pub fn sanitize_context(text: &str) -> String {
    let text = INTERNAL_CONTEXT_RE.replace_all(text, "");
    let text = INTERNAL_NOTE_RE.replace_all(&text, "");
    let text = FENCE_TAG_RE.replace_all(&text, "");
    text.into_owned()
}

// ── background-review harness strip (hermes_state.py module level) ─────────

const REVIEW_HARNESS_PREFIXES: [&str; 2] = [
    "Review the conversation above and update the skill library",
    "Review the conversation above and consider saving to memory",
];

fn is_background_review_harness_message(msg: &Value) -> bool {
    if !matches!(msg.get("role").and_then(Value::as_str), Some("user" | "system")) {
        return false;
    }
    let Some(content) = msg.get("content").and_then(Value::as_str) else {
        return false;
    };
    let head = content.trim_start();
    REVIEW_HARNESS_PREFIXES.iter().any(|p| head.starts_with(p))
}

fn strip_background_review_harness(messages: Vec<Value>) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::with_capacity(messages.len());
    let mut skip_next_assistant = false;
    for msg in messages {
        if is_background_review_harness_message(&msg) {
            skip_next_assistant = true;
            continue;
        }
        if skip_next_assistant {
            skip_next_assistant = false;
            if msg.get("role").and_then(Value::as_str) == Some("assistant") {
                continue;
            }
        }
        out.push(msg);
    }
    out
}

// ── stale tool-call marker strip (#78148) ──────────────────────────────────

static STALE_TOOL_CALL_MARKER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\[[A-Za-z_][A-Za-z0-9_.-]*\]$").expect("stale marker re")
});

fn is_stale_tool_call_marker_message(msg: &Value) -> bool {
    if msg.get("role").and_then(Value::as_str) != Some("assistant") {
        return false;
    }
    if !msg.get("tool_calls").is_some() || msg.get("tool_calls") == Some(&Value::Null) {
        return false;
    }
    let Some(content) = msg.get("content").and_then(Value::as_str) else {
        return false;
    };
    STALE_TOOL_CALL_MARKER_RE.is_match(content.trim())
}

fn strip_stale_tool_call_markers(messages: Vec<Value>) -> Vec<Value> {
    let mut repaired = 0;
    let out: Vec<Value> = messages
        .into_iter()
        .map(|mut m| {
            if is_stale_tool_call_marker_message(&m) {
                if let Some(obj) = m.as_object_mut() {
                    obj.insert("content".to_string(), Value::String(String::new()));
                }
                repaired += 1;
            }
            m
        })
        .collect();
    let _ = repaired;
    out
}

// ── repair_message_sequence (agent/agent_runtime_helpers.py) ───────────────

fn is_codex_interim(m: &Value) -> bool {
    m.get("codex_reasoning_items").is_some_and(|v| !v.is_null())
        || m.get("codex_message_items").is_some_and(|v| !v.is_null())
        || m.get("finish_reason").and_then(Value::as_str) == Some("incomplete")
}

fn is_verification_candidate(m: &Value) -> bool {
    matches!(
        m.get("finish_reason").and_then(Value::as_str),
        Some("verification_required" | "verify_hook_continue")
    )
}

fn drop_stale_api_content(msg: &mut Value) {
    if let Some(obj) = msg.as_object_mut() {
        obj.remove("api_content");
    }
}

/// Collapse malformed role-alternation left in the live history. Mutates
/// `messages` in place and returns the number of repairs made (for
/// logging). Pass 0 merges consecutive assistant turns; pass 1 drops stray
/// tool results; pass 2 merges consecutive user turns.
///
/// PARITY: agent.agent_runtime_helpers.repair_message_sequence @ b9aa928
/// (550–748)
pub fn repair_message_sequence(messages: &mut Vec<Value>) -> i64 {
    if messages.is_empty() {
        return 0;
    }
    let mut repairs: i64 = 0;

    // Pass 0: merge consecutive assistant messages.
    let mut collapsed: Vec<Value> = Vec::with_capacity(messages.len());
    for msg in messages.drain(..) {
        let adjacent_assistant = !collapsed.is_empty()
            && msg.get("role").and_then(Value::as_str) == Some("assistant")
            && collapsed.last().is_some_and(|p| {
                p.get("role").and_then(Value::as_str) == Some("assistant")
            })
            && !is_codex_interim(&msg)
            && !collapsed.last().is_some_and(is_codex_interim);
        if adjacent_assistant {
            let mut prev = collapsed.pop().expect("non-empty");
            if is_verification_candidate(&prev) {
                // Verification candidate: later response supersedes it.
                collapsed.push(msg);
                repairs += 1;
                continue;
            }
            // Union tool_calls (preserve order, both may carry them).
            let prev_calls: Vec<Value> = prev
                .get("tool_calls")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let new_calls: Vec<Value> = msg
                .get("tool_calls")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let merged_calls: Vec<Value> = if !new_calls.is_empty() {
                prev_calls
                    .into_iter()
                    .chain(new_calls)
                    .collect()
            } else if !prev_calls.is_empty() {
                prev_calls
            } else {
                Vec::new()
            };
            if msg.get("tool_calls").is_some() && !merged_calls.is_empty() {
                prev
                    .as_object_mut()
                    .unwrap()
                    .insert("tool_calls".to_string(), Value::Array(merged_calls));
            }
            // Concatenate plain-text content; leave multimodal list content
            // alone to avoid mangling attachment blocks.
            let prev_content = prev.get("content").cloned();
            let new_content = msg.get("content").cloned();
            match (prev_content, new_content) {
                (Some(Value::String(a)), Some(Value::String(b))) => {
                    let joined = [
                        a.trim().to_string(),
                        b.trim().to_string(),
                    ]
                    .into_iter()
                    .filter(|p| !p.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                    prev.as_object_mut().unwrap().insert("content".to_string(), Value::String(joined));
                }
                (None, Some(newc)) => {
                    prev.as_object_mut().unwrap().insert("content".to_string(), newc);
                }
                _ => {}
            }
            // Carry reasoning_content from the later turn only if the
            // earlier turn lacks it.
            if !prev.get("reasoning_content").is_some_and(|v| !v.is_null())
                && msg.get("reasoning_content").is_some()
            {
                prev.as_object_mut().unwrap().insert(
                    "reasoning_content".to_string(),
                    msg.get("reasoning_content").cloned().unwrap_or(Value::Null),
                );
            }
            repairs += 1;
            collapsed.push(prev);
            continue;
        }
        collapsed.push(msg);
    }

    // Pass 1: drop stray tool messages that don't follow a known assistant
    // tool_call_id. Rolling set refreshed on each assistant message.
    let mut known_tool_ids: HashSet<String> = HashSet::new();
    let mut filtered: Vec<Value> = Vec::with_capacity(collapsed.len());
    for msg in collapsed {
        let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
        if role == "assistant" {
            known_tool_ids.clear();
            if let Some(tc) = msg.get("tool_calls").and_then(Value::as_array) {
                for call in tc {
                    if let Some(obj) = call.as_object() {
                        for key in ["id", "call_id"] {
                            if let Some(id) = obj.get(key).and_then(Value::as_str) {
                                if !id.is_empty() {
                                    known_tool_ids.insert(id.to_string());
                                }
                            }
                        }
                    }
                }
            }
            filtered.push(msg);
        } else if role == "tool" {
            let tc_id = msg.get("tool_call_id").and_then(Value::as_str).unwrap_or("");
            if !tc_id.is_empty() && known_tool_ids.contains(tc_id) {
                known_tool_ids.remove(tc_id);
                filtered.push(msg);
            } else {
                repairs += 1;
            }
        } else {
            if role == "user" {
                known_tool_ids.clear();
            }
            filtered.push(msg);
        }
    }

    // Pass 2: merge consecutive user messages (plain-text only).
    let mut merged: Vec<Value> = Vec::with_capacity(filtered.len());
    for msg in filtered {
        let adjacent_user = !merged.is_empty()
            && msg.get("role").and_then(Value::as_str) == Some("user")
            && merged.last().is_some_and(|p| {
                p.get("role").and_then(Value::as_str) == Some("user")
            });
        if adjacent_user {
            let mut prev = merged.pop().expect("non-empty");
            let prev_content = prev.get("content").cloned();
            let new_content = msg.get("content").cloned();
            if let (Some(Value::String(a)), Some(Value::String(b))) = (prev_content, new_content) {
                let combined = if !a.is_empty() && !b.is_empty() {
                    format!("{a}\n\n{b}")
                } else if !a.is_empty() {
                    a
                } else {
                    b
                };
                prev
                    .as_object_mut()
                    .unwrap()
                    .insert("content".to_string(), Value::String(combined));
                drop_stale_api_content(&mut prev);
                repairs += 1;
                merged.push(prev);
                continue;
            }
            merged.push(prev);
            merged.push(msg);
            continue;
        }
        merged.push(msg);
    }

    // Rewrite in place so downstream paths see the repaired sequence.
    *messages = merged;
    repairs
}

// ── conversation row decoding ──────────────────────────────────────────────

const CONVERSATION_ROW_COLUMNS: &str = "id, role, content, tool_call_id, tool_calls, \
     tool_name, effect_disposition, finish_reason, reasoning, reasoning_content, \
     reasoning_details, codex_reasoning_items, codex_message_items, \
     platform_message_id, observed, timestamp, api_content, display_kind, display_metadata";

fn parse_json_field(row: &Value, key: &str) -> Option<Value> {
    match row.get(key) {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => serde_json::from_str::<Value>(s).ok(),
        Some(v) => Some(v.clone()),
    }
}

impl SessionDB {
    /// Walk `parent_session_id` forward from `session_id`, returning the
    /// descendant in the chain that holds the most recent messages.
    ///
    /// PARITY: SessionDB.resolve_resume_session_id @ b9aa928 (7481–7568)
    pub fn resolve_resume_session_id(&self, session_id: &str) -> Result<String, WriteError> {
        if session_id.is_empty() {
            return Ok(session_id.to_string());
        }
        // Follow the compression-continuation chain forward to the live tip
        // FIRST (lineage-aware, so delegation/branch children never hijack).
        let session_id = self.get_compression_tip(session_id)?;

        let mut current = session_id.clone();
        let mut seen: HashSet<String> = [current.clone()].into_iter().collect();
        let mut best: Option<String> = None;

        for _ in 0..32 {
            let conn = self.writer_conn();
            let has_msgs: Option<i64> = conn
                .query_row(
                    "SELECT 1 FROM messages WHERE session_id = ? LIMIT 1",
                    rusqlite::params![current],
                    |r| r.get(0),
                )
                .optional_err()
                .map_err(WriteError::Sqlite)?;
            if has_msgs.is_some() {
                best = Some(current.clone());
            }
            let child: Option<String> = conn
                .query_row(
                    "SELECT id FROM sessions \
                     WHERE parent_session_id = ? \
                       AND json_extract(COALESCE(model_config, '{}'), '$._branched_from') IS NULL \
                       AND json_extract(COALESCE(model_config, '{}'), '$._delegate_from') IS NULL \
                       AND COALESCE(source, '') != 'tool' \
                     ORDER BY started_at DESC, id DESC LIMIT 1",
                    rusqlite::params![current],
                    |r| r.get(0),
                )
                .optional_err()
                .map_err(WriteError::Sqlite)?;
            drop(conn);
            let Some(child_id) = child else {
                break;
            };
            if child_id.is_empty() || seen.contains(&child_id) {
                break;
            }
            seen.insert(child_id.clone());
            current = child_id;
        }
        Ok(best.unwrap_or(session_id))
    }

    /// The chain from root to `session_id` via `parent_session_id` links.
    /// Returns `[session_id]` when it has no recorded ancestry.
    ///
    /// PARITY: SessionDB._session_lineage_root_to_tip @ b9aa928 (7875–7896)
    pub fn session_lineage_root_to_tip(&self, session_id: &str) -> Result<Vec<String>, WriteError> {
        if session_id.is_empty() {
            return Ok(vec![session_id.to_string()]);
        }
        let mut chain: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut current = session_id.to_string();
        for _ in 0..100 {
            if current.is_empty() || seen.contains(&current) {
                break;
            }
            seen.insert(current.clone());
            chain.push(current.clone());
            let conn = self.writer_conn();
            let parent: Option<Option<String>> = conn
                .query_row(
                    "SELECT parent_session_id FROM sessions WHERE id = ?",
                    rusqlite::params![current],
                    |r| r.get(0),
                )
                .optional_err()
                .map_err(WriteError::Sqlite)?;
            drop(conn);
            let Some(parent) = parent else {
                break;
            };
            match parent {
                Some(p) => current = p,
                None => break,
            }
        }
        chain.reverse();
        if chain.is_empty() {
            Ok(vec![session_id.to_string()])
        } else {
            Ok(chain)
        }
    }

    /// Decode fetched message rows into the OpenAI conversation format.
    ///
    /// PARITY: SessionDB._rows_to_conversation @ b9aa928 (7636–7767)
    pub(crate) fn rows_to_conversation(
        rows: Vec<Value>,
        _session_id: &str,
        include_ancestors: bool,
        repair_alternation: bool,
        include_row_ids: bool,
    ) -> Vec<Value> {
        let mut messages: Vec<Value> = Vec::with_capacity(rows.len());
        for row in rows {
            let mut content = crate::conversation::decode_content_json(row.get("content"));
            if matches!(row.get("role").and_then(Value::as_str), Some("user" | "assistant")) {
                if let Some(Value::String(s)) = content {
                    content = Some(Value::String(sanitize_context(&s).trim().to_string()));
                }
            }
            let mut msg = Map::new();
            msg.insert("role".to_string(), row.get("role").cloned().unwrap_or(Value::Null));
            msg.insert("content".to_string(), content.unwrap_or(Value::Null));
            if include_row_ids {
                if let Some(id) = row.get("id").and_then(Value::as_i64) {
                    msg.insert("_row_id".to_string(), json!(id));
                }
            }
            // api_content: byte-fidelity sidecar returned VERBATIM.
            if let Some(Value::String(s)) = row.get("api_content") {
                if !s.is_empty() {
                    msg.insert("api_content".to_string(), Value::String(s.clone()));
                }
            }
            if let Some(k) = row.get("display_kind").and_then(Value::as_str) {
                if !k.is_empty() {
                    msg.insert("display_kind".to_string(), Value::String(k.to_string()));
                }
            }
            if let Some(raw) = row.get("display_metadata") {
                let raw_s = raw.as_str().map(|s| s.to_string());
                if let Some(raw_s) = raw_s {
                    if !raw_s.is_empty() {
                        if let Some(decoded) = crate::crud::decode_display_metadata(Some(&raw_s)) {
                            msg.insert("display_metadata".to_string(), decoded);
                        }
                    }
                }
            }
            if let Some(ts) = row.get("timestamp").and_then(Value::as_f64) {
                if ts != 0.0 {
                    msg.insert("timestamp".to_string(), json!(ts));
                }
            }
            if let Some(v) = row.get("tool_call_id") {
                if v.as_str().is_some_and(|s| !s.is_empty()) {
                    msg.insert("tool_call_id".to_string(), v.clone());
                }
            }
            if let Some(v) = row.get("tool_name") {
                if v.as_str().is_some_and(|s| !s.is_empty()) {
                    msg.insert("tool_name".to_string(), v.clone());
                }
            }
            if let Some(v) = row.get("effect_disposition") {
                if v.as_str().is_some_and(|s| !s.is_empty()) {
                    msg.insert("effect_disposition".to_string(), v.clone());
                }
            }
            if let Some(tc) = parse_json_field(&row, "tool_calls") {
                msg.insert("tool_calls".to_string(), tc);
            }
            if let Some(pmid) = row.get("platform_message_id").and_then(Value::as_str) {
                if !pmid.is_empty() {
                    msg.insert("message_id".to_string(), Value::String(pmid.to_string()));
                }
            }
            if row.get("observed").and_then(Value::as_bool).unwrap_or(false)
                || row.get("observed").and_then(Value::as_i64) == Some(1)
            {
                msg.insert("observed".to_string(), Value::Bool(true));
            }
            if row.get("role").and_then(Value::as_str) == Some("assistant") {
                for key in ["finish_reason", "reasoning", "reasoning_content"] {
                    if let Some(v) = row.get(key) {
                        let keep = match v {
                            Value::String(s) => !s.is_empty(),
                            Value::Null => false,
                            _ => true,
                        };
                        if keep {
                            msg.insert(key.to_string(), v.clone());
                        }
                    }
                }
                for key in ["reasoning_details", "codex_reasoning_items", "codex_message_items"] {
                    if let Some(parsed) = parse_json_field(&row, key) {
                        msg.insert(key.to_string(), parsed);
                    }
                }
            }
            let msg = Value::Object(msg);
            if include_ancestors && duplicate_replayed_user_message(&messages, &msg) {
                continue;
            }
            messages.push(msg);
        }
        // Defense-in-depth: strip polluting harness turns and stale markers.
        messages = strip_background_review_harness(messages);
        messages = strip_stale_tool_call_markers(messages);
        if repair_alternation && !messages.is_empty() {
            repair_message_sequence(&mut messages);
        }
        messages
    }

    /// Load messages in OpenAI conversation format (role + content dicts).
    ///
    /// PARITY: SessionDB.get_messages_as_conversation @ b9aa928 (7570–7634)
    pub fn get_messages_as_conversation(
        &self,
        session_id: &str,
        include_ancestors: bool,
        include_inactive: bool,
        repair_alternation: bool,
        include_row_ids: bool,
    ) -> Result<Vec<Value>, WriteError> {
        if session_id.is_empty() {
            return Ok(Vec::new());
        }
        let session_ids = if include_ancestors {
            self.session_lineage_root_to_tip(session_id)?
        } else {
            vec![session_id.to_string()]
        };
        let active_clause = if include_inactive { "" } else { " AND active = 1" };
        let placeholders = vec!["?"; session_ids.len()].join(",");
        let sql = format!(
            "SELECT {CONVERSATION_ROW_COLUMNS} \
             FROM messages WHERE session_id IN ({placeholders}){active_clause} \
             ORDER BY id"
        );
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(session_ids.iter()), crate::portability::row_to_value)
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(Self::rows_to_conversation(
            rows,
            session_id,
            include_ancestors,
            repair_alternation,
            include_row_ids,
        ))
    }

    /// Return `(model_history, display_history)` for a session resume in
    /// one lineage SELECT.
    ///
    /// PARITY: SessionDB.get_resume_conversations @ b9aa928 (7769–7818)
    pub fn get_resume_conversations(
        &self,
        session_id: &str,
    ) -> Result<(Vec<Value>, Vec<Value>), WriteError> {
        if session_id.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let session_ids = self.session_lineage_root_to_tip(session_id)?;
        let placeholders = vec!["?"; session_ids.len()].join(",");
        let sql = format!(
            "SELECT session_id, {CONVERSATION_ROW_COLUMNS} \
             FROM messages WHERE session_id IN ({placeholders}) AND active = 1 \
             ORDER BY id"
        );
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(session_ids.iter()), crate::portability::row_to_value)
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        let tip_rows: Vec<Value> = rows
            .iter()
            .filter(|r| r.get("session_id").and_then(Value::as_str) == Some(session_id))
            .cloned()
            .collect();
        let model_history = Self::rows_to_conversation(
            tip_rows,
            session_id,
            false,
            true,
            true,
        );
        let display_history = Self::rows_to_conversation(rows, session_id, true, false, true);
        Ok((model_history, display_history))
    }

    /// Return ancestor-only display messages for a session lineage.
    ///
    /// PARITY: SessionDB.get_ancestor_display_prefix @ b9aa928 (7820–7859)
    pub fn get_ancestor_display_prefix(
        &self,
        session_id: &str,
    ) -> Result<Vec<Value>, WriteError> {
        if session_id.is_empty() {
            return Ok(Vec::new());
        }
        let session_ids = self.session_lineage_root_to_tip(session_id)?;
        if session_ids.len() <= 1 {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; session_ids.len()].join(",");
        let sql = format!(
            "SELECT session_id, {CONVERSATION_ROW_COLUMNS} \
             FROM messages WHERE session_id IN ({placeholders}) AND active = 1 \
             ORDER BY id"
        );
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(session_ids.iter()), crate::portability::row_to_value)
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        let ancestor_rows: Vec<Value> = rows
            .into_iter()
            .filter(|r| r.get("session_id").and_then(Value::as_str) != Some(session_id))
            .collect();
        if ancestor_rows.is_empty() {
            return Ok(Vec::new());
        }
        Ok(Self::rows_to_conversation(ancestor_rows, session_id, true, false, false))
    }

    /// Return the ROOT id of `session_id`'s lineage chain (stable
    /// conversation id), or the input when it has no parent.
    ///
    /// PARITY: SessionDB.get_conversation_root @ b9aa928 (7861–7873)
    pub fn get_conversation_root(&self, session_id: &str) -> Result<String, WriteError> {
        let chain = self.session_lineage_root_to_tip(session_id)?;
        Ok(chain
            .first()
            .cloned()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| session_id.to_string()))
    }

    /// Mark inactive messages with id >= `since_message_id` active again.
    /// Returns the number of rows flipped back to `active=1`.
    ///
    /// PARITY: SessionDB.restore_rewound @ b9aa928 (8002–8015)
    pub fn restore_rewound(&self, session_id: &str, since_message_id: i64) -> Result<i64, WriteError> {
        let sid = session_id.to_string();
        let f = |conn: &Connection| -> Result<i64, WriteError> {
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM messages \
                     WHERE session_id = ? AND id >= ? AND active = 0",
                )
                .map_err(WriteError::Sqlite)?;
            let ids: Vec<i64> = stmt
                .query_map(rusqlite::params![sid, since_message_id], |r| r.get(0))
                .map_err(WriteError::Sqlite)?
                .collect::<Result<_, _>>()
                .map_err(WriteError::Sqlite)?;
            if ids.is_empty() {
                return Ok(0);
            }
            let placeholders = vec!["?"; ids.len()].join(",");
            conn.execute(
                &format!("UPDATE messages SET active = 1 WHERE id IN ({placeholders})"),
                rusqlite::params_from_iter(ids.iter().copied()),
            )?;
            Ok(ids.len() as i64)
        };
        self.execute_write(&f, None)
    }
}

/// Decode a JSON-object message `content` column like
/// `crud::decode_content` does for rusqlite values.
fn decode_content_json(raw: Option<&Value>) -> Option<Value> {
    match raw {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.starts_with(CONTENT_JSON_PREFIX) => {
            match serde_json::from_str::<Value>(&s[CONTENT_JSON_PREFIX.len()..]) {
                Ok(v) => Some(v),
                Err(_) => Some(Value::String(s.clone())),
            }
        }
        Some(Value::String(s)) => Some(Value::String(s.clone())),
        Some(Value::Number(n)) => Some(Value::Number(n.clone())),
        Some(Value::Bool(b)) => Some(Value::Bool(*b)),
        Some(v) => Some(v.clone()),
    }
}

/// True when `msg` is a user message whose text exactly duplicates a prior
/// user message still adjacent within the conversation (compression-fork
/// replay dedup).
///
/// PARITY: SessionDB._is_duplicate_replayed_user_message @ b9aa928
/// (7898–7913)
fn duplicate_replayed_user_message(messages: &[Value], msg: &Value) -> bool {
    if msg.get("role").and_then(Value::as_str) != Some("user") {
        return false;
    }
    let Some(content) = msg.get("content").and_then(Value::as_str) else {
        return false;
    };
    if content.is_empty() {
        return false;
    }
    for prev in messages.iter().rev() {
        if prev.get("role").and_then(Value::as_str) == Some("user")
            && prev.get("content").and_then(Value::as_str) == Some(content)
        {
            return true;
        }
        if prev.get("role").and_then(Value::as_str) == Some("assistant")
            && (prev.get("content").is_some_and(|v| !v.is_null())
                || prev.get("tool_calls").is_some())
        {
            return false;
        }
    }
    false
}

trait OptionalErr<T> {
    fn optional_err(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalErr<T> for Result<T, rusqlite::Error> {
    fn optional_err(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
