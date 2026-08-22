//! Message rewrite / rewind / in-place-compaction surfaces.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   _merge_model_config_json        (4672–4723)
//!   replace_messages                (7170–7243)
//!   has_archived_messages           (7243–7255)
//!   archive_and_compact             (7255–7330)
//!   rewind_to_message               (7915–7996)

use rusqlite::{Connection, OptionalExtension, Row};
use serde_json::{json, Value};


use super::crud::MessageInput;
use super::state::{SessionDB, WriteError};

/// SELECT + tolerant-parse + merge `patch` into a session's model_config.
///
/// PARITY: hermes_state.py SessionDB._merge_model_config_json @ b9aa928.
/// Runs inside an open write transaction (callers own the UPDATE). Returns
/// the serialized merged JSON — None when the merged dict is empty
/// (matching create_session's NULL convention). `on_missing_raise` mirrors
/// upstream's `on_missing="raise"`: ValueError when the session row is gone.
pub(crate) fn merge_model_config_json(
    conn: &Connection,
    session_id: &str,
    patch: &serde_json::Map<String, Value>,
    on_missing_raise: bool,
) -> Result<Option<String>, WriteError> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT model_config FROM sessions WHERE id = ?",
            rusqlite::params![session_id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(raw) = raw else {
        if on_missing_raise {
            return Err(WriteError::ValueError(format!(
                "Session not found: {}",
                session_id
            )));
        }
        return Ok(None);
    };
    let mut config: serde_json::Map<String, Value> = serde_json::Map::new();
    if !raw.trim().is_empty() {
        if let Ok(Value::Object(parsed)) = serde_json::from_str::<Value>(&raw) {
            config = parsed;
        }
    }
    for (key, value) in patch {
        if value.is_null() {
            config.remove(key);
        } else {
            config.insert(key.clone(), value.clone());
        }
    }
    if config.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Object(config).to_string()))
    }
}

impl SessionDB {
    /// Atomically replace the stored messages for a session.
    ///
    /// PARITY: hermes_state.py SessionDB.replace_messages @ b9aa928. The
    /// delete + reinsert commit as one transaction. DESTRUCTIVE by default;
    /// `active_only=true` replaces ONLY live rows (the #80216-safe path used
    /// by transcript-rewrite flows sharing a session with in-place
    /// compaction).
    pub fn replace_messages(
        &self,
        session_id: &str,
        messages: &[MessageInput],
        active_only: bool,
    ) -> Result<(), WriteError> {
        let active_clause = if active_only { " AND active = 1" } else { "" };
        let sql = format!(
            "DELETE FROM messages WHERE session_id = ?{}",
            active_clause
        );
        let session_id = session_id.to_string();
        let messages = messages.to_vec();
        let f = move |conn: &Connection| -> Result<(), WriteError> {
            let session = conn
                .query_row(
                    "SELECT ended_at, end_reason FROM sessions WHERE id = ?",
                    rusqlite::params![session_id],
                    |r| {
                        Ok((
                            r.get::<_, Option<f64>>(0)?,
                            r.get::<_, Option<String>>(1)?,
                        ))
                    },
                )
                .optional()?;
            if let Some((ended_at, end_reason)) = session {
                if ended_at.is_some() && end_reason.as_deref() == Some("compression") {
                    return Err(WriteError::CompressionSessionClosed(session_id.clone()));
                }
            }
            conn.execute(&sql, rusqlite::params![session_id])?;
            conn.execute(
                "UPDATE sessions SET message_count = 0, tool_call_count = 0 WHERE id = ?",
                rusqlite::params![session_id],
            )?;
            let (total_messages, total_tool_calls) =
                SessionDB::insert_message_rows(conn, &session_id, &messages)?;
            conn.execute(
                "UPDATE sessions SET message_count = ?, tool_call_count = ? WHERE id = ?",
                rusqlite::params![total_messages, total_tool_calls, session_id],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// True if the session has any soft-archived (`active = 0`) rows.
    ///
    /// PARITY: hermes_state.py SessionDB.has_archived_messages @ b9aa928.
    /// Cheap existence probe; kept for tests/diagnostics (production rewrite
    /// paths pass `active_only=True` unconditionally rather than probing).
    pub fn has_archived_messages(&self, session_id: &str) -> Result<bool, WriteError> {
        let conn = self.writer_conn();
        let found: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM messages WHERE session_id = ? AND active = 0 LIMIT 1",
                rusqlite::params![session_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(found.is_some())
    }

    /// Non-destructive in-place compaction for a single durable session id.
    ///
    /// PARITY: hermes_state.py SessionDB.archive_and_compact @ b9aa928.
    /// Soft-archives every currently-active message (active=0, compacted=1)
    /// and inserts `compacted_messages` as fresh active rows — atomically.
    /// `model_config_patch` is merged into the session's JSON config in the
    /// same transaction (None removes that key; missing session raises
    /// ValueError). Returns the new active count.
    pub fn archive_and_compact(
        &self,
        session_id: &str,
        compacted_messages: &[MessageInput],
        model_config_patch: Option<&serde_json::Map<String, Value>>,
    ) -> Result<i64, WriteError> {
        let session_id = session_id.to_string();
        let compacted_messages = compacted_messages.to_vec();
        let model_config_patch = model_config_patch.cloned();
        let f = move |conn: &Connection| -> Result<i64, WriteError> {
            let patched_model_config = match &model_config_patch {
                Some(patch) => Some(merge_model_config_json(
                    conn,
                    &session_id,
                    patch,
                    true,
                )?),
                None => None,
            };
            // Soft-archive the live turns (active=0 hides them from the live
            // context; compacted=1 keeps them discoverable by search).
            conn.execute(
                "UPDATE messages SET active = 0, compacted = 1 \
                 WHERE session_id = ? AND active = 1",
                rusqlite::params![session_id],
            )?;
            let (inserted, tool_calls_total) =
                SessionDB::insert_message_rows(conn, &session_id, &compacted_messages)?;
            // message_count/tool_call_count reflect the LIVE (active) set.
            match patched_model_config {
                None => {
                    conn.execute(
                        "UPDATE sessions SET message_count = ?, tool_call_count = ? \
                         WHERE id = ?",
                        rusqlite::params![inserted, tool_calls_total, session_id],
                    )?;
                }
                Some(cfg) => {
                    conn.execute(
                        "UPDATE sessions SET message_count = ?, tool_call_count = ?, \
                         model_config = ? WHERE id = ?",
                        rusqlite::params![inserted, tool_calls_total, cfg, session_id],
                    )?;
                }
            }
            Ok(inserted as i64)
        };
        self.execute_write(&f, None)
    }

    /// Soft-delete all messages with id >= `target_message_id`.
    ///
    /// PARITY: hermes_state.py SessionDB.rewind_to_message @ b9aa928. The
    /// target itself becomes inactive (so callers can pre-fill it without it
    /// appearing twice). Returns rewound_count, the target row (content
    /// decoded for prompt prefill; other columns raw as stored), and the new
    /// head id. Raises ValueError when the target is missing or not a user
    /// message. Always bumps sessions.rewind_count.
    pub fn rewind_to_message(
        &self,
        session_id: &str,
        target_message_id: i64,
    ) -> Result<RewindOutcome, WriteError> {
        // 1) Validate target up-front (read-only, outside the write txn).
        let target_row = {
            let conn = self.writer_conn();
            let raw: Option<RawMessageRow> = conn
                .query_row(
                    "SELECT * FROM messages WHERE id = ? AND session_id = ?",
                    rusqlite::params![target_message_id, session_id],
                    RawMessageRow::from_row,
                )
                .optional()
                .map_err(WriteError::Sqlite)?;
            raw
        };
        let Some(target) = target_row else {
            return Err(WriteError::ValueError(format!(
                "message {} not found in session {}",
                target_message_id, session_id
            )));
        };
        if target.role != "user" {
            return Err(WriteError::ValueError(format!(
                "rewind target must be a 'user' message (got role={:?}, id={})",
                target.role, target_message_id
            )));
        }

        let sid_for_head = session_id.to_string();
        let session_id = session_id.to_string();
        let f = move |conn: &Connection| -> Result<Vec<i64>, WriteError> {
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM messages \
                     WHERE session_id = ? AND id >= ? AND active = 1",
                )
                .map_err(WriteError::Sqlite)?;
            let ids: Vec<i64> = stmt
                .query_map(
                    rusqlite::params![session_id, target_message_id],
                    |r| r.get(0),
                )
                .map_err(WriteError::Sqlite)?
                .collect::<Result<_, _>>()
                .map_err(WriteError::Sqlite)?;
            if !ids.is_empty() {
                let placeholders = vec!["?"; ids.len()].join(",");
                let sql = format!(
                    "UPDATE messages SET active = 0 WHERE id IN ({})",
                    placeholders
                );
                conn.execute(&sql, rusqlite::params_from_iter(ids.iter()))?;
            }
            conn.execute(
                "UPDATE sessions SET rewind_count = COALESCE(rewind_count, 0) + 1 \
                 WHERE id = ?",
                rusqlite::params![session_id],
            )?;
            Ok(ids)
        };
        let rewound = self.execute_write(&f, None)?;

        // 2) Compute new head id (largest still-active row id in session).
        let conn = self.writer_conn();
        let new_head_id: Option<i64> = conn
            .query_row(
                "SELECT MAX(id) FROM messages WHERE session_id = ? AND active = 1",
                rusqlite::params![sid_for_head],
                |r| r.get(0),
            )
            .optional()
            .map_err(WriteError::Sqlite)?
            .flatten();

        Ok(RewindOutcome {
            rewound_count: rewound.len(),
            target_message: target.to_value(),
            new_head_id,
        })
    }
}

/// Result of `rewind_to_message`, mirroring upstream's dict shape.
#[derive(Debug, Clone)]
pub struct RewindOutcome {
    pub rewound_count: usize,
    /// Full target row as raw dict — except `content`, which is decoded for
    /// prompt-prefill callers (upstream `dict(row)` + `_decode_content`).
    pub target_message: Value,
    pub new_head_id: Option<i64>,
}

/// Raw messages row as stored (id/ints kept as SQLite typed; JSON sidecars
/// left as stored strings) — mirrors `dict(row)` before decoding.
struct RawMessageRow {
    id: i64,
    session_id: String,
    role: String,
    content: Option<rusqlite::types::Value>,
    tool_call_id: Option<String>,
    tool_calls: Option<String>,
    tool_name: Option<String>,
    effect_disposition: Option<String>,
    timestamp: f64,
    token_count: Option<i64>,
    finish_reason: Option<String>,
    reasoning: Option<String>,
    reasoning_content: Option<String>,
    reasoning_details: Option<String>,
    codex_reasoning_items: Option<String>,
    codex_message_items: Option<String>,
    platform_message_id: Option<String>,
    observed: i64,
    active: i64,
    api_content: Option<String>,
    display_kind: Option<String>,
    display_metadata: Option<String>,
}

impl RawMessageRow {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(RawMessageRow {
            id: r.get("id")?,
            session_id: r.get("session_id")?,
            role: r.get("role")?,
            content: r.get("content")?,
            tool_call_id: r.get("tool_call_id")?,
            tool_calls: r.get("tool_calls")?,
            tool_name: r.get("tool_name")?,
            effect_disposition: r.get("effect_disposition")?,
            timestamp: r.get("timestamp")?,
            token_count: r.get("token_count")?,
            finish_reason: r.get("finish_reason")?,
            reasoning: r.get("reasoning")?,
            reasoning_content: r.get("reasoning_content")?,
            reasoning_details: r.get("reasoning_details")?,
            codex_reasoning_items: r.get("codex_reasoning_items")?,
            codex_message_items: r.get("codex_message_items")?,
            platform_message_id: r.get("platform_message_id")?,
            observed: r.get("observed")?,
            active: r.get("active")?,
            api_content: r.get("api_content")?,
            display_kind: r.get("display_kind")?,
            display_metadata: r.get("display_metadata")?,
        })
    }

    fn to_value(&self) -> Value {
        let content = super::crud::decode_content(self.content.clone());
        json!({
            "id": self.id,
            "session_id": self.session_id,
            "role": self.role,
            "content": content,
            "tool_call_id": self.tool_call_id,
            "tool_calls": self.tool_calls,
            "tool_name": self.tool_name,
            "effect_disposition": self.effect_disposition,
            "timestamp": self.timestamp,
            "token_count": self.token_count,
            "finish_reason": self.finish_reason,
            "reasoning": self.reasoning,
            "reasoning_content": self.reasoning_content,
            "reasoning_details": self.reasoning_details,
            "codex_reasoning_items": self.codex_reasoning_items,
            "codex_message_items": self.codex_message_items,
            "platform_message_id": self.platform_message_id,
            "observed": self.observed,
            "active": self.active,
            "api_content": self.api_content,
            "display_kind": self.display_kind,
            "display_metadata": self.display_metadata,
        })
    }
}
