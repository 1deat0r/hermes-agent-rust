//! Space reclamation — archive / prune surfaces.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   prune_empty_ghost_sessions        (5484–5520)
//!   set_session_archived              (5769–5824)
//!   _prune_filter_where               (8638–8776)
//!   list_prune_candidates             (8777–8816)
//!   archive_sessions                  (8818–8843)
//!   archive_stale_sessions            (8844–8892)
//!   prune_sessions                    (8890–8945)
//!   _remove_session_files             (8314–8340)
//!
//! `_prune_filter_where`'s TypeError-on-unknown-filter contract is
//! structurally impossible in Rust: every filter is an explicit field of
//! `PruneFilters`, so unknown names cannot be passed.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::common;
use crate::crud;
use crate::portability::cwd_prefix_clause;
use crate::state::{now, SessionDB, WriteError};

// ── filter surface ──────────────────────────────────────────────────────────

/// Shared WHERE-clause inputs for prune/archive selection. All fields AND
/// together; `archived` is tri-state: None = both, Some(true) = only
/// archived, Some(false) = only unarchived.
#[derive(Debug, Clone, Default)]
pub struct PruneFilters {
    pub last_active_before: Option<f64>,
    pub last_active_after: Option<f64>,
    pub started_before: Option<f64>,
    pub started_after: Option<f64>,
    pub source: Option<String>,
    pub title_like: Option<String>,
    pub end_reason: Option<String>,
    pub cwd_prefix: Option<String>,
    pub min_messages: Option<i64>,
    pub max_messages: Option<i64>,
    pub archived: Option<bool>,
    pub model_like: Option<String>,
    pub provider: Option<String>,
    pub user_id: Option<String>,
    pub chat_id: Option<String>,
    pub chat_type: Option<String>,
    pub branch_like: Option<String>,
    pub min_tokens: Option<i64>,
    pub max_tokens: Option<i64>,
    pub min_cost: Option<f64>,
    pub max_cost: Option<f64>,
    pub min_tool_calls: Option<i64>,
    pub max_tool_calls: Option<i64>,
}

/// Build the shared WHERE clause (referencing the `s` alias) plus params.
///
/// PARITY: hermes_state.py _prune_filter_where @ b9aa928 (8638–8776)
pub fn prune_filter_where(filters: &PruneFilters) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut clauses: Vec<String> = vec!["s.ended_at IS NOT NULL".to_string()];
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(v) = filters.last_active_before {
        clauses.push(
            "COALESCE(\
               (SELECT MAX(m.timestamp) FROM messages m \
                WHERE m.session_id = s.id),\
               s.started_at\
           ) < ?"
                .to_string(),
        );
        params.push(Box::new(v));
    }
    if let Some(v) = filters.last_active_after {
        clauses.push(
            "COALESCE(\
               (SELECT MAX(m.timestamp) FROM messages m \
                WHERE m.session_id = s.id),\
               s.started_at\
           ) >= ?"
                .to_string(),
        );
        params.push(Box::new(v));
    }
    if let Some(v) = filters.started_before {
        clauses.push("s.started_at < ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = filters.started_after {
        clauses.push("s.started_at >= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = &filters.source {
        clauses.push("s.source = ?".to_string());
        params.push(Box::new(v.clone()));
    }
    if let Some(v) = &filters.title_like {
        clauses.push("LOWER(COALESCE(s.title, '')) LIKE ? ESCAPE '\\'".to_string());
        params.push(Box::new(format!(
            "%{}%",
            common::escape_like(&v.to_lowercase())
        )));
    }
    if let Some(v) = &filters.end_reason {
        clauses.push("s.end_reason = ?".to_string());
        params.push(Box::new(v.clone()));
    }
    if let Some(v) = &filters.cwd_prefix {
        let (clause, clause_params) = cwd_prefix_clause(v);
        clauses.push(clause);
        params.extend(
            clause_params
                .into_iter()
                .map(|p| Box::new(p) as Box<dyn rusqlite::ToSql>),
        );
    }
    if let Some(v) = filters.min_messages {
        clauses.push("s.message_count >= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = filters.max_messages {
        clauses.push("s.message_count <= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = &filters.model_like {
        clauses.push("LOWER(COALESCE(s.model, '')) LIKE ? ESCAPE '\\'".to_string());
        params.push(Box::new(format!(
            "%{}%",
            common::escape_like(&v.to_lowercase())
        )));
    }
    if let Some(v) = &filters.provider {
        clauses.push("LOWER(COALESCE(s.billing_provider, '')) = ?".to_string());
        params.push(Box::new(v.to_lowercase()));
    }
    if let Some(v) = &filters.user_id {
        clauses.push("s.user_id = ?".to_string());
        params.push(Box::new(v.clone()));
    }
    if let Some(v) = &filters.chat_id {
        clauses.push("s.chat_id = ?".to_string());
        params.push(Box::new(v.clone()));
    }
    if let Some(v) = &filters.chat_type {
        clauses.push("s.chat_type = ?".to_string());
        params.push(Box::new(v.clone()));
    }
    if let Some(v) = &filters.branch_like {
        clauses.push("LOWER(COALESCE(s.git_branch, '')) LIKE ? ESCAPE '\\'".to_string());
        params.push(Box::new(format!(
            "%{}%",
            common::escape_like(&v.to_lowercase())
        )));
    }
    if let Some(v) = filters.min_tokens {
        clauses
            .push("(COALESCE(s.input_tokens, 0) + COALESCE(s.output_tokens, 0)) >= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = filters.max_tokens {
        clauses
            .push("(COALESCE(s.input_tokens, 0) + COALESCE(s.output_tokens, 0)) <= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = filters.min_cost {
        clauses.push("COALESCE(s.actual_cost_usd, s.estimated_cost_usd, 0) >= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = filters.max_cost {
        clauses.push("COALESCE(s.actual_cost_usd, s.estimated_cost_usd, 0) <= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = filters.min_tool_calls {
        clauses.push("COALESCE(s.tool_call_count, 0) >= ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = filters.max_tool_calls {
        clauses.push("COALESCE(s.tool_call_count, 0) <= ?".to_string());
        params.push(Box::new(v));
    }
    match filters.archived {
        Some(true) => clauses.push("s.archived = 1".to_string()),
        Some(false) => clauses.push("s.archived = 0".to_string()),
        None => {}
    }
    (clauses.join(" AND "), params)
}

// ── candidate row ───────────────────────────────────────────────────────────

/// One entry of `list_prune_candidates` (id/source/title/model/started_at/
/// last_active/ended_at/message_count/archived).
#[derive(Debug, Clone)]
pub struct PruneCandidate {
    pub id: String,
    pub source: String,
    pub title: Option<String>,
    pub model: Option<String>,
    pub started_at: f64,
    pub last_active: f64,
    pub ended_at: Option<f64>,
    pub message_count: i64,
    pub archived: bool,
}

impl SessionDB {
    /// Archive or unarchive a session (soft hide; the whole compression
    /// lineage flips as a unit). Returns True when >= 1 row updated.
    ///
    /// PARITY: SessionDB.set_session_archived @ b9aa928 (5769–5824)
    pub fn set_session_archived(
        &self,
        session_id: &str,
        archived: bool,
    ) -> Result<bool, WriteError> {
        let session_id = session_id.to_string();
        let flag: i64 = if archived { 1 } else { 0 };
        let f = |conn: &Connection| -> Result<bool, WriteError> {
            let rowcount = conn.execute(
                "WITH RECURSIVE \
                   ancestors(id) AS ( \
                     SELECT ? \
                     UNION \
                     SELECT parent.id \
                     FROM ancestors a \
                     JOIN sessions child ON child.id = a.id \
                     JOIN sessions parent ON parent.id = child.parent_session_id \
                     WHERE parent.end_reason = 'compression' \
                   ), \
                   descendants(id) AS ( \
                     SELECT ? \
                     UNION \
                     SELECT child.id \
                     FROM descendants d \
                     JOIN sessions parent ON parent.id = d.id \
                     JOIN sessions child ON child.parent_session_id = parent.id \
                     WHERE parent.end_reason = 'compression' \
                   ), \
                   lineage(id) AS ( \
                     SELECT id FROM ancestors \
                     UNION \
                     SELECT id FROM descendants \
                   ) \
                 UPDATE sessions \
                 SET archived = ? \
                 WHERE id IN (SELECT id FROM lineage)",
                rusqlite::params![session_id, session_id, flag],
            )?;
            Ok(rowcount > 0)
        };
        self.execute_write(&f, None)
    }

    /// Sessions a matching prune/archive call would touch (dry-run /
    /// pre-confirmation counts), oldest first.
    ///
    /// PARITY: SessionDB.list_prune_candidates @ b9aa928 (8777–8816)
    pub fn list_prune_candidates(
        &self,
        older_than_days: Option<f64>,
        source: Option<&str>,
        mut filters: PruneFilters,
    ) -> Result<Vec<PruneCandidate>, WriteError> {
        if filters.last_active_before.is_none() && filters.started_before.is_none() {
            if let Some(days) = older_than_days {
                filters.last_active_before = Some(now() - days * 86400.0);
            }
        }
        if let Some(src) = source {
            filters.source = Some(src.to_string());
        }
        let (where_clause, where_params) = prune_filter_where(&filters);
        let sql = format!(
            "SELECT s.id, s.source, s.title, s.model, s.started_at, \
                COALESCE(\
                    (SELECT MAX(m.timestamp) FROM messages m \
                     WHERE m.session_id = s.id),\
                    s.started_at\
                ) AS last_active, \
                s.ended_at, s.message_count, s.archived \
             FROM sessions s WHERE {where_clause} \
             ORDER BY last_active ASC, s.started_at ASC"
        );
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(where_params.iter().map(|p| p as &dyn rusqlite::ToSql)),
                |r| {
                    Ok(PruneCandidate {
                        id: r.get("id")?,
                        source: r.get("source")?,
                        title: r.get("title")?,
                        model: r.get("model")?,
                        started_at: r.get("started_at")?,
                        last_active: r.get("last_active")?,
                        ended_at: r.get("ended_at")?,
                        message_count: r.get("message_count")?,
                        archived: r.get::<_, i64>("archived")? != 0,
                    })
                },
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows)
    }

    /// Bulk-archive (soft-hide) every session matching the filters, archiving
    /// each match's whole compression lineage. Returns count matched;
    /// `archived` defaults to False so repeat runs are idempotent no-ops.
    ///
    /// PARITY: SessionDB.archive_sessions @ b9aa928 (8818–8843)
    pub fn archive_sessions(
        &self,
        older_than_days: Option<f64>,
        source: Option<&str>,
        mut filters: PruneFilters,
    ) -> Result<usize, WriteError> {
        if filters.archived.is_none() {
            filters.archived = Some(false);
        }
        let rows = self.list_prune_candidates(older_than_days, source, filters)?;
        for row in &rows {
            self.set_session_archived(&row.id, true)?;
        }
        Ok(rows.len())
    }

    /// Archive every session untouched for at least `idle_days` days
    /// (real recency, not creation time). Guards: pinned=0 when
    /// `exclude_pinned`, archived=0, lineage tips only. Never raises for
    /// empty/non-positive `idle_days`.
    ///
    /// PARITY: SessionDB.archive_stale_sessions @ b9aa928 (8844–8892)
    pub fn archive_stale_sessions(
        &self,
        idle_days: f64,
        exclude_pinned: bool,
    ) -> Result<usize, WriteError> {
        if idle_days < 0.0 {
            return Ok(0);
        }
        let cutoff = now() - idle_days * 86400.0;
        let pin_clause = if exclude_pinned {
            "AND s.pinned = 0"
        } else {
            ""
        };
        let last_active = common::_sql_session_last_active("s");
        let conn = self.writer_conn();
        let sql = format!(
            "SELECT s.id FROM sessions s \
             WHERE s.archived = 0 \
               AND COALESCE(s.end_reason, '') <> 'compression' \
               {pin_clause} \
               AND {last_active} < ? \
             ORDER BY s.started_at ASC"
        );
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let ids: Vec<String> = stmt
            .query_map(rusqlite::params![cutoff], |r| r.get(0))
            .map_err(WriteError::Sqlite)?
            .collect::<Result<_, _>>()
            .map_err(WriteError::Sqlite)?;
        drop(stmt);
        drop(conn);
        let mut archived = 0usize;
        for sid in &ids {
            self.set_session_archived(sid, true)?;
            archived += 1;
        }
        Ok(archived)
    }

    /// Delete sessions matching the filters. Ended sessions only; child
    /// sessions outside the window are orphaned (parent → NULL) rather than
    /// cascade-deleted; on-disk transcript files are removed outside the DB
    /// transaction when `sessions_dir` is provided.
    ///
    /// PARITY: SessionDB.prune_sessions @ b9aa928 (8890–8945)
    pub fn prune_sessions(
        &self,
        older_than_days: Option<f64>,
        source: Option<&str>,
        sessions_dir: Option<&Path>,
        mut filters: PruneFilters,
    ) -> Result<usize, WriteError> {
        if filters.last_active_before.is_none() && filters.started_before.is_none() {
            if let Some(days) = older_than_days {
                filters.last_active_before = Some(now() - days * 86400.0);
            }
        }
        if let Some(src) = source {
            filters.source = Some(src.to_string());
        }
        let (where_clause, where_params) = prune_filter_where(&filters);
        let sql = format!("SELECT s.id FROM sessions s WHERE {where_clause}");
        let removed_ids: Vec<String> = {
            let conn = self.writer_conn();
            let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
            let ids: Vec<String> = stmt
                .query_map(
                    rusqlite::params_from_iter(
                        where_params.iter().map(|p| p as &dyn rusqlite::ToSql),
                    ),
                    |r| r.get(0),
                )
                .map_err(WriteError::Sqlite)?
                .collect::<Result<_, _>>()
                .map_err(WriteError::Sqlite)?;
            drop(stmt);
            ids
        };
        if removed_ids.is_empty() {
            return Ok(0);
        }
        let count = {
            let p: Vec<&str> = removed_ids.iter().map(|s| s.as_str()).collect();
            let f = |conn: &Connection| -> Result<usize, WriteError> {
                let placeholders = p.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                // Orphan any sessions whose parent is about to be deleted.
                conn.execute(
                    &format!(
                        "UPDATE sessions SET parent_session_id = NULL \
                         WHERE parent_session_id IN ({placeholders})"
                    ),
                    rusqlite::params_from_iter(p.iter().copied()),
                )?;
                for sid in &p {
                    conn.execute(
                        "DELETE FROM messages WHERE session_id = ?",
                        rusqlite::params![sid],
                    )?;
                    conn.execute("DELETE FROM sessions WHERE id = ?", rusqlite::params![sid])?;
                }
                crud::delete_unreferenced_system_prompts(conn)?;
                Ok(p.len())
            };
            self.execute_write(&f, None)?
        };
        // Clean up on-disk files outside the DB transaction.
        if let Some(dir) = sessions_dir {
            for sid in &removed_ids {
                remove_session_files(dir, sid);
            }
        }
        Ok(count)
    }

    /// Remove empty TUI ghost sessions (no messages, no title, ended, older
    /// than 24h), plus any on-disk session files for them.
    ///
    /// PARITY: SessionDB.prune_empty_ghost_sessions @ b9aa928 (5484–5520)
    pub fn prune_empty_ghost_sessions(
        &self,
        sessions_dir: Option<&Path>,
    ) -> Result<usize, WriteError> {
        let cutoff = now() - 86400.0;
        let removed_ids: Vec<String> = {
            let conn = self.writer_conn();
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM sessions \
                     WHERE source = 'tui' \
                       AND title IS NULL \
                       AND ended_at IS NOT NULL \
                       AND started_at < ? \
                       AND NOT EXISTS ( \
                           SELECT 1 FROM messages WHERE messages.session_id = sessions.id \
                       )",
                )
                .map_err(WriteError::Sqlite)?;
            let ids: Vec<String> = stmt
                .query_map(rusqlite::params![cutoff], |r| r.get(0))
                .map_err(WriteError::Sqlite)?
                .collect::<Result<_, _>>()
                .map_err(WriteError::Sqlite)?;
            drop(stmt);
            ids
        };
        if removed_ids.is_empty() {
            return Ok(0);
        }
        let count = {
            let p: Vec<&str> = removed_ids.iter().map(|s| s.as_str()).collect();
            let f = |conn: &Connection| -> Result<usize, WriteError> {
                let placeholders = p.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                conn.execute(
                    &format!("DELETE FROM sessions WHERE id IN ({placeholders})"),
                    rusqlite::params_from_iter(p.iter().copied()),
                )?;
                crud::delete_unreferenced_system_prompts(conn)?;
                Ok(p.len())
            };
            self.execute_write(&f, None)?
        };
        if let Some(dir) = sessions_dir {
            for sid in &removed_ids {
                remove_session_files(dir, sid);
            }
        }
        Ok(count)
    }
}

/// Best-effort removal of on-disk transcript files for a session
/// (`{sid}.json`, `{sid}.jsonl`, `request_dump_{sid}_*.json`). Never raises.
///
/// PARITY: hermes_state.py _remove_session_files @ b9aa928 (8314–8340)
pub fn remove_session_files(sessions_dir: &Path, session_id: &str) {
    for suffix in [".json", ".jsonl"] {
        let p: PathBuf = sessions_dir.join(format!("{session_id}{suffix}"));
        let _ = std::fs::remove_file(&p);
    }
    if let Ok(entries) = std::fs::read_dir(sessions_dir) {
        let prefix = format!("request_dump_{}_", session_id);
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name_str) = name.to_str() else {
                continue;
            };
            if name_str.starts_with(&prefix) && name_str.ends_with(".json") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}
