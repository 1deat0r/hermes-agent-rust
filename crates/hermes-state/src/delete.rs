//! Session deletion + maintenance surfaces: message counting, on-disk
//! cleanup, cascade deletion of delegate subagent children, empty-session
//! reaping, marker purging, and optional VACUUM/auto-maintenance.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   message_count                         (8201–8208)
//!   has_platform_message_id               (8212–8229)
//!   clear_messages                        (8301–8309)
//!   get_session_delete_targets            (8340–8355)
//!   delete_session                        (8357–8414)
//!   delete_session_if_empty               (8416–8458)
//!   delete_sessions                       (8460–8528)
//!   delete_empty_sessions                 (8569–8640)
//!   purge_stale_tool_call_markers         (8974–9038)
//!   retag_kanban_worker_sessions          (9121–9145)
//!   logical_size_bytes                    (9669–9695)
//!   vacuum                                (9697–9720)
//!   maybe_auto_prune_and_vacuum           (9735–9823)
//!   maybe_auto_archive                    (9825–9860)
//! module helpers: _collect_delegate_child_ids (212–243),
//! _delete_delegate_children (244–270)

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::prune::{remove_session_files, PruneFilters};
use rusqlite::OptionalExtension;
use crate::state::{now, SessionDB, WriteError};

static STALE_TOOL_CALL_MARKER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\[[A-Za-z_][A-Za-z0-9_.-]*\]$").expect("stale marker re")
});

fn delegate_from_json_sql(col: &str) -> String {
    format!("json_extract(COALESCE({col}, '{{}}'), '$._delegate_from')")
}

/// Delegate-subagent ids to cascade-delete with `parent_ids`. Only rows
/// carrying the `_delegate_from` marker; walks marker chains recursively.
///
/// PARITY: hermes_state.py _collect_delegate_child_ids @ b9aa928 (212–243)
pub(crate) fn collect_delegate_child_ids(
    conn: &Connection,
    parent_ids: &[String],
) -> Result<Vec<String>, WriteError> {
    let seeds: HashSet<String> = parent_ids.iter().filter(|s| !s.is_empty()).cloned().collect();
    let df = delegate_from_json_sql("model_config");
    let mut found: HashSet<String> = seeds.clone();
    let mut frontier: Vec<String> = seeds.iter().cloned().collect();
    while !frontier.is_empty() {
        let placeholders = vec!["?"; frontier.len()].join(",");
        let sql = format!(
            "SELECT id FROM sessions WHERE {df} IN ({ph}) \
             OR (parent_session_id IN ({ph}) AND {df} IS NOT NULL)",
            df = df,
            ph = placeholders,
        );
        let mut frontier_params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for _ in 0..2 {
            for f in &frontier {
                frontier_params.push(f as &dyn rusqlite::ToSql);
            }
        }
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let next: Vec<String> = stmt
            .query_map(rusqlite::params_from_iter(frontier_params.iter()), |r| r.get(0))
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<String>, _>>()
            .map_err(WriteError::Sqlite)?
            .into_iter()
            .filter(|id| !found.contains(id))
            .collect();
        frontier = next.clone();
        found.extend(next);
    }
    // Return only the discovered children — never the parents themselves.
    Ok(found
        .iter()
        .filter(|sid| !seeds.contains(*sid))
        .cloned()
        .collect())
}

/// Delete delegate children under `parent_ids` (messages + rows), orphaning
/// any untagged stragglers pointing at a doomed row (FK safety).
///
/// PARITY: hermes_state.py _delete_delegate_children @ b9aa928 (244–270)
pub(crate) fn delete_delegate_children(
    conn: &Connection,
    parent_ids: &[String],
) -> Result<Vec<String>, WriteError> {
    let ids = collect_delegate_child_ids(conn, parent_ids)?;
    if !ids.is_empty() {
        let placeholders = vec!["?"; ids.len()].join(",");
        conn.execute(
            &format!("DELETE FROM messages WHERE session_id IN ({placeholders})"),
            rusqlite::params_from_iter(ids.iter()),
        )?;
        conn.execute(
            &format!(
                "UPDATE sessions SET parent_session_id = NULL \
                 WHERE parent_session_id IN ({placeholders})"
            ),
            rusqlite::params_from_iter(ids.iter()),
        )?;
        conn.execute(
            &format!("DELETE FROM sessions WHERE id IN ({placeholders})"),
            rusqlite::params_from_iter(ids.iter()),
        )?;
    }
    Ok(ids)
}

impl SessionDB {
    /// Count messages, optionally for a specific session.
    ///
    /// PARITY: SessionDB.message_count @ b9aa928 (8201–8208)
    pub fn message_count(&self, session_id: Option<&str>) -> Result<i64, WriteError> {
        let conn = self.writer_conn();
        let count = match session_id {
            Some(sid) => conn.query_row(
                "SELECT COUNT(*) FROM messages WHERE session_id = ?",
                rusqlite::params![sid],
                |r| r.get(0),
            ),
            None => conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0)),
        };
        count.map_err(WriteError::Sqlite)
    }

    /// Check if a message with the given platform_message_id exists.
    ///
    /// PARITY: SessionDB.has_platform_message_id @ b9aa928 (8212–8229)
    pub fn has_platform_message_id(
        &self,
        session_id: &str,
        platform_message_id: &str,
    ) -> Result<bool, WriteError> {
        let conn = self.writer_conn();
        let found = conn
            .query_row(
                "SELECT 1 FROM messages \
                 WHERE session_id = ? AND platform_message_id = ? LIMIT 1",
                rusqlite::params![session_id, platform_message_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(found.is_some())
    }

    /// Delete all messages for a session and reset its counters.
    ///
    /// PARITY: SessionDB.clear_messages @ b9aa928 (8301–8309)
    pub fn clear_messages(&self, session_id: &str) -> Result<(), WriteError> {
        let sid = session_id.to_string();
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute("DELETE FROM messages WHERE session_id = ?", rusqlite::params![sid])?;
            conn.execute(
                "UPDATE sessions SET message_count = 0, tool_call_count = 0 WHERE id = ?",
                rusqlite::params![sid],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Every session row that `delete_session` would remove: the requested
    /// session first, then its recursively discovered delegate/subagent
    /// children.
    ///
    /// PARITY: SessionDB.get_session_delete_targets @ b9aa928 (8340–8355)
    pub fn get_session_delete_targets(&self, session_id: &str) -> Result<Vec<String>, WriteError> {
        let conn = self.writer_conn();
        let exists = conn
            .query_row(
                "SELECT 1 FROM sessions WHERE id = ? LIMIT 1",
                rusqlite::params![session_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        if exists.is_none() {
            return Ok(Vec::new());
        }
        let mut delegate_ids = collect_delegate_child_ids(&conn, &[session_id.to_string()])?;
        delegate_ids.sort();
        let mut out = vec![session_id.to_string()];
        out.extend(delegate_ids);
        Ok(out)
    }

    /// Delete a session and all its messages (delegate cascade, branch
    /// orphan, expected-set guard, on-disk file cleanup).
    ///
    /// PARITY: SessionDB.delete_session @ b9aa928 (8357–8414)
    pub fn delete_session(
        &self,
        session_id: &str,
        sessions_dir: Option<&Path>,
        expected_delete_ids: Option<&[String]>,
    ) -> Result<bool, WriteError> {
        let sid = session_id.to_string();
        let expected_ids: Option<HashSet<String>> = expected_delete_ids.map(|v| v.iter().cloned().collect());
        let removed_delegate_ids: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());

        let f = |conn: &Connection| -> Result<bool, WriteError> {
            let exists = conn
                .query_row(
                    "SELECT 1 FROM sessions WHERE id = ? LIMIT 1",
                    rusqlite::params![sid],
                    |_| Ok(()),
                )
                .optional()?;
            if exists.is_none() {
                return Ok(false);
            }
            if let Some(expected) = &expected_ids {
                let actual_ids: HashSet<String> = {
                    let mut s: HashSet<String> = [sid.clone()].into_iter().collect();
                    s.extend(collect_delegate_child_ids(conn, std::slice::from_ref(&sid))?);
                    s
                };
                if actual_ids != *expected {
                    return Ok(false);
                }
            }
            removed_delegate_ids.borrow_mut().extend(delete_delegate_children(conn, std::slice::from_ref(&sid))?);
            conn.execute(
                "UPDATE sessions SET parent_session_id = NULL WHERE parent_session_id = ?",
                rusqlite::params![sid],
            )?;
            conn.execute("DELETE FROM messages WHERE session_id = ?", rusqlite::params![sid])?;
            conn.execute("DELETE FROM sessions WHERE id = ?", rusqlite::params![sid])?;
            crate::crud::delete_unreferenced_system_prompts(conn)?;
            Ok(true)
        };
        let deleted = self.execute_write(&f, None)?;
        if deleted {
            for delegate_id in removed_delegate_ids.borrow().iter() {
                if let Some(dir) = sessions_dir {
                    remove_session_files(dir, delegate_id);
                }
            }
            if let Some(dir) = sessions_dir {
                remove_session_files(dir, session_id);
            }
        }
        Ok(deleted)
    }

    /// Delete `session_id` only when it never gained resumable content
    /// (no messages, no title, no children).
    ///
    /// PARITY: SessionDB.delete_session_if_empty @ b9aa928 (8416–8458)
    pub fn delete_session_if_empty(
        &self,
        session_id: &str,
        sessions_dir: Option<&Path>,
    ) -> Result<bool, WriteError> {
        let sid = session_id.to_string();
        let f = |conn: &Connection| -> Result<bool, WriteError> {
            let rowcount = conn.execute(
                "DELETE FROM sessions \
                 WHERE id = ? \
                   AND title IS NULL \
                   AND NOT EXISTS (SELECT 1 FROM messages WHERE messages.session_id = sessions.id) \
                   AND NOT EXISTS (SELECT 1 FROM sessions child WHERE child.parent_session_id = sessions.id)",
                rusqlite::params![sid],
            )?;
            if rowcount > 0 {
                crate::crud::delete_unreferenced_system_prompts(conn)?;
            }
            Ok(rowcount > 0)
        };
        let deleted = self.execute_write(&f, None)?;
        if deleted {
            if let Some(dir) = sessions_dir {
                remove_session_files(dir, session_id);
            }
        }
        Ok(deleted)
    }

    /// Delete every session in `session_ids` in a single transaction.
    ///
    /// PARITY: SessionDB.delete_sessions @ b9aa928 (8460–8528)
    pub fn delete_sessions(
        &self,
        session_ids: &[String],
        sessions_dir: Option<&Path>,
    ) -> Result<i64, WriteError> {
        if session_ids.is_empty() {
            return Ok(0);
        }
        let unique_ids: Vec<String> = session_ids
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        if unique_ids.is_empty() {
            return Ok(0);
        }
        let unique_ids = sort_for_stability(unique_ids);
        let removed_ids: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
        let removed_delegate_ids: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());

        let f = |conn: &Connection| -> Result<i64, WriteError> {
            let placeholders = vec!["?"; unique_ids.len()].join(",");
            let mut stmt = conn
                .prepare(&format!("SELECT id FROM sessions WHERE id IN ({placeholders})"))
                .map_err(WriteError::Sqlite)?;
            let existing: Vec<String> = stmt
                .query_map(rusqlite::params_from_iter(unique_ids.iter()), |r| r.get(0))
                .map_err(WriteError::Sqlite)?
                .collect::<Result<_, _>>()
                .map_err(WriteError::Sqlite)?;
            if existing.is_empty() {
                return Ok(0);
            }
            let existing_placeholders = vec!["?"; existing.len()].join(",");
            removed_delegate_ids.borrow_mut().extend(delete_delegate_children(conn, &existing)?);
            conn.execute(
                &format!(
                    "UPDATE sessions SET parent_session_id = NULL \
                     WHERE parent_session_id IN ({existing_placeholders})"
                ),
                rusqlite::params_from_iter(existing.iter()),
            )?;
            conn.execute(
                &format!("DELETE FROM messages WHERE session_id IN ({existing_placeholders})"),
                rusqlite::params_from_iter(existing.iter()),
            )?;
            conn.execute(
                &format!("DELETE FROM sessions WHERE id IN ({existing_placeholders})"),
                rusqlite::params_from_iter(existing.iter()),
            )?;
            crate::crud::delete_unreferenced_system_prompts(conn)?;
            removed_ids.borrow_mut().extend(existing.iter().cloned());
            Ok(existing.len() as i64)
        };
        let count = self.execute_write(&f, None)?;
        if let Some(dir) = sessions_dir {
            for sid in removed_delegate_ids.borrow().iter() {
                remove_session_files(dir, sid);
            }
            for sid in removed_ids.borrow().iter() {
                remove_session_files(dir, sid);
            }
        }
        Ok(count)
    }

    /// Delete every empty, ended, non-archived session.
    ///
    /// PARITY: SessionDB.delete_empty_sessions @ b9aa928 (8569–8640)
    pub fn delete_empty_sessions(&self, sessions_dir: Option<&Path>) -> Result<i64, WriteError> {
        let removed_ids: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
        let f = |conn: &Connection| -> Result<i64, WriteError> {
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM sessions \
                     WHERE message_count = 0 AND ended_at IS NOT NULL AND archived = 0",
                )
                .map_err(WriteError::Sqlite)?;
            let session_ids: Vec<String> = stmt
                .query_map([], |r| r.get(0))
                .map_err(WriteError::Sqlite)?
                .collect::<Result<_, _>>()
                .map_err(WriteError::Sqlite)?;
            if session_ids.is_empty() {
                return Ok(0);
            }
            let placeholders = vec!["?"; session_ids.len()].join(",");
            conn.execute(
                &format!(
                    "UPDATE sessions SET parent_session_id = NULL \
                     WHERE parent_session_id IN ({placeholders})"
                ),
                rusqlite::params_from_iter(session_ids.iter()),
            )?;
            for sid in &session_ids {
                conn.execute(
                    "DELETE FROM messages WHERE session_id = ?",
                    rusqlite::params![sid],
                )?;
                conn.execute("DELETE FROM sessions WHERE id = ?", rusqlite::params![sid])?;
                removed_ids.borrow_mut().push(sid.clone());
            }
            crate::crud::delete_unreferenced_system_prompts(conn)?;
            Ok(session_ids.len() as i64)
        };
        let count = self.execute_write(&f, None)?;
        if let Some(dir) = sessions_dir {
            for sid in removed_ids.borrow().iter() {
                remove_session_files(dir, sid);
            }
        }
        Ok(count)
    }

    /// Permanently clear bare tool-call marker content (e.g. `[memory]`)
    /// left by pre-#78148 sessions.
    ///
    /// PARITY: SessionDB.purge_stale_tool_call_markers @ b9aa928
    /// (8974–9038)
    pub fn purge_stale_tool_call_markers(
        &self,
        dry_run: bool,
        backup: bool,
    ) -> Result<Value, WriteError> {
        fn find_affected(conn: &Connection) -> Result<Vec<i64>, WriteError> {
            let mut stmt = conn
                .prepare(
                    "SELECT id, content FROM messages \
                     WHERE role = 'assistant' AND tool_calls IS NOT NULL AND tool_calls != ''",
                )
                .map_err(WriteError::Sqlite)?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))
                })
                .map_err(WriteError::Sqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(WriteError::Sqlite)?;
            let mut affected = Vec::new();
            for (id, content) in rows {
                if let Some(content) = content {
                    if STALE_TOOL_CALL_MARKER_RE.is_match(content.trim()) {
                        affected.push(id);
                    }
                }
            }
            Ok(affected)
        }

        let conn = self.writer_conn();
        let affected_ids = find_affected(&conn)?;
        if dry_run {
            return Ok(json!({
                "dry_run": true,
                "rows_affected": affected_ids.len(),
                "row_ids": affected_ids,
                "backup_path": Value::Null,
            }));
        }
        if affected_ids.is_empty() {
            return Ok(json!({
                "dry_run": false,
                "rows_affected": 0,
                "row_ids": [],
                "backup_path": Value::Null,
            }));
        }

        let backup_path: Option<PathBuf> = if backup { self.vacuum_into_backup()? } else { None };
        let _ = affected_ids; // pre-scan exists only for dry_run / empty short-circuit
        let f = |conn: &Connection| -> Result<Vec<i64>, WriteError> {
            let ids = find_affected(conn)?;
            if !ids.is_empty() {
                let placeholders = vec!["?"; ids.len()].join(",");
                conn.execute(
                    &format!("UPDATE messages SET content = '' WHERE id IN ({placeholders})"),
                    rusqlite::params_from_iter(ids.iter()),
                )?;
            }
            Ok(ids)
        };
        let affected = self.execute_write(&f, None)?;
        Ok(json!({
            "dry_run": false,
            "rows_affected": affected.len(),
            "row_ids": affected,
            "backup_path": backup_path.map(|p| p.to_string_lossy().into_owned()),
        }))
    }

    /// `VACUUM INTO` a timestamped full snapshot of the main DB file.
    /// Returns None when the live SessionDB has no addressable path on disk.
    fn vacuum_into_backup(&self) -> Result<Option<PathBuf>, WriteError> {
        let db_path = self.db_path.clone();
        let stamp = now_stamp_compact();
        let dest = db_path.with_file_name(format!(
            "{}.pre-clean-markers-backup-{}",
            db_path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "state.db".into()),
            stamp
        ));
        let conn = self.writer_conn();
        conn.execute("VACUUM INTO ?", rusqlite::params![dest.to_string_lossy()])
            .map_err(WriteError::Sqlite)?;
        Ok(Some(dest.clone()))
    }
    /// Retag legacy kanban worker rows from `cli` to `kanban`, gated per
    /// workspaces root.
    ///
    /// PARITY: SessionDB.retag_kanban_worker_sessions @ b9aa928 (9121–9145)
    pub fn retag_kanban_worker_sessions(&self, workspaces_root: &str) -> Result<i64, WriteError> {
        let prefix = workspaces_root.trim_end_matches(['/', '\\']).to_string();
        if prefix.is_empty() {
            return Ok(0);
        }
        let gate = format!("kanban_worker_source_retagged:{prefix}");
        if self.get_meta(&gate) == Some("1".to_string()) {
            return Ok(0);
        }
        let escaped = crate::common::escape_like(&prefix);
        let prefix2 = prefix.clone();
        let gate2 = gate.clone();
        let f = |conn: &Connection| -> Result<i64, WriteError> {
            // Read rowcount BEFORE set_meta reuses the statement cursor.
            let rowcount = conn.execute(
                "UPDATE sessions SET source = 'kanban' \
                 WHERE source = 'cli' AND (cwd = ? OR cwd LIKE ? ESCAPE '\\')",
                rusqlite::params![prefix2, format!("{escaped}/%")],
            )?;
            conn.execute(
                "INSERT INTO state_meta (key, value) VALUES (?, ?) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![gate2, "1"],
            )?;
            Ok(rowcount as i64)
        };
        self.execute_write(&f, None)
    }

    /// Database size in bytes as SQLite itself accounts for it
    /// (page_count * page_size), or None when the pragmas cannot be read.
    ///
    /// PARITY: SessionDB.logical_size_bytes @ b9aa928 (9669–9695)
    pub fn logical_size_bytes(&self) -> Option<i64> {
        let conn = self.writer_conn();
        let page_count: Option<i64> = conn
            .query_row("PRAGMA page_count", [], |r| r.get(0))
            .optional()
            .ok()
            .flatten();
        let page_size: Option<i64> = conn
            .query_row("PRAGMA page_size", [], |r| r.get(0))
            .optional()
            .ok()
            .flatten();
        match (page_count, page_size) {
            (Some(pc), Some(ps)) => Some(pc * ps),
            _ => None,
        }
    }

    /// Run VACUUM to reclaim disk space after large deletes. Returns the
    /// number of FTS indexes optimized first (0 if the merge failed).
    ///
    /// PARITY: SessionDB.vacuum @ b9aa928 (9697–9720)
    pub fn vacuum(&self) -> Result<i64, WriteError> {
        // optimize_fts returns the merged-index count directly (the Python
        // version catches optimize_fts exceptions and falls back to 0; this
        // port's optimize_fts does not raise for merge failures).
        let optimized = self.optimize_fts();
        let conn = self.writer_conn();
        let _ = conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
                Ok((r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
            })
            .map_err(WriteError::Sqlite);
        conn.execute_batch("VACUUM").map_err(WriteError::Sqlite)?;
        Ok(optimized)
    }

    /// Idempotent auto-maintenance: prune inactive sessions + optional
    /// VACUUM. Never raises; returns a dict with error marker on failure.
    ///
    /// PARITY: SessionDB.maybe_auto_prune_and_vacuum @ b9aa928
    /// (9735–9823)
    #[allow(clippy::too_many_arguments)]
    pub fn maybe_auto_prune_and_vacuum(
        &self,
        retention_days: i64,
        min_interval_hours: i64,
        vacuum: bool,
        sessions_dir: Option<&Path>,
        min_vacuum_interval_days: i64,
    ) -> Result<Value, String> {
        let mut result = json!({"skipped": false, "pruned": 0, "vacuumed": false});
        let mut run = || -> Result<(), WriteError> {
            let now = now();
            if let Some(last_raw) = self.get_meta("last_auto_prune") {
                if let Ok(last_ts) = last_raw.parse::<f64>() {
                    if now - last_ts < (min_interval_hours * 3600) as f64 {
                        if let Some(o) = result.as_object_mut() {
                            o.insert("skipped".to_string(), json!(true));
                        }
                        return Ok(());
                    }
                }
            }
            let pruned = self
                .prune_sessions(
                    Some(retention_days as f64),
                    None,
                    sessions_dir,
                    PruneFilters::default(),
                )?
                as i64;
            if let Some(o) = result.as_object_mut() {
                o.insert("pruned".to_string(), json!(pruned));
            }
            let mut vacuum_due = true;
            if let Some(last_vacuum_raw) = self.get_meta("last_vacuum") {
                if let Ok(last_vacuum) = last_vacuum_raw.parse::<f64>() {
                    vacuum_due = now - last_vacuum >= (min_vacuum_interval_days * 86400) as f64;
                }
            }
            if vacuum && pruned > 0 && vacuum_due {
                self.vacuum()?;
                if let Some(o) = result.as_object_mut() {
                    o.insert("vacuumed".to_string(), json!(true));
                }
                self.set_meta("last_vacuum", &now.to_string())
                    .map_err(WriteError::Runtime)?;
            }
            self.set_meta("last_auto_prune", &now.to_string())
                .map_err(WriteError::Runtime)?;
            Ok(())
        };
        match run() {
            Ok(()) => Ok(result),
            Err(e) => {
                let mut r = result;
                if let Some(o) = r.as_object_mut() {
                    o.insert("error".to_string(), json!(e.to_string()));
                }
                Ok(r)
            }
        }
    }

    /// Idempotent auto-archive: soft-hide sessions idle for `idle_days`.
    /// Never raises; returns a dict with error marker on failure.
    ///
    /// PARITY: SessionDB.maybe_auto_archive @ b9aa928 (9825–9860)
    pub fn maybe_auto_archive(
        &self,
        idle_days: f64,
        min_interval_hours: i64,
        exclude_pinned: bool,
    ) -> Result<Value, String> {
        let mut result = json!({"skipped": false, "archived": 0});
        let mut run = || -> Result<(), WriteError> {
            let now = now();
            if let Some(last_raw) = self.get_meta("last_auto_archive") {
                if let Ok(last_ts) = last_raw.parse::<f64>() {
                    if now - last_ts < (min_interval_hours * 3600) as f64 {
                        if let Some(o) = result.as_object_mut() {
                            o.insert("skipped".to_string(), json!(true));
                        }
                        return Ok(());
                    }
                }
            }
            let archived = self.archive_stale_sessions(idle_days, exclude_pinned)? as i64;
            if let Some(o) = result.as_object_mut() {
                o.insert("archived".to_string(), json!(archived));
            }
            self.set_meta("last_auto_archive", &now.to_string())
                .map_err(WriteError::Runtime)?;
            Ok(())
        };
        match run() {
            Ok(()) => Ok(result),
            Err(e) => {
                let mut r = result;
                if let Some(o) = r.as_object_mut() {
                    o.insert("error".to_string(), json!(e.to_string()));
                }
                Ok(r)
            }
        }
    }
}

fn sort_for_stability(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

/// `%Y%m%d_%H%M%S`-style compact timestamp (UTC) for backup names.
fn now_stamp_compact() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    let rem = secs % 86400;
    let h = rem / 3600;
    let min = (rem % 3600) / 60;
    let s = rem % 60;
    // Civil-from-days (Howard Hinnant's algorithm) for y/m/d.
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}{m:02}{d:02}_{h:02}{min:02}{s:02}")
}
