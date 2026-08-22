//! Gateway routing surface — the durable path-index that replaces
//! sessions.json (#9006): per-session peer metadata, routing entries, and
//! origin/peer recovery lookups.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   record_gateway_session_peer          (3309–3391)
//!   set_expiry_finalized                 (3393–3410)
//!   save_gateway_routing_entry           (3412–3438)
//!   replace_gateway_routing_entries      (3440–3461)
//!   load_gateway_routing_entries         (3463–3471)
//!   delete_gateway_routing_entries       (3472–3485)
//!   find_session_by_origin               (3528–3576)
//!   find_latest_gateway_session_for_peer (3578–3617)

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;

use crate::crud::fold_session_dict;
use crate::state::{now, SessionDB, WriteError};

impl SessionDB {
    /// Persist the gateway routing peer for an existing session row.
    /// ``display_name`` / ``origin_json`` are COALESCE'd (None leaves the
    /// existing value untouched). ``include_compression_ancestors`` keeps a
    /// logical compression lineage on one routing peer.
    ///
    /// PARITY: SessionDB.record_gateway_session_peer @ b9aa928 (3309–3391)
    #[allow(clippy::too_many_arguments)]
    pub fn record_gateway_session_peer(
        &self,
        session_id: &str,
        source: &str,
        user_id: Option<&str>,
        session_key: Option<&str>,
        chat_id: Option<&str>,
        chat_type: Option<&str>,
        thread_id: Option<&str>,
        display_name: Option<&str>,
        origin_json: Option<&str>,
        include_compression_ancestors: bool,
    ) -> Result<(), WriteError> {
        if session_id.is_empty() || session_key.map(str::is_empty).unwrap_or(true) {
            return Ok(());
        }
        let session_id = session_id.to_string();
        let session_key = session_key.unwrap_or("").to_string();
        let source = source.to_string();
        let user_id = user_id.map(str::to_string);
        let chat_id = chat_id.map(str::to_string);
        let chat_type = chat_type.map(str::to_string);
        let thread_id = thread_id.map(str::to_string);
        let display_name = display_name.map(str::to_string);
        let origin_json = origin_json.map(str::to_string);

        let f = |conn: &Connection| -> Result<(), WriteError> {
            let lineage_cte = if include_compression_ancestors {
                "WITH RECURSIVE compression_lineage(id) AS ( \
                     SELECT ? \
                     UNION \
                     SELECT parent.id \
                     FROM compression_lineage lineage \
                     JOIN sessions child ON child.id = lineage.id \
                     JOIN sessions parent ON parent.id = child.parent_session_id \
                     WHERE parent.end_reason = 'compression' \
                       AND json_extract(COALESCE(child.model_config, '{}'), '$._branched_from') IS NULL \
                       AND json_extract(COALESCE(child.model_config, '{}'), '$._delegate_from') IS NULL \
                       AND COALESCE(child.source, '') != 'tool' \
                 )
".to_string()
            } else {
                String::new()
            };
            let target_clause = if include_compression_ancestors {
                "WHERE id IN (SELECT id FROM compression_lineage)"
            } else {
                "WHERE id = ?"
            };
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            if include_compression_ancestors {
                params.push(Box::new(session_id.clone()));
            }
            params.push(Box::new(session_key.clone()));
            params.push(Box::new(source.clone()));
            params.push(Box::new(user_id.clone()));
            params.push(Box::new(chat_id.clone()));
            params.push(Box::new(chat_type.clone()));
            params.push(Box::new(thread_id.clone()));
            params.push(Box::new(display_name.clone()));
            params.push(Box::new(origin_json.clone()));
            if !include_compression_ancestors {
                params.push(Box::new(session_id.clone()));
            }
            conn.execute(
                &format!(
                    "{lineage_cte}\
                     UPDATE sessions \
                     SET session_key = ?, source = ?, user_id = ?, chat_id = ?, \
                         chat_type = ?, thread_id = ?, \
                         display_name = COALESCE(?, display_name), \
                         origin_json = COALESCE(?, origin_json) \
                     {target_clause}",
                    lineage_cte = lineage_cte,
                    target_clause = target_clause,
                ),
                rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Mark a gateway session's expiry-finalization flag in state.db.
    ///
    /// PARITY: SessionDB.set_expiry_finalized @ b9aa928 (3393–3410)
    pub fn set_expiry_finalized(&self, session_id: &str, finalized: bool) -> Result<(), WriteError> {
        if session_id.is_empty() {
            return Ok(());
        }
        let session_id = session_id.to_string();
        let flag: i64 = if finalized { 1 } else { 0 };
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET expiry_finalized = ? WHERE id = ?",
                rusqlite::params![flag, session_id],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Upsert one gateway routing entry (session_key -> SessionEntry JSON).
    ///
    /// PARITY: SessionDB.save_gateway_routing_entry @ b9aa928 (3412–3438)
    pub fn save_gateway_routing_entry(
        &self,
        session_key: &str,
        entry_json: &str,
        scope: &str,
    ) -> Result<(), WriteError> {
        if session_key.is_empty() || entry_json.is_empty() {
            return Ok(());
        }
        let session_key = session_key.to_string();
        let entry_json = entry_json.to_string();
        let scope = scope.to_string();
        let when = now();
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "INSERT INTO gateway_routing (scope, session_key, entry_json, updated_at) \
                 VALUES (?, ?, ?, ?) \
                 ON CONFLICT(scope, session_key) DO UPDATE SET \
                     entry_json = excluded.entry_json, \
                     updated_at = excluded.updated_at",
                rusqlite::params![scope, session_key, entry_json, when],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Atomically replace the routing index for `scope` with `entries`.
    ///
    /// PARITY: SessionDB.replace_gateway_routing_entries @ b9aa928 (3440–3461)
    pub fn replace_gateway_routing_entries(
        &self,
        entries: &HashMap<String, String>,
        scope: &str,
    ) -> Result<(), WriteError> {
        let scope = scope.to_string();
        let when = now();
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "DELETE FROM gateway_routing WHERE scope = ?",
                rusqlite::params![scope],
            )?;
            let mut rows: Vec<(String, String, String, f64)> = Vec::new();
            for (k, v) in entries {
                if !k.is_empty() && !v.is_empty() {
                    rows.push((scope.clone(), k.clone(), v.clone(), when));
                }
            }
            if !rows.is_empty() {
                let mut stmt = conn.prepare_cached(
                    "INSERT INTO gateway_routing (scope, session_key, entry_json, updated_at) \
                     VALUES (?, ?, ?, ?)",
                )?;
                for r in &rows {
                    stmt.execute(rusqlite::params![r.0, r.1, r.2, r.3])?;
                }
            }
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Load routing entries for `scope` as {session_key: entry_json}.
    ///
    /// PARITY: SessionDB.load_gateway_routing_entries @ b9aa928 (3463–3471)
    pub fn load_gateway_routing_entries(&self, scope: &str) -> Result<HashMap<String, String>, WriteError> {
        let conn = self.writer_conn();
        let mut stmt = conn
            .prepare("SELECT session_key, entry_json FROM gateway_routing WHERE scope = ?")
            .map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map([scope], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows.into_iter().collect())
    }

    /// Remove routing entries for the given session keys in `scope`.
    ///
    /// PARITY: SessionDB.delete_gateway_routing_entries @ b9aa928 (3472–3485)
    pub fn delete_gateway_routing_entries(
        &self,
        session_keys: &[String],
        scope: &str,
    ) -> Result<(), WriteError> {
        if session_keys.is_empty() {
            return Ok(());
        }
        let keys: Vec<String> = session_keys.to_vec();
        let scope = scope.to_string();
        let f = |conn: &Connection| -> Result<(), WriteError> {
            let mut stmt = conn.prepare_cached(
                "DELETE FROM gateway_routing WHERE scope = ? AND session_key = ?",
            )?;
            for k in &keys {
                stmt.execute(rusqlite::params![scope, k])?;
            }
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Find the most recent live session_id for a platform + chat origin.
    ///
    /// PARITY: SessionDB.find_session_by_origin @ b9aa928 (3528–3576)
    pub fn find_session_by_origin(
        &self,
        platform: &str,
        chat_id: &str,
        thread_id: Option<&str>,
        user_id: Option<&str>,
    ) -> Result<Option<String>, WriteError> {
        if platform.is_empty() || chat_id.is_empty() {
            return Ok(None);
        }
        let mut query = "SELECT id, user_id, started_at FROM sessions \
                         WHERE LOWER(source) = LOWER(?) \
                           AND session_key IS NOT NULL \
                           AND chat_id = ? \
                           AND ended_at IS NULL"
            .to_string();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params.push(Box::new(platform.to_string()));
        params.push(Box::new(chat_id.to_string()));
        if let Some(thread_id) = thread_id {
            query += " AND COALESCE(thread_id, '') = ?";
            params.push(Box::new(thread_id.to_string()));
        }
        query += " ORDER BY started_at DESC";

        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&query).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, f64>(2)?,
                    ))
                },
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;

        if rows.is_empty() {
            return Ok(None);
        }
        if let Some(user_id) = user_id {
            let exact: Vec<&(String, Option<String>, f64)> = rows
                .iter()
                .filter(|r| r.1.as_deref().unwrap_or("") == user_id)
                .collect();
            if !exact.is_empty() {
                return Ok(Some(exact[0].0.clone()));
            }
            if rows.len() > 1 {
                return Ok(None);
            }
        } else if rows.len() > 1 {
            let distinct: std::collections::HashSet<String> = rows
                .iter()
                .filter_map(|r| {
                    r.1.as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                })
                .collect();
            if distinct.len() > 1 {
                return Ok(None);
            }
        }
        Ok(Some(rows[0].0.clone()))
    }

    /// Find the latest recoverable gateway session for a routing peer.
    ///
    /// PARITY: SessionDB.find_latest_gateway_session_for_peer @ b9aa928
    /// (3578–3617)
    pub fn find_latest_gateway_session_for_peer(
        &self,
        source: &str,
        user_id: Option<&str>,
        session_key: Option<&str>,
        chat_id: Option<&str>,
        chat_type: Option<&str>,
        thread_id: Option<&str>,
    ) -> Result<Option<Value>, WriteError> {
        let Some(session_key) = session_key else {
            return Ok(None);
        };
        if session_key.is_empty() {
            return Ok(None);
        }
        let conn = self.writer_conn();
        let peer_recoverable = "(s.ended_at IS NULL OR s.end_reason IN ('agent_close', 'ws_orphan_reap')) \
            AND (COALESCE(s.message_count, 0) > 0 OR EXISTS ( \
                SELECT 1 FROM messages WHERE messages.session_id = s.id LIMIT 1 \
            ))";
        let row = conn
            .query_row(
                &format!(
                    "SELECT s.*, \
                         COALESCE(sp.prompt, s.system_prompt) AS _system_prompt_resolved \
                     FROM sessions s \
                     LEFT JOIN system_prompts sp ON sp.hash = s.system_prompt_hash \
                     WHERE s.session_key = ? \
                       AND s.source = ? \
                       AND {peer_recoverable} \
                     ORDER BY s.started_at DESC \
                     LIMIT 1"
                ),
                rusqlite::params![session_key, source],
                crate::portability::row_to_value,
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        if let Some(row) = row {
            return Ok(Some(fold_session_dict(row)));
        }

        // Conservative fallback for rows created by current code but with a
        // temporarily-missing exact key: require the complete peer tuple.
        let (Some(chat_id), Some(chat_type)) = (chat_id, chat_type) else {
            return Ok(None);
        };
        let row = conn
            .query_row(
                &format!(
                    "SELECT s.*, \
                         COALESCE(sp.prompt, s.system_prompt) AS _system_prompt_resolved \
                     FROM sessions s \
                     LEFT JOIN system_prompts sp ON sp.hash = s.system_prompt_hash \
                     WHERE s.source = ? \
                       AND COALESCE(s.user_id, '') = COALESCE(?, '') \
                       AND COALESCE(s.chat_id, '') = COALESCE(?, '') \
                       AND COALESCE(s.chat_type, '') = COALESCE(?, '') \
                       AND COALESCE(s.thread_id, '') = COALESCE(?, '') \
                       AND {peer_recoverable} \
                     ORDER BY s.started_at DESC \
                     LIMIT 1",
                    peer_recoverable = peer_recoverable,
                ),
                rusqlite::params![
                    source,
                    user_id.unwrap_or(""),
                    chat_id,
                    chat_type,
                    thread_id.unwrap_or("")
                ],
                crate::portability::row_to_value,
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(row.map(fold_session_dict))
    }
}
