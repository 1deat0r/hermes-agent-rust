//! Telegram DM topic-mode + bindings surface.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   apply_telegram_topic_migration       (9156–9255)
//!   enable / disable / is_enabled        (9256–9355)
//!   get / list / get_by_session / delete (9356–9497)
//!   bind / is_linked                     (9498–9583)
//!   list_unlinked_telegram_sessions_for_user (9584–9700)
//!
//! The migration is deliberately NOT part of SessionDB startup
//! reconciliation — tables are created only on explicit /topic opt-in
//! (enable / bind call it; the read-only helpers never do).

use rusqlite::{Connection, OptionalExtension, Row};
use serde_json::{json, Value};

use crate::crud;
use crate::state::{now, SessionDB, WriteError};

/// Capability flags on the telegram_dm_topic_mode row.
pub type TopicCapabilityFlags = (Option<i64>, Option<i64>);

pub(crate) fn is_missing_table(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(_, Some(msg))
            if msg.to_ascii_lowercase().contains("no such table")
    )
}

/// One row of `telegram_dm_topic_bindings` (SELECT * order preserved by
/// `to_value`, matching upstream `dict(row)`).
#[derive(Debug, Clone)]
pub struct TopicBinding {
    pub chat_id: String,
    pub thread_id: String,
    pub user_id: String,
    pub session_key: String,
    pub session_id: String,
    pub managed_mode: String,
    pub linked_at: f64,
    pub updated_at: f64,
}

impl TopicBinding {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<TopicBinding> {
        Ok(TopicBinding {
            chat_id: row.get("chat_id")?,
            thread_id: row.get("thread_id")?,
            user_id: row.get("user_id")?,
            session_key: row.get("session_key")?,
            session_id: row.get("session_id")?,
            managed_mode: row.get("managed_mode")?,
            linked_at: row.get("linked_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// JSON object in the table's column order (upstream `dict(row)`).
    pub fn to_value(&self) -> Value {
        json!({
            "chat_id": self.chat_id,
            "thread_id": self.thread_id,
            "user_id": self.user_id,
            "session_key": self.session_key,
            "session_id": self.session_id,
            "managed_mode": self.managed_mode,
            "linked_at": self.linked_at,
            "updated_at": self.updated_at,
        })
    }
}

impl SessionDB {
    /// Create Telegram DM topic-mode tables on explicit /topic opt-in.
    ///
    /// Schema versions: v1 (initial, no ON DELETE CASCADE), v2 (session_id
    /// FK gets ON DELETE CASCADE so session pruning clears bindings).
    ///
    /// PARITY: SessionDB.apply_telegram_topic_migration @ b9aa928 (9156–9255)
    pub fn apply_telegram_topic_migration(&self) -> Result<(), WriteError> {
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS telegram_dm_topic_mode (
                    chat_id TEXT PRIMARY KEY,
                    user_id TEXT NOT NULL,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    activated_at REAL NOT NULL,
                    updated_at REAL NOT NULL,
                    has_topics_enabled INTEGER,
                    allows_users_to_create_topics INTEGER,
                    capability_checked_at REAL,
                    intro_message_id TEXT,
                    pinned_message_id TEXT
                );

                CREATE TABLE IF NOT EXISTS telegram_dm_topic_bindings (
                    chat_id TEXT NOT NULL,
                    thread_id TEXT NOT NULL,
                    user_id TEXT NOT NULL,
                    session_key TEXT NOT NULL,
                    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                    managed_mode TEXT NOT NULL DEFAULT 'auto',
                    linked_at REAL NOT NULL,
                    updated_at REAL NOT NULL,
                    PRIMARY KEY (chat_id, thread_id)
                );

                CREATE UNIQUE INDEX IF NOT EXISTS idx_telegram_dm_topic_bindings_session
                ON telegram_dm_topic_bindings(session_id);

                CREATE INDEX IF NOT EXISTS idx_telegram_dm_topic_bindings_user
                ON telegram_dm_topic_bindings(user_id, chat_id);",
            )?;

            // v1 → v2: rebuild the bindings table when its session_id FK
            // lacks ON DELETE CASCADE (SQLite can't ALTER a foreign key).
            let current_version: i64 = conn
                .query_row(
                    "SELECT value FROM state_meta WHERE key = ?",
                    rusqlite::params!["telegram_dm_topic_schema_version"],
                    |r| {
                        r.get::<_, String>(0).and_then(|s| {
                            s.parse::<i64>().map_err(|_| rusqlite::Error::InvalidQuery)
                        })
                    },
                )
                .optional()?
                .unwrap_or(0);
            if current_version < 2 {
                let mut stmt = conn
                    .prepare("PRAGMA foreign_key_list('telegram_dm_topic_bindings')")
                    .map_err(WriteError::Sqlite)?;
                let fk_rows = stmt
                    .query_map([], |r| {
                        Ok((r.get::<_, String>(2)?, r.get::<_, Option<String>>(6)?))
                    })
                    .map_err(WriteError::Sqlite)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(WriteError::Sqlite)?;
                let needs_rebuild = fk_rows.iter().any(|(table, on_delete)| {
                    table == "sessions" && on_delete.as_deref() != Some("CASCADE")
                });
                if needs_rebuild {
                    conn.execute_batch(
                        "CREATE TABLE telegram_dm_topic_bindings_new (
                            chat_id TEXT NOT NULL,
                            thread_id TEXT NOT NULL,
                            user_id TEXT NOT NULL,
                            session_key TEXT NOT NULL,
                            session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                            managed_mode TEXT NOT NULL DEFAULT 'auto',
                            linked_at REAL NOT NULL,
                            updated_at REAL NOT NULL,
                            PRIMARY KEY (chat_id, thread_id)
                        );
                        INSERT INTO telegram_dm_topic_bindings_new
                            SELECT chat_id, thread_id, user_id, session_key,
                                   session_id, managed_mode, linked_at, updated_at
                            FROM telegram_dm_topic_bindings;
                        DROP TABLE telegram_dm_topic_bindings;
                        ALTER TABLE telegram_dm_topic_bindings_new
                            RENAME TO telegram_dm_topic_bindings;
                        CREATE UNIQUE INDEX idx_telegram_dm_topic_bindings_session
                            ON telegram_dm_topic_bindings(session_id);
                        CREATE INDEX idx_telegram_dm_topic_bindings_user
                            ON telegram_dm_topic_bindings(user_id, chat_id);",
                    )?;
                }
            }

            conn.execute(
                "INSERT INTO state_meta (key, value) VALUES (?, ?) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params!["telegram_dm_topic_schema_version", "2"],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Enable Telegram DM topic mode for one private chat/user. Owns the
    /// explicit topic migration.
    ///
    /// PARITY: SessionDB.enable_telegram_topic_mode @ b9aa928 (9256–9304)
    pub fn enable_telegram_topic_mode(
        &self,
        chat_id: &str,
        user_id: &str,
        has_topics_enabled: Option<bool>,
        allows_users_to_create_topics: Option<bool>,
    ) -> Result<(), WriteError> {
        self.apply_telegram_topic_migration()?;
        let ts = now();
        let to_int = |v: Option<bool>| v.map(|b| if b { 1 } else { 0 });
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "INSERT INTO telegram_dm_topic_mode (
                    chat_id, user_id, enabled, activated_at, updated_at,
                    has_topics_enabled, allows_users_to_create_topics,
                    capability_checked_at
                ) VALUES (?, ?, 1, ?, ?, ?, ?, ?)
                ON CONFLICT(chat_id) DO UPDATE SET
                    user_id = excluded.user_id,
                    enabled = 1,
                    updated_at = excluded.updated_at,
                    has_topics_enabled = excluded.has_topics_enabled,
                    allows_users_to_create_topics = excluded.allows_users_to_create_topics,
                    capability_checked_at = excluded.capability_checked_at",
                rusqlite::params![
                    chat_id.to_string(),
                    user_id.to_string(),
                    ts,
                    ts,
                    to_int(has_topics_enabled),
                    to_int(allows_users_to_create_topics),
                    ts,
                ],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Test/insight helper — read the capability flags off the mode row.
    pub fn get_telegram_topic_bound_cols(
        &self,
        chat_id: &str,
        user_id: &str,
    ) -> Result<Option<TopicCapabilityFlags>, WriteError> {
        let conn = self.writer_conn();
        let row = conn
            .query_row(
                "SELECT has_topics_enabled, allows_users_to_create_topics \
                 FROM telegram_dm_topic_mode WHERE chat_id = ? AND user_id = ?",
                rusqlite::params![chat_id.to_string(), user_id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(row)
    }

    /// Disable Telegram DM topic mode for one private chat. Never creates
    /// the topic-mode tables: if they don't exist the call is a no-op.
    ///
    /// PARITY: SessionDB.disable_telegram_topic_mode @ b9aa928 (9305–9336)
    pub fn disable_telegram_topic_mode(
        &self,
        chat_id: &str,
        clear_bindings: bool,
    ) -> Result<(), WriteError> {
        let f = |conn: &Connection| -> Result<(), WriteError> {
            match conn.execute(
                "UPDATE telegram_dm_topic_mode SET enabled = 0, updated_at = ? \
                 WHERE chat_id = ?",
                rusqlite::params![now(), chat_id.to_string()],
            ) {
                Ok(_) => {}
                Err(e) if is_missing_table(&e) => return Ok(()),
                Err(e) => return Err(WriteError::Sqlite(e)),
            }
            if clear_bindings {
                match conn.execute(
                    "DELETE FROM telegram_dm_topic_bindings WHERE chat_id = ?",
                    rusqlite::params![chat_id.to_string()],
                ) {
                    Ok(_) => {}
                    Err(e) if is_missing_table(&e) => return Ok(()),
                    Err(e) => return Err(WriteError::Sqlite(e)),
                }
            }
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Whether Telegram DM topic mode is enabled for this chat/user.
    ///
    /// PARITY: SessionDB.is_telegram_topic_mode_enabled @ b9aa928 (9338–9354)
    pub fn is_telegram_topic_mode_enabled(&self, chat_id: &str, user_id: &str) -> bool {
        let conn = self.writer_conn();
        let result = conn
            .query_row(
                "SELECT enabled FROM telegram_dm_topic_mode \
                 WHERE chat_id = ? AND user_id = ?",
                rusqlite::params![chat_id.to_string(), user_id.to_string()],
                |r| r.get::<_, i64>(0),
            )
            .optional();
        match result {
            Ok(Some(enabled)) => enabled != 0,
            Ok(None) => false,
            // Fail-open for missing tables and any transient read error
            // (upstream: OperationalError -> False; row-missing -> False).
            Err(_) => false,
        }
    }

    /// The session binding for a Telegram DM topic, if present.
    ///
    /// PARITY: SessionDB.get_telegram_topic_binding @ b9aa928 (9356–9375)
    pub fn get_telegram_topic_binding(
        &self,
        chat_id: &str,
        thread_id: &str,
    ) -> Result<Option<TopicBinding>, WriteError> {
        let conn = self.writer_conn();
        let row = conn
            .query_row(
                "SELECT * FROM telegram_dm_topic_bindings \
                 WHERE chat_id = ? AND thread_id = ?",
                rusqlite::params![chat_id.to_string(), thread_id.to_string()],
                TopicBinding::from_row,
            )
            .optional();
        match row {
            Ok(r) => Ok(r),
            Err(e) if is_missing_table(&e) => Ok(None),
            Err(e) => Err(WriteError::Sqlite(e)),
        }
    }

    /// All Telegram DM topic bindings for one chat, newest first. Returns
    /// [] when the bindings table doesn't exist yet (no migration trigger).
    ///
    /// PARITY: SessionDB.list_telegram_topic_bindings_for_chat @ b9aa928 (9376–9396)
    pub fn list_telegram_topic_bindings_for_chat(
        &self,
        chat_id: &str,
    ) -> Result<Vec<TopicBinding>, WriteError> {
        let conn = self.writer_conn();
        let result = conn
            .prepare(
                "SELECT * FROM telegram_dm_topic_bindings \
                      WHERE chat_id = ? ORDER BY updated_at DESC",
            )
            .and_then(|mut stmt| {
                let rows = stmt.query_map(
                    rusqlite::params![chat_id.to_string()],
                    TopicBinding::from_row,
                )?;
                rows.collect::<Result<Vec<_>, _>>()
            });
        match result {
            Ok(v) => Ok(v),
            Err(e) if is_missing_table(&e) => Ok(Vec::new()),
            Err(e) => Err(WriteError::Sqlite(e)),
        }
    }

    /// The Telegram DM topic binding for a given session_id, if present.
    ///
    /// PARITY: SessionDB.get_telegram_topic_binding_by_session @ b9aa928 (9397–9420)
    pub fn get_telegram_topic_binding_by_session(
        &self,
        session_id: &str,
    ) -> Result<Option<TopicBinding>, WriteError> {
        let conn = self.writer_conn();
        let row = conn
            .query_row(
                "SELECT * FROM telegram_dm_topic_bindings WHERE session_id = ?",
                rusqlite::params![session_id.to_string()],
                TopicBinding::from_row,
            )
            .optional();
        match row {
            Ok(r) => Ok(r),
            Err(e) if is_missing_table(&e) => Ok(None),
            Err(e) => Err(WriteError::Sqlite(e)),
        }
    }

    /// Remove the binding row for a single (chat, thread) pair — the
    /// targeted prune after a Telegram-confirmed topic deletion (#31501).
    /// When the prune removes the chat's last binding, the chat's
    /// `telegram_dm_topic_mode.enabled` is flipped to 0 in the same
    /// transaction so recovery fully stands down. Returns the number of
    /// binding rows deleted (0 for absent row or missing tables — silent).
    ///
    /// PARITY: SessionDB.delete_telegram_topic_binding @ b9aa928 (9421–9497)
    pub fn delete_telegram_topic_binding(
        &self,
        chat_id: &str,
        thread_id: &str,
    ) -> Result<i64, WriteError> {
        let chat_id = chat_id.to_string();
        let thread_id = thread_id.to_string();
        let deleted = std::cell::Cell::new(0i64);
        let f = |conn: &Connection| -> Result<(), WriteError> {
            let rowcount = match conn.execute(
                "DELETE FROM telegram_dm_topic_bindings \
                 WHERE chat_id = ? AND thread_id = ?",
                rusqlite::params![chat_id, thread_id],
            ) {
                Ok(n) => n as i64,
                Err(e) if is_missing_table(&e) => 0,
                Err(e) => return Err(WriteError::Sqlite(e)),
            };
            deleted.set(rowcount);
            if rowcount == 0 {
                return Ok(());
            }
            // If that was the chat's last binding, disable topic mode so
            // recovery stops steering lobby messages at an empty lane set.
            let remaining = conn
                .query_row(
                    "SELECT 1 FROM telegram_dm_topic_bindings \
                     WHERE chat_id = ? LIMIT 1",
                    rusqlite::params![chat_id],
                    |r| r.get::<_, i64>(0),
                )
                .optional();
            match remaining {
                Ok(None) => {
                    match conn.execute(
                        "UPDATE telegram_dm_topic_mode \
                         SET enabled = 0, updated_at = ? WHERE chat_id = ?",
                        rusqlite::params![now(), chat_id],
                    ) {
                        Ok(_) => {}
                        Err(e) if is_missing_table(&e) => {} // prune still stands
                        Err(e) => return Err(WriteError::Sqlite(e)),
                    }
                }
                Ok(Some(_)) => {}
                Err(e) if is_missing_table(&e) => {}
                Err(e) => return Err(WriteError::Sqlite(e)),
            }
            Ok(())
        };
        self.execute_write(&f, None)?;
        Ok(deleted.get())
    }

    /// Bind one Telegram DM topic thread to one Hermes session. A session
    /// may only be linked to one topic; linking to a different topic raises
    /// ValueError. Idempotent for the same topic.
    ///
    /// PARITY: SessionDB.bind_telegram_topic @ b9aa928 (9498–9561)
    pub fn bind_telegram_topic(
        &self,
        chat_id: &str,
        thread_id: &str,
        user_id: &str,
        session_key: &str,
        session_id: &str,
        managed_mode: &str,
    ) -> Result<(), WriteError> {
        self.apply_telegram_topic_migration()?;
        let ts = now();
        let chat_id = chat_id.to_string();
        let thread_id = thread_id.to_string();
        let user_id = user_id.to_string();
        let session_key = session_key.to_string();
        let session_id = session_id.to_string();
        let f = |conn: &Connection| -> Result<(), WriteError> {
            let existing_session = conn
                .query_row(
                    "SELECT chat_id, thread_id FROM telegram_dm_topic_bindings \
                     WHERE session_id = ?",
                    rusqlite::params![session_id],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(WriteError::Sqlite)?;
            if let Some((linked_chat, linked_thread)) = existing_session {
                if linked_chat != chat_id || linked_thread != thread_id {
                    return Err(WriteError::ValueError(
                        "session is already linked to another Telegram topic".to_string(),
                    ));
                }
            }
            conn.execute(
                "INSERT INTO telegram_dm_topic_bindings (
                    chat_id, thread_id, user_id, session_key, session_id,
                    managed_mode, linked_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(chat_id, thread_id) DO UPDATE SET
                    user_id = excluded.user_id,
                    session_key = excluded.session_key,
                    session_id = excluded.session_id,
                    managed_mode = excluded.managed_mode,
                    updated_at = excluded.updated_at",
                rusqlite::params![
                    chat_id,
                    thread_id,
                    user_id,
                    session_key,
                    session_id,
                    managed_mode.to_string(),
                    ts,
                    ts,
                ],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// True if a Hermes session is already bound to any Telegram DM topic.
    /// Read-only: does NOT trigger the migration (absent tables → False).
    ///
    /// PARITY: SessionDB.is_telegram_session_linked_to_topic @ b9aa928 (9562–9583)
    pub fn is_telegram_session_linked_to_topic(&self, session_id: &str) -> bool {
        let conn = self.writer_conn();
        let result = conn
            .query_row(
                "SELECT 1 FROM telegram_dm_topic_bindings \
                 WHERE session_id = ? LIMIT 1",
                rusqlite::params![session_id.to_string()],
                |r| r.get::<_, i64>(0),
            )
            .optional();
        match result {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(_) => false,
        }
    }

    /// Previous Telegram sessions for this user not bound to a topic,
    /// newest-first, each carrying a preview and resolved last-active.
    /// Absent topic tables fall back to "every telegram session is
    /// unlinked" (no NOT EXISTS clause).
    ///
    /// PARITY: SessionDB.list_unlinked_telegram_sessions_for_user @ b9aa928
    /// (9584–9700)
    pub fn list_unlinked_telegram_sessions_for_user(
        &self,
        _chat_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<Value>, WriteError> {
        let preview = crate::common::_preview_raw_select();
        let last_active = crate::common::_sql_session_last_active("s");
        let base = format!(
            "SELECT s.*,
                COALESCE(sp.prompt, s.system_prompt) AS _system_prompt_resolved,
                COALESCE(
                    (SELECT {preview}
                     FROM messages m
                     WHERE m.session_id = s.id AND m.role = 'user' AND m.content IS NOT NULL
                     ORDER BY m.timestamp, m.id LIMIT 1),
                    ''
                ) AS _preview_raw,
                {last_active} AS last_active
            FROM sessions s
            LEFT JOIN system_prompts sp ON sp.hash = s.system_prompt_hash
            WHERE s.source = 'telegram' AND s.user_id = ?
            "
        );
        let linked_clause = "AND NOT EXISTS (SELECT 1 FROM telegram_dm_topic_bindings b WHERE b.session_id = s.id)\n";
        let order_limit = "ORDER BY last_active DESC, s.started_at DESC LIMIT ?";

        let conn = self.writer_conn();
        let attempt = query_unlinked(
            &conn,
            &format!("{base}{linked_clause}{order_limit}"),
            user_id,
            limit,
        );
        let rows = match attempt {
            Ok(rows) => rows,
            Err(e) => match &e {
                WriteError::Sqlite(se) if is_missing_table(se) => {
                    query_unlinked(&conn, &format!("{base}{order_limit}"), user_id, limit)?
                }
                _ => return Err(e),
            },
        };

        let mut sessions: Vec<Value> = Vec::new();
        for row in rows {
            let mut session = crud::fold_session_dict(row);
            let preview_raw = session
                .as_object_mut()
                .and_then(|o| o.remove("_preview_raw"))
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            if let Some(obj) = session.as_object_mut() {
                obj.insert(
                    "preview".to_string(),
                    json!(crate::common::_shape_preview(preview_raw)),
                );
            }
            sessions.push(session);
        }
        Ok(sessions)
    }
}

fn query_unlinked(
    conn: &Connection,
    sql: &str,
    user_id: &str,
    limit: i64,
) -> Result<Vec<Value>, WriteError> {
    let mut stmt = conn.prepare(sql).map_err(WriteError::Sqlite)?;
    let rows = stmt
        .query_map(
            rusqlite::params![user_id.to_string(), limit],
            crate::portability::row_to_value,
        )
        .map_err(WriteError::Sqlite)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(WriteError::Sqlite)?;
    Ok(rows)
}
