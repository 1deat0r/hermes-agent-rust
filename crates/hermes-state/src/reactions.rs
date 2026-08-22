//! Message presentation + reaction surfaces.
//!
//! `display_kind` / `display_metadata` carry producer provenance without
//! classifying content; reactions live inside `display_metadata.reactions`
//! so they survive rewind/compaction row rewrites with the row itself.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   set_latest_matching_message_display_kind  (6831–6868)
//!   set_message_reaction                      (6870–6931)
//!   get_message_reactions                     (6933–6952)
//!   take_unseen_reactions                     (6954–7002)
//!   set_latest_user_api_content               (7317–7344)

use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Value};

use crate::crud::{
    decode_content, decode_display_metadata, encode_content, encode_display_metadata,
    scrub_surrogates,
};
use crate::state::{now, SessionDB, WriteError};

const REACTIONS_METADATA_KEY: &str = "reactions";

impl SessionDB {
    /// Stamp presentation metadata on this turn's freshly persisted row.
    ///
    /// PARITY: SessionDB.set_latest_matching_message_display_kind @ b9aa928
    /// (6831–6868)
    pub fn set_latest_matching_message_display_kind(
        &self,
        session_id: &str,
        role: &str,
        content: Option<&Value>,
        display_kind: &str,
        display_metadata: Option<&Value>,
    ) -> Result<bool, WriteError> {
        if session_id.is_empty() || content.is_none() || display_kind.is_empty() {
            return Ok(false);
        }
        let encoded = encode_content(content);
        let sid = session_id.to_string();
        let role = role.to_string();
        let display_kind = display_kind.to_string();
        let display_kind = scrub_surrogates(Some(display_kind)).expect("kind");
        let display_metadata = encode_display_metadata(display_metadata);

        let f = move |conn: &Connection| -> Result<bool, WriteError> {
            let row_id: Option<i64> = conn
                .query_row(
                    "SELECT id FROM messages WHERE session_id = ? AND role = ? \
                     AND content = ? AND active = 1 ORDER BY id DESC LIMIT 1",
                    rusqlite::params![sid, role, encoded, ],
                    |r| r.get(0),
                )
                .optional()?;
            let Some(row_id) = row_id else {
                return Ok(false);
            };
            conn.execute(
                "UPDATE messages SET display_kind = ?, display_metadata = ? WHERE id = ?",
                rusqlite::params![display_kind, display_metadata, row_id],
            )?;
            Ok(true)
        };
        self.execute_write(&f, None)
    }

    /// Set (or with `emoji=None` clear) `author`'s reaction on one message.
    /// Returns the message's full reaction list after the write, or None
    /// when the row doesn't exist or isn't part of `session_id`.
    ///
    /// PARITY: SessionDB.set_message_reaction @ b9aa928 (6870–6931)
    pub fn set_message_reaction(
        &self,
        session_id: &str,
        message_row_id: i64,
        emoji: Option<&str>,
        author: &str,
    ) -> Result<Option<Value>, WriteError> {
        if session_id.is_empty() {
            return Ok(None);
        }
        let sid = session_id.to_string();
        let emoji = emoji.map(str::to_string);
        let emoji = emoji.map(|e| scrub_surrogates(Some(e)).expect("emoji"));
        let author = author.to_string();

        let f = move |conn: &Connection| -> Result<Option<Value>, WriteError> {
            let raw = match conn.query_row(
                "SELECT display_metadata FROM messages WHERE id = ? AND session_id = ?",
                rusqlite::params![message_row_id, sid],
                |r| r.get::<_, Option<String>>(0),
            ) {
                Ok(raw) => raw,
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                Err(e) => return Err(WriteError::Sqlite(e)),
            };
            let raw = raw.unwrap_or_default(); // NULL metadata = no reactions yet
            let mut meta = decode_display_metadata(Some(&raw))
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default();
            let existing = meta
                .get(REACTIONS_METADATA_KEY)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();

            let mut reactions: Vec<Value> = existing
                .iter()
                .filter(|r| {
                    !(r.is_object() && r.get("author").and_then(Value::as_str) == Some(author.as_str()))
                })
                .cloned()
                .collect();
            let previous = existing.iter().find(|r| {
                r.is_object() && r.get("author").and_then(Value::as_str) == Some(author.as_str())
            });
            let toggling_off = emoji.is_some()
                && previous.is_some()
                && previous
                    .and_then(|r| r.get("emoji"))
                    .and_then(Value::as_str)
                    == emoji.as_deref();
            if let Some(emoji) = &emoji {
                if !toggling_off {
                    reactions.push(json!({
                        "emoji": emoji,
                        "author": author,
                        "at": now(),
                    }));
                }
            }

            if reactions.is_empty() {
                meta.remove(REACTIONS_METADATA_KEY);
            } else {
                meta.insert(REACTIONS_METADATA_KEY.to_string(), Value::Array(reactions.clone()));
            }
            let stored = if meta.is_empty() {
                None
            } else {
                encode_display_metadata(Some(&Value::Object(meta)))
            };
            conn.execute(
                "UPDATE messages SET display_metadata = ? WHERE id = ?",
                rusqlite::params![stored, message_row_id],
            )?;
            if reactions.is_empty() {
                Ok(Some(Value::Array(Vec::new())))
            } else {
                Ok(Some(Value::Array(reactions)))
            }
        };
        self.execute_write(&f, None)
    }

    /// Return the reaction list persisted on one message row (never None).
    ///
    /// PARITY: SessionDB.get_message_reactions @ b9aa928 (6933–6952)
    pub fn get_message_reactions(
        &self,
        session_id: &str,
        message_row_id: i64,
    ) -> Result<Value, WriteError> {
        if session_id.is_empty() {
            return Ok(Value::Array(Vec::new()));
        }
        let conn = self.writer_conn();
        let raw = match conn.query_row(
            "SELECT display_metadata FROM messages WHERE id = ? AND session_id = ?",
            rusqlite::params![message_row_id, session_id],
            |r| r.get::<_, Option<String>>(0),
        ) {
            Ok(raw) => raw,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(Value::Array(Vec::new())),
            Err(e) => return Err(WriteError::Sqlite(e)),
        };
        let Some(raw) = raw else {
            return Ok(Value::Array(Vec::new()));
        };
        let reactions = match decode_display_metadata(Some(&raw)) {
            Some(meta) => meta
                .get(REACTIONS_METADATA_KEY)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            None => Vec::new(),
        };
        Ok(Value::Array(
            reactions
                .into_iter()
                .filter(|r| r.is_object())
                .collect(),
        ))
    }

    /// Return `author`'s not-yet-surfaced reactions and mark them seen.
    ///
    /// PARITY: SessionDB.take_unseen_reactions @ b9aa928 (6954–7002)
    pub fn take_unseen_reactions(
        &self,
        session_id: &str,
        author: &str,
    ) -> Result<Value, WriteError> {
        if session_id.is_empty() {
            return Ok(Value::Array(Vec::new()));
        }
        let sid = session_id.to_string();
        let author = author.to_string();

        let f = move |conn: &Connection| -> Result<Value, WriteError> {
            let mut stmt = conn
                .prepare(
                    "SELECT id, role, content, display_metadata FROM messages \
                     WHERE session_id = ? AND active = 1 AND display_metadata IS NOT NULL \
                     ORDER BY id",
                )
                .map_err(WriteError::Sqlite)?;
            let rows = stmt
                .query_map(rusqlite::params![sid], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, rusqlite::types::Value>(2)?,
                        r.get::<_, Option<String>>(3)?,
                    ))
                })
                .map_err(WriteError::Sqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(WriteError::Sqlite)?;

            let mut pending: Vec<Value> = Vec::new();
            for (row_id, role, content_raw, metadata_raw) in rows {
                let Some(metadata_raw) = metadata_raw else { continue };
                let Some(mut meta) = decode_display_metadata(Some(&metadata_raw)) else {
                    continue;
                };
                let Some(reactions) = meta
                    .get_mut(REACTIONS_METADATA_KEY)
                    .and_then(Value::as_array_mut)
                else {
                    continue;
                };
                let mut changed = false;
                for reaction in reactions.iter_mut() {
                    let is_mine = reaction
                        .get("author")
                        .and_then(Value::as_str)
                        == Some(author.as_str());
                    let seen = reaction.get("seen").and_then(Value::as_bool).unwrap_or(false);
                    if !reaction.is_object() || !is_mine || seen {
                        continue;
                    }
                    reaction.as_object_mut().unwrap().insert("seen".into(), Value::Bool(true));
                    changed = true;
                    let content = decode_content(Some(content_raw.clone()));
                    let text = match content {
                        Some(Value::String(s)) => s,
                        _ => String::new(),
                    };
                    pending.push(json!({
                        "row_id": row_id,
                        "role": role,
                        "emoji": reaction.get("emoji").and_then(Value::as_str).unwrap_or(""),
                        "text": text,
                    }));
                }
                if changed {
                    let stored = encode_display_metadata(Some(&meta));
                    conn.execute(
                        "UPDATE messages SET display_metadata = ? WHERE id = ?",
                        rusqlite::params![stored, row_id],
                    )?;
                }
            }
            Ok(Value::Array(pending))
        };
        self.execute_write(&f, None)
    }

    /// Backfill the `api_content` sidecar onto the newest ACTIVE user row.
    ///
    /// PARITY: SessionDB.set_latest_user_api_content @ b9aa928 (7317–7344)
    pub fn set_latest_user_api_content(
        &self,
        session_id: &str,
        content: Option<&Value>,
        api_content: &str,
    ) -> Result<i64, WriteError> {
        let encoded = encode_content(content);
        let sid = session_id.to_string();
        let api_content = scrub_surrogates(Some(api_content.to_string())).expect("api");
        let f = move |conn: &Connection| -> Result<i64, WriteError> {
            let rowcount = conn.execute(
                "UPDATE messages SET api_content = ? WHERE id = ( \
                    SELECT id FROM messages \
                    WHERE session_id = ? AND role = 'user' AND active = 1 \
                    ORDER BY id DESC LIMIT 1 \
                 ) AND content IS ?",
                rusqlite::params![api_content, sid, encoded],
            )?;
            Ok(rowcount as i64)
        };
        self.execute_write(&f, None)
    }
}
