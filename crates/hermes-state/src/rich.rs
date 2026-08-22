//! Surface read helpers — rich session listings, gateway scopes, read/pin
//! state, activity heartbeats, session counts, and id search.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   touch_session_activity                 (4489–4534)
//!   clear_session_activity_labels          (4536–4582)
//!   get_session_activity                   (4584–4616)
//!   list_gateway_sessions                  (3487–3530)
//!   set_session_pinned                     (5819–5869)
//!   set_session_read / session_unread      (5871–5952)
//!   get_compression_tip                    (6023–6088)
//!   list_sessions_rich                     (6094–6454)
//!   session_count / session_count_ge /
//!     session_count_by_source              (8079–8200)
//!   count_empty_sessions                   (8544–8565)
//! hermes_state_search.py @ b9aa928 —
//!   search_sessions_by_id                  (2101–2160)

use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;

use crate::common::{
    _preview_raw_select, _shape_preview, _sql_session_last_active,
    _sql_session_last_active_by_id, _listable_child_sql,
};
use crate::crud::fold_session_dict;
use crate::portability::{compact_session_cols, cwd_prefix_clause};
use crate::state::{now, SessionDB, WriteError};

static COMPACT_NEEDLE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\W_]+").expect("compact needle regex"));

fn delegate_from_json_sql(col: &str) -> String {
    format!("json_extract(COALESCE({col}, '{{}}'), '$._delegate_from')")
}

fn like_pattern(needle: &str) -> String {
    format!("%{}%", crate::common::escape_like(needle))
}

/// Owned sqlite parameter that can be cloned (the WHERE params are bound
/// twice by the CTE query and once by the pinned back-fill).
#[derive(Debug, Clone)]
enum P {
    S(String),
    I(i64),
}

impl rusqlite::ToSql for P {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(match self {
            P::S(v) => rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Text(v.as_bytes()),
            ),
            P::I(v) => rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Integer(*v),
            ),

        })
    }
}

/// Materialize cloneable params into boxed ToSql for a query call.
fn boxed_params(ps: &[P]) -> Vec<Box<dyn rusqlite::ToSql>> {
    ps.iter().map(|p| Box::new(p.clone()) as Box<dyn rusqlite::ToSql>).collect()
}

/// Holds the ordering/window decisions of `list_sessions_rich`.
#[derive(Debug, Clone)]
pub struct RichListParams {
    pub source: Option<String>,
    pub sources: Vec<String>,
    pub exclude_sources: Vec<String>,
    pub cwd_prefix: Option<String>,
    pub limit: i64,
    pub offset: i64,
    pub include_children: bool,
    pub min_message_count: i64,
    pub project_compression_tips: bool,
    pub order_by_last_active: bool,
    pub include_archived: bool,
    pub archived_only: bool,
    pub id_query: Option<String>,
    pub search_query: Option<String>,
    pub compact_rows: bool,
    pub include_pinned: bool,
    pub session_key: Option<String>,
}

impl Default for RichListParams {
    fn default() -> Self {
        RichListParams {
            source: None,
            sources: Vec::new(),
            exclude_sources: Vec::new(),
            cwd_prefix: None,
            limit: 20,
            offset: 0,
            include_children: false,
            min_message_count: 0,
            project_compression_tips: true,
            order_by_last_active: false,
            include_archived: false,
            archived_only: false,
            id_query: None,
            search_query: None,
            compact_rows: false,
            include_pinned: false,
            session_key: None,
        }
    }
}

impl SessionDB {
    /// Stamp durable mid-turn session activity (observation-only); never
    /// moves `last_activity_at` backwards.
    ///
    /// PARITY: SessionDB.touch_session_activity @ b9aa928 (4489–4534)
    pub fn touch_session_activity(
        &self,
        session_id: &str,
        ts: Option<f64>,
        description: Option<&str>,
        provenance: Option<&crate::activity::ActivityProvenance>,
    ) -> Result<(), WriteError> {
        if session_id.is_empty() {
            return Ok(());
        }
        let when = ts.unwrap_or_else(now);
        let desc = crate::activity::bound_activity_description(description);
        let prov = crate::activity::normalize_activity_provenance(
            provenance.map(|p| p.as_str()),
        )
        .as_str()
        .to_string();
        let sid = session_id.to_string();

        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET \
                 last_activity_at = ?, \
                 last_activity_description = ?, \
                 last_activity_provenance = ? \
                 WHERE id = ? AND (last_activity_at IS NULL OR last_activity_at < ?)",
                rusqlite::params![when, desc, prov, sid, when],
            )?;
            Ok(())
        };
        // Observation-only write: never ride the full write-patience budget
        // (#76354 review S1).
        self.execute_write(&f, Some(Self::ACTIVITY_WRITE_PATIENCE_S))
    }

    /// Clear mid-turn activity labels after a turn ends. Keeps
    /// `last_activity_at` intact so idle/watchdog clocks stay continuous.
    ///
    /// PARITY: SessionDB.clear_session_activity_labels @ b9aa928 (4536–4582)
    pub fn clear_session_activity_labels(&self, session_id: &str) -> Result<(), WriteError> {
        if session_id.is_empty() {
            return Ok(());
        }
        let unknown = crate::activity::ActivityProvenance::Unknown;

        // No-op fast path: skip the transaction when there is nothing to
        // clear. Read-only, no write lock.
        let conn = self.writer_conn();
        let row: Option<(Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT last_activity_description, last_activity_provenance \
                 FROM sessions WHERE id = ?",
                rusqlite::params![session_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        drop(conn);
        if let Some((desc, prov)) = row {
            let desc_empty = desc.as_deref().unwrap_or("").is_empty();
            let prov_empty = prov.as_deref().map(str::is_empty).unwrap_or(true);
            let prov_unknown = prov.as_deref() == Some(unknown.as_str());
            if desc_empty && (prov_empty || prov_unknown) {
                return Ok(());
            }
        }

        let sid = session_id.to_string();
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET \
                 last_activity_description = ?, \
                 last_activity_provenance = ? \
                 WHERE id = ?",
                rusqlite::params!["", unknown.as_str(), sid],
            )?;
            Ok(())
        };
        self.execute_write(&f, Some(Self::ACTIVITY_WRITE_PATIENCE_S))
    }

    /// Return the durable activity snapshot for `session_id`, or None.
    ///
    /// PARITY: SessionDB.get_session_activity @ b9aa928 (4584–4616)
    pub fn get_session_activity(&self, session_id: &str) -> Result<Option<Value>, WriteError> {
        if session_id.is_empty() {
            return Ok(None);
        }
        let conn = self.writer_conn();
        let row: Option<(Option<f64>, Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT last_activity_at, last_activity_description, \
                        last_activity_provenance \
                 FROM sessions WHERE id = ?",
                rusqlite::params![session_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(row.map(|(at, desc, prov)| {
            crate::activity::build_activity_snapshot(at, desc.as_deref(), prov.as_deref(), None)
        }))
    }

    /// List gateway sessions (rows with a session_key): the newest row per
    /// session_key, one live mapping per routing key.
    ///
    /// PARITY: SessionDB.list_gateway_sessions @ b9aa928 (3487–3530)
    pub fn list_gateway_sessions(
        &self,
        platform: Option<&str>,
        active_only: bool,
    ) -> Result<Vec<Value>, WriteError> {
        // Full rows carry token/cost totals — drain queued deltas first.
        let _ = self.flush_token_counts(5.0);
        let last_active = _sql_session_last_active("sessions");
        let mut sql = format!(
            "SELECT sessions.*, \
                COALESCE(sp.prompt, sessions.system_prompt) AS _system_prompt_resolved, \
                {last_active} AS last_active \
             FROM sessions \
             LEFT JOIN system_prompts sp ON sp.hash = sessions.system_prompt_hash \
             WHERE session_key IS NOT NULL \
               AND started_at = ( \
                   SELECT MAX(s2.started_at) FROM sessions s2 \
                   WHERE s2.session_key = sessions.session_key \
               )"
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(platform) = platform {
            sql += " AND LOWER(source) = LOWER(?)";
            params.push(Box::new(platform.to_string()));
        }
        if active_only {
            sql += " AND ended_at IS NULL";
        }
        sql += " ORDER BY last_active DESC";
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|p| p as &dyn rusqlite::ToSql)),
                crate::portability::row_to_value,
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows.into_iter().map(fold_session_dict).collect())
    }

    /// Pin or unpin a session (and its whole compression lineage).
    ///
    /// PARITY: SessionDB.set_session_pinned @ b9aa928 (5819–5869)
    pub fn set_session_pinned(&self, session_id: &str, pinned: bool) -> Result<bool, WriteError> {
        let session_id = session_id.to_string();
        let flag: i64 = if pinned { 1 } else { 0 };
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
                 SET pinned = ? \
                 WHERE id IN (SELECT id FROM lineage)",
                rusqlite::params![session_id, session_id, flag],
            )?;
            Ok(rowcount > 0)
        };
        self.execute_write(&f, None)
    }

    /// Mark a session read or unread (and its whole compression lineage).
    ///
    /// PARITY: SessionDB.set_session_read @ b9aa928 (5871–5946)
    pub fn set_session_read(&self, session_id: &str, read: bool) -> Result<bool, WriteError> {
        let session_id = session_id.to_string();
        let watermark: f64 = if read { now() } else { 0.0 };
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
                 SET last_read_at = ? \
                 WHERE id IN (SELECT id FROM lineage)",
                rusqlite::params![session_id, session_id, watermark],
            )?;
            Ok(rowcount > 0)
        };
        self.execute_write(&f, None)
    }

    /// Derive unread from a row's watermark and activity. NULL watermark =
    /// never tracked = read.
    ///
    /// PARITY: SessionDB.session_unread @ b9aa928 (5932–5952)
    pub fn session_unread(row: &Value) -> bool {
        match row.get("last_read_at") {
            None => false,
            Some(Value::Null) => false,
            Some(v) => {
                let last_read = v.as_f64().unwrap_or(0.0);
                let last_active = row
                    .get("last_active")
                    .or_else(|| row.get("started_at"))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                last_active > last_read
            }
        }
    }

    /// Walk the compression-continuation chain forward; return the tip id
    /// (or the input id when no continuation exists).
    ///
    /// PARITY: SessionDB.get_compression_tip @ b9aa928 (6023–6088)
    pub fn get_compression_tip(&self, session_id: &str) -> Result<String, WriteError> {
        let mut current = session_id.to_string();
        let mut seen: HashSet<String> = [current.clone()].into_iter().collect();
        let last_active = _sql_session_last_active("child");
        for _ in 0..100 {
            let conn = self.writer_conn();
            let row = conn
                .query_row(
                    &format!(
                        "SELECT child.id \
                         FROM sessions parent \
                         JOIN sessions child ON child.parent_session_id = parent.id \
                         WHERE parent.id = ? \
                           AND parent.end_reason = 'compression' \
                           AND json_extract(COALESCE(child.model_config, '{{}}'), '$._branched_from') IS NULL \
                           AND json_extract(COALESCE(child.model_config, '{{}}'), '$._delegate_from') IS NULL \
                           AND COALESCE(child.source, '') != 'tool' \
                         ORDER BY \
                           CASE \
                             WHEN child.end_reason = 'compression' THEN 0 \
                             WHEN child.ended_at IS NULL THEN 1 \
                             ELSE 2 \
                           END, \
                           {last_active} DESC, \
                           child.started_at DESC, \
                           child.id DESC \
                         LIMIT 1"
                    ),
                    rusqlite::params![current],
                    |r| r.get::<_, String>(0),
                )
                .optional()
                .map_err(WriteError::Sqlite)?;
            drop(conn);
            match row {
                None => return Ok(current),
                Some(child_id) => {
                    if child_id.is_empty() || seen.contains(&child_id) {
                        return Ok(current);
                    }
                    seen.insert(child_id.clone());
                    current = child_id;
                }
            }
        }
        Ok(current)
    }

    /// List sessions with preview (first user message) and last-active
    /// timestamp, with compression projection, pinned back-fill, and derived
    /// read state.
    ///
    /// PARITY: SessionDB.list_sessions_rich @ b9aa928 (6094–6454)
    pub fn list_sessions_rich(
        &self,
        p: &RichListParams,
    ) -> Result<Vec<Value>, WriteError> {
        // Rows carry token/cost totals — drain queued deltas first.
        let _ = self.flush_token_counts(5.0);

        let empty: Vec<String> = Vec::new();
        let source_opt = p.source.as_deref();
        let sources: &[String] = if p.sources.is_empty() { &empty } else { &p.sources };

        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<P> = Vec::new();

        if !p.include_children {
            where_clauses.push(_listable_child_sql());
            where_clauses.push(format!("{} IS NULL", delegate_from_json_sql("s.model_config")));
        }
        let include_sources: Vec<String> = match source_opt {
            Some(s) => vec![s.to_string()],
            None => sources.to_vec(),
        };
        if !include_sources.is_empty() {
            let placeholders = include_sources.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            where_clauses.push(format!("s.source IN ({placeholders})"));
            for s in &include_sources {
                params.push(P::S(s.clone()));
            }
        }
        if let Some(key) = &p.session_key {
            where_clauses.push("s.session_key = ?".to_string());
            params.push(P::S(key.clone()));
        }
        if !p.exclude_sources.is_empty() {
            let placeholders = p.exclude_sources.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            where_clauses.push(format!("s.source NOT IN ({placeholders})"));
            for s in &p.exclude_sources {
                params.push(P::S(s.clone()));
            }
        }
        if let Some(prefix) = &p.cwd_prefix {
            let (clause, clause_params) = cwd_prefix_clause(prefix);
            where_clauses.push(clause);
            for cp in clause_params {
                params.push(P::S(cp));
            }
        }
        if p.min_message_count > 0 {
            where_clauses.push("s.message_count >= ?".to_string());
            params.push(P::I(p.min_message_count));
        }
        if p.archived_only {
            where_clauses.push("s.archived = 1".to_string());
        } else if !p.include_archived {
            where_clauses.push("s.archived = 0".to_string());
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };
        // Snapshot before LIMIT/OFFSET extension — the pinned back-fill reuses
        // the base WHERE params.
        let base_where_params = params.clone();

        let prompt_select = if p.compact_rows {
            String::new()
        } else {
            ", COALESCE(sp.prompt, s.system_prompt) AS _system_prompt_resolved".to_string()
        };
        let prompt_join = if p.compact_rows {
            String::new()
        } else {
            "LEFT JOIN system_prompts sp ON sp.hash = s.system_prompt_hash".to_string()
        };
        let sel = if p.compact_rows {
            compact_session_cols()
        } else {
            "s.*".to_string()
        };
        let preview_select = _preview_raw_select();

        let rows: Vec<Value> = if p.order_by_last_active {
            let id_needle = p.id_query.as_deref().unwrap_or("").trim().to_lowercase();
            let search_needle = p.search_query.as_deref().unwrap_or("").trim().to_lowercase();
            let mut filter_clauses: Vec<String> = Vec::new();
            let mut id_params: Vec<P> = Vec::new();

            if !id_needle.is_empty() {
                filter_clauses.push(
                    "EXISTS (SELECT 1 FROM chain cq \
                     WHERE cq.root_id = s.id \
                       AND LOWER(cq.cur_id) LIKE ? ESCAPE '\\')"
                        .to_string(),
                );
                id_params.push(P::S(like_pattern(&id_needle)));
            }
            if !search_needle.is_empty() {
                let compact_needle = COMPACT_NEEDLE_RE.replace_all(&search_needle, "").to_string();
                let compact_sql = "REPLACE(REPLACE(REPLACE(REPLACE(LOWER(COALESCE({0}, '')), \
                     '-', ''), '_', ''), '.', ''), ' ', '')";
                let mut search_clause = "EXISTS (SELECT 1 FROM chain cq \
                     JOIN sessions cs ON cs.id = cq.cur_id \
                     WHERE cq.root_id = s.id \
                       AND (LOWER(COALESCE(cs.title, '')) LIKE ? ESCAPE '\\' \
                       OR LOWER(cq.cur_id) LIKE ? ESCAPE '\\'".to_string();
                id_params.push(P::S(like_pattern(&search_needle)));
                id_params.push(P::S(like_pattern(&search_needle)));
                if !compact_needle.is_empty() {
                    search_clause += &format!(
                        " OR {} LIKE ? ESCAPE '\\'",
                        compact_sql.replace("{0}", "cs.title")
                    );
                    id_params.push(P::S(like_pattern(&compact_needle)));
                }
                filter_clauses.push(search_clause + "))");
            }
            let outer_where = if filter_clauses.is_empty() {
                where_sql.clone()
            } else {
                let combined = filter_clauses.join(" AND ");
                if where_sql.is_empty() {
                    format!("WHERE {combined}")
                } else {
                    format!("{where_sql} AND {combined}")
                }
            };
            let query = format!(
                "WITH RECURSIVE chain(root_id, cur_id) AS ( \
                     SELECT s.id, s.id FROM sessions s {where_sql} \
                     UNION ALL \
                     SELECT c.root_id, child.id \
                     FROM chain c \
                     JOIN sessions parent ON parent.id = c.cur_id \
                     JOIN sessions child ON child.parent_session_id = c.cur_id \
                     WHERE parent.end_reason = 'compression' \
                       AND json_extract(COALESCE(child.model_config, '{{}}'), '$._branched_from') IS NULL \
                       AND json_extract(COALESCE(child.model_config, '{{}}'), '$._delegate_from') IS NULL \
                       AND COALESCE(child.source, '') != 'tool' \
                 ), \
                 chain_max AS ( \
                     SELECT root_id, \
                         MAX({}) AS effective_last_active \
                     FROM chain \
                     GROUP BY root_id \
                 ) \
                 SELECT {}{},\
                     COALESCE(\
                         (SELECT {} \
                          FROM messages m \
                          WHERE m.session_id = s.id AND m.role = 'user' AND m.content IS NOT NULL \
                          ORDER BY m.timestamp, m.id LIMIT 1),\
                         ''\
                     ) AS _preview_raw,\
                     {} AS last_active,\
                     COALESCE(cm.effective_last_active, s.started_at) AS _effective_last_active \
                 FROM sessions s \
                 LEFT JOIN chain_max cm ON cm.root_id = s.id \
                 {} \
                 {} \
                 ORDER BY _effective_last_active DESC, s.started_at DESC, s.id DESC \
                 LIMIT ? OFFSET ?",
                _sql_session_last_active_by_id("cur_id"),
                sel,
                prompt_select,
                preview_select,
                _sql_session_last_active("s"),
                prompt_join,
                outer_where,
            );
            // WHERE params apply twice (CTE seed + outer select); id filter
            // only applies to the outer select.
            let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            all_params.extend(boxed_params(&params));
            all_params.extend(boxed_params(&params));
            all_params.extend(boxed_params(&id_params));
            all_params.push(Box::new(P::I(p.limit)));
            all_params.push(Box::new(P::I(p.offset)));
            self.query_values(&query, &all_params)?
        } else {
            let query = format!(
                "SELECT {}{},\
                     COALESCE(\
                         (SELECT {} \
                          FROM messages m \
                          WHERE m.session_id = s.id AND m.role = 'user' AND m.content IS NOT NULL \
                          ORDER BY m.timestamp, m.id LIMIT 1),\
                         ''\
                     ) AS _preview_raw,\
                     {} AS last_active \
                 FROM sessions s \
                 {} \
                 {} \
                 ORDER BY s.started_at DESC \
                 LIMIT ? OFFSET ?",
                sel,
                prompt_select,
                preview_select,
                _sql_session_last_active("s"),
                prompt_join,
                where_sql,
            );
            let mut boxed = boxed_params(&params);
            boxed.push(Box::new(P::I(p.limit)));
            boxed.push(Box::new(P::I(p.offset)));
            self.query_values(&query, &boxed)?
        };

        let mut sessions: Vec<Value> = Vec::new();
        for row in rows {
            let mut s = fold_session_dict(row);
            let preview_raw = s
                .as_object_mut()
                .and_then(|o| o.remove("_preview_raw"))
                .and_then(|v| v.as_str().map(|x| x.to_string()))
                .unwrap_or_default();
            if let Some(obj) = s.as_object_mut() {
                obj.insert("preview".to_string(), Value::String(_shape_preview(preview_raw)));
                obj.remove("_effective_last_active");
            }
            sessions.push(s);
        }

        // Back-fill pinned conversations the page missed (before projection).
        if p.include_pinned {
            let mut seen_ids: HashSet<String> =
                sessions.iter().filter_map(|s| s.get("id").and_then(Value::as_str).map(|x| x.to_string())).collect();
            let pinned_where = if where_sql.is_empty() {
                "WHERE s.pinned = 1".to_string()
            } else {
                format!("{where_sql} AND s.pinned = 1")
            };
            let pinned_query = format!(
                "SELECT {}{},\
                     COALESCE(\
                         (SELECT {} \
                          FROM messages m \
                          WHERE m.session_id = s.id AND m.role = 'user' AND m.content IS NOT NULL \
                          ORDER BY m.timestamp, m.id LIMIT 1),\
                         ''\
                     ) AS _preview_raw,\
                     COALESCE(\
                         (SELECT MAX(m2.timestamp) FROM messages m2 WHERE m2.session_id = s.id),\
                         s.started_at\
                     ) AS last_active \
                 FROM sessions s \
                 {} \
                 {} \
                 ORDER BY s.started_at DESC",
                sel,
                prompt_select,
                preview_select,
                prompt_join,
                pinned_where,
            );
            let pinned_rows = self.query_values(&pinned_query, &boxed_params(&base_where_params))?;
            for row in pinned_rows {
                let mut s = fold_session_dict(row);
                let s_id = s.get("id").and_then(Value::as_str).map(|x| x.to_string());
                if s_id.as_deref().map(|x| seen_ids.contains(x)).unwrap_or(false) {
                    continue;
                }
                let preview_raw = s
                    .as_object_mut()
                    .and_then(|o| o.remove("_preview_raw"))
                    .and_then(|v| v.as_str().map(|x| x.to_string()))
                    .unwrap_or_default();
                if let Some(obj) = s.as_object_mut() {
                    obj.insert("preview".to_string(), Value::String(_shape_preview(preview_raw)));
                }
                if let Some(sid) = s_id {
                    seen_ids.insert(sid);
                }
                sessions.push(s);
            }
        }

        // Project compression roots forward to their tips.
        if p.project_compression_tips && !p.include_children {
            let mut tip_ids_by_root: HashMap<String, String> = HashMap::new();
            for s in &sessions {
                if s.get("end_reason").and_then(Value::as_str) != Some("compression") {
                    continue;
                }
                let root_id = s.get("id").and_then(Value::as_str).unwrap_or("").to_string();
                let tip_id = self.get_compression_tip(&root_id)?;
                if tip_id != root_id {
                    tip_ids_by_root.insert(root_id, tip_id);
                }
            }
            let tip_ids: Vec<String> = tip_ids_by_root.values().cloned().collect();
            let tip_rows = if tip_ids.is_empty() {
                HashMap::new()
            } else {
                self.get_session_rich_rows_batch(&tip_ids, p.compact_rows)?
            };
            let mut projected: Vec<Value> = Vec::new();
            for s in sessions {
                let root_id = s.get("id").and_then(Value::as_str).unwrap_or("").to_string();
                let tip_id = tip_ids_by_root.get(&root_id);
                let tip_row = tip_id.and_then(|tid| tip_rows.get(tid));
                let Some(tip_row) = tip_row else {
                    projected.push(s);
                    continue;
                };
                let mut merged = s.clone();
                const MERGE_KEYS: [&str; 13] = [
                    "id", "ended_at", "end_reason", "message_count",
                    "tool_call_count", "title", "last_active", "preview",
                    "model", "system_prompt", "cwd", "git_branch", "git_repo_root",
                ];
                if let (Some(src), Some(dst)) = (tip_row.as_object(), merged.as_object_mut()) {
                    for key in MERGE_KEYS {
                        if let Some(v) = src.get(key) {
                            dst.insert(key.to_string(), v.clone());
                        }
                    }
                }
                merged
                    .as_object_mut()
                    .unwrap()
                    .insert("_lineage_root_id".to_string(), Value::String(root_id));
                projected.push(merged);
            }
            sessions = projected;
        }

        // Derive read state per surfaced conversation.
        for s in &mut sessions {
            let unread = Self::session_unread(s);
            s.as_object_mut()
                .unwrap()
                .insert("unread".to_string(), Value::Bool(unread));
        }
        Ok(sessions)
    }

    /// Count sessions, optionally filtered. `exclude_children=True` mirrors
    /// `list_sessions_rich` visibility so the count matches surfaced rows.
    ///
    // `clippy::too_many_arguments` allowed: the parameter set mirrors the
    // upstream keyword surface 1:1 (a struct would diverge from the
    // observable call shape consumers use).
    #[allow(clippy::too_many_arguments)]
    // PARITY: SessionDB.session_count @ b9aa928 (8079–8138)
    pub fn session_count(
        &self,
        source: Option<&str>,
        sources: &[String],
        cwd_prefix: Option<&str>,
        min_message_count: i64,
        include_archived: bool,
        archived_only: bool,
        exclude_children: bool,
        exclude_sources: &[String],
    ) -> Result<i64, WriteError> {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if exclude_children {
            where_clauses.push(_listable_child_sql());
            where_clauses.push(format!("{} IS NULL", delegate_from_json_sql("s.model_config")));
        }
        let mut include_sources: Vec<String> = Vec::new();
        if let Some(s) = source {
            include_sources.push(s.to_string());
        }
        include_sources.extend(sources.iter().cloned());
        if !include_sources.is_empty() {
            let placeholders = include_sources.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            where_clauses.push(format!("s.source IN ({placeholders})"));
            for s in &include_sources {
                params.push(Box::new(s.clone()));
            }
        }
        if !exclude_sources.is_empty() {
            let placeholders = exclude_sources.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            where_clauses.push(format!("s.source NOT IN ({placeholders})"));
            for s in exclude_sources {
                params.push(Box::new(s.clone()));
            }
        }
        if let Some(prefix) = cwd_prefix {
            let (clause, clause_params) = cwd_prefix_clause(prefix);
            where_clauses.push(clause);
            for cp in clause_params {
                params.push(Box::new(cp));
            }
        }
        if min_message_count > 0 {
            where_clauses.push("s.message_count >= ?".to_string());
            params.push(Box::new(min_message_count));
        }
        if archived_only {
            where_clauses.push("s.archived = 1".to_string());
        } else if !include_archived {
            where_clauses.push("s.archived = 0".to_string());
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };
        let conn = self.writer_conn();
        let count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM sessions s{where_sql}"),
                rusqlite::params_from_iter(params.iter().map(|p| p as &dyn rusqlite::ToSql)),
                |r| r.get(0),
            )
            .map_err(WriteError::Sqlite)?;
        Ok(count)
    }

    /// Check if at least N sessions exist (archived included); short-circuits
    /// via LIMIT.
    ///
    /// PARITY: SessionDB.session_count_ge @ b9aa928 (8140–8153)
    pub fn session_count_ge(&self, n: i64) -> Result<bool, WriteError> {
        let conn = self.writer_conn();
        let rows = conn
            .prepare("SELECT 1 FROM sessions LIMIT ?")
            .map_err(WriteError::Sqlite)?
            .query_map([n], |_| Ok(()))
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows.len() >= n.max(0) as usize)
    }

    /// Return a `{source: count}` dict via a single GROUP BY query.
    ///
    /// PARITY: SessionDB.session_count_by_source @ b9aa928 (8156–8200)
    pub fn session_count_by_source(
        &self,
        include_archived: bool,
        archived_only: bool,
        exclude_children: bool,
    ) -> Result<HashMap<String, i64>, WriteError> {
        let mut where_clauses: Vec<String> = Vec::new();
        let params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if exclude_children {
            where_clauses.push(_listable_child_sql());
            where_clauses.push(format!("{} IS NULL", delegate_from_json_sql("s.model_config")));
        }
        if archived_only {
            where_clauses.push("s.archived = 1".to_string());
        } else if !include_archived {
            where_clauses.push("s.archived = 0".to_string());
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };
        let sql = format!(
            "SELECT COALESCE(NULLIF(s.source, ''), 'cli') AS source, COUNT(*) AS count \
             FROM sessions s{where_sql} \
             GROUP BY COALESCE(NULLIF(s.source, ''), 'cli') \
             ORDER BY count DESC"
        );
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|p| p as &dyn rusqlite::ToSql)),
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows.into_iter().collect())
    }

    /// Count empty, ended, non-archived sessions — the "Delete empty"
    /// dashboard guard.
    ///
    /// PARITY: SessionDB.count_empty_sessions @ b9aa928 (8544–8565)
    pub fn count_empty_sessions(&self) -> Result<i64, WriteError> {
        let conn = self.writer_conn();
        conn.query_row(
            "SELECT COUNT(*) FROM sessions \
             WHERE message_count = 0 AND ended_at IS NOT NULL AND archived = 0",
            [],
            |r| r.get(0),
        )
        .map_err(WriteError::Sqlite)
    }

    /// Search surfaced sessions by exact/prefix/substring session id.
    ///
    /// PARITY: hermes_state_search.py search_sessions_by_id @ b9aa928
    /// (2101–2160)
    pub fn search_sessions_by_id(
        &self,
        query: &str,
        limit: i64,
        include_archived: bool,
        source: Option<&str>,
        sources: Vec<String>,
        exclude_sources: Vec<String>,
    ) -> Result<Vec<Value>, WriteError> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() || limit <= 0 {
            return Ok(Vec::new());
        }
        let fetch_limit = (limit * 4).max(limit);
        let candidates = self.list_sessions_rich(&RichListParams {
            source: source.map(|s| s.to_string()),
            sources,
            exclude_sources,
            limit: fetch_limit,
            offset: 0,
            include_archived,
            order_by_last_active: true,
            id_query: Some(needle.clone()),
            ..Default::default()
        })?;

        fn score(row: &Value, needle: &str) -> i64 {
            let mut ids: Vec<String> = Vec::new();
            if let Some(v) = row.get("id").and_then(Value::as_str) {
                ids.push(v.to_lowercase());
            }
            if let Some(v) = row.get("_lineage_root_id").and_then(Value::as_str) {
                ids.push(v.to_lowercase());
            }
            if ids.iter().any(|x| x == needle) {
                return 0;
            }
            if ids.iter().any(|x| x.starts_with(needle)) {
                return 1;
            }
            2
        }

        let mut ranked: Vec<(i64, usize, Value)> = candidates
            .into_iter()
            .enumerate()
            .map(|(i, row)| (score(&row, &needle), i, row))
            .collect();
        ranked.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        Ok(ranked
            .into_iter()
            .take(limit as usize)
            .map(|(_, _, row)| row)
            .collect())
    }

    /// Run a SELECT and materialize all rows as JSON objects.
    fn query_values(
        &self,
        sql: &str,
        params: &[Box<dyn rusqlite::ToSql>],
    ) -> Result<Vec<Value>, WriteError> {
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
                crate::portability::row_to_value,
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows)
    }
}
