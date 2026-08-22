//! SessionDB CRUD surface — sessions, messages, titles.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   _insert_session_row + create_session        (3137–3308)
//!   end_session / reopen_session / promote...   (3895–3971)
//!   update_session_cwd / backfill_repo_roots    (3971–4040)
//!   sanitize_title + title family               (5598–6023)
//!   append_message / batch / _insert_message_rows (6478–7170)
//!   get_messages / latest-message helpers       (7013–7073, 7349–7401)
//!   _check_transcript_write_guards              (6555–6609)
//!   _encode/_decode content + display metadata  (6478–6609)

use rusqlite::{Connection, OptionalExtension, Row};
use serde_json::{json, Value};

use crate::schema;
use crate::state::now;

use super::state::{SessionDB, WriteError};

// ── input structs ───────────────────────────────────────────────────────────

/// Keyword-style options for `SessionDB::create_session` (mirrors upstream's
/// `create_session(..., **kwargs)` / `_insert_session_row` signature).
#[derive(Debug, Clone, Default)]
pub struct NewSession {
    pub model: Option<String>,
    pub model_config: Option<Value>,
    pub system_prompt: Option<String>,
    pub user_id: Option<String>,
    pub session_key: Option<String>,
    pub chat_id: Option<String>,
    pub chat_type: Option<String>,
    pub thread_id: Option<String>,
    pub parent_session_id: Option<String>,
    pub cwd: Option<String>,
    pub profile_name: Option<String>,
    pub git_repo_root: Option<String>,
}

/// One appendable message (mirrors `append_message` kwargs; the batch writer
/// consumes the same struct so all multi-row writers share one serializer).
#[derive(Debug, Clone, Default)]
pub struct MessageInput {
    pub role: String,
    pub content: Option<Value>,
    pub tool_name: Option<String>,
    pub tool_calls: Option<Value>,
    pub tool_call_id: Option<String>,
    pub token_count: Option<i64>,
    pub finish_reason: Option<String>,
    pub reasoning: Option<String>,
    pub reasoning_content: Option<String>,
    pub reasoning_details: Option<Value>,
    pub codex_reasoning_items: Option<Value>,
    pub codex_message_items: Option<Value>,
    pub platform_message_id: Option<String>,
    /// Legacy alias accepted by import/export dicts (yuanbao `message_id`).
    pub message_id: Option<String>,
    pub observed: bool,
    pub effect_disposition: Option<String>,
    pub timestamp: Option<f64>,
    pub api_content: Option<String>,
    pub display_kind: Option<String>,
    pub display_metadata: Option<Value>,
}

// ── output structs ──────────────────────────────────────────────────────────

/// A sessions row as `get_session` / `get_session_by_title` return it, with
/// the system prompt resolved (the same `_session_row_dict` transformation:
/// `COALESCE(sp.prompt, s.system_prompt)` folded into `system_prompt`).
#[derive(Debug, Clone, Default)]
pub struct SessionRow {
    pub id: String,
    pub source: String,
    pub user_id: Option<String>,
    pub session_key: Option<String>,
    pub chat_id: Option<String>,
    pub chat_type: Option<String>,
    pub thread_id: Option<String>,
    pub model: Option<String>,
    pub model_config: Option<String>,
    pub system_prompt: Option<String>,
    pub system_prompt_hash: Option<String>,
    pub parent_session_id: Option<String>,
    pub started_at: f64,
    pub ended_at: Option<f64>,
    pub end_reason: Option<String>,
    pub message_count: i64,
    pub tool_call_count: i64,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub git_repo_root: Option<String>,
    pub title: Option<String>,
    pub profile_name: Option<String>,
    pub archived: bool,
}

/// A messages row as `get_messages` returns it (decoded).
#[derive(Debug, Clone, Default)]
pub struct StoredMessage {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub content: Option<Value>,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<Value>,
    pub tool_name: Option<String>,
    pub effect_disposition: Option<String>,
    pub timestamp: f64,
    pub token_count: Option<i64>,
    pub finish_reason: Option<String>,
    pub reasoning: Option<String>,
    pub reasoning_content: Option<String>,
    pub reasoning_details: Option<String>,
    pub codex_reasoning_items: Option<String>,
    pub codex_message_items: Option<String>,
    pub platform_message_id: Option<String>,
    pub observed: bool,
    pub active: bool,
    pub compacted: bool,
    pub api_content: Option<String>,
    pub display_kind: Option<String>,
    pub display_metadata: Option<Value>,
}

// ── content JSON framing (mirrors _encode_content/_decode_content) ─────────

pub(crate) const CONTENT_JSON_PREFIX: &str = "\u{0}json:";

pub(crate) fn encode_content(content: Option<&Value>) -> Option<rusqlite::types::Value> {
    // Returns the sqlite-bindable value: strings pass through (Rust strings
    // are already surrogate-free — the Python surrogate scrub is a no-op),
    // numbers pass through, structured values become the sentinel JSON form.
    // Python bool is an int subclass, mirroring the 0/1 binding.
    match content {
        None | Some(Value::Null) => Some(rusqlite::types::Value::Null),
        Some(Value::String(s)) => Some(rusqlite::types::Value::Text(s.clone())),
        Some(Value::Bool(b)) => Some(rusqlite::types::Value::Integer(if *b { 1 } else { 0 })),
        Some(Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                Some(rusqlite::types::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Some(rusqlite::types::Value::Real(f))
            } else {
                Some(rusqlite::types::Value::Null)
            }
        }
        Some(v @ (Value::Array(_) | Value::Object(_))) => Some(rusqlite::types::Value::Text(
            format!("{}{}", CONTENT_JSON_PREFIX, v),
        )),
    }
}

pub(crate) fn decode_content(raw: Option<rusqlite::types::Value>) -> Option<Value> {
    // Upstream reads the column via dict(row) and _decode_content: scalars
    // (str/int/float) pass through unchanged; sentinel JSON is parsed.
    match raw {
        None => None,
        Some(rusqlite::types::Value::Text(s)) if s.starts_with(CONTENT_JSON_PREFIX) => {
            // On parse failure Python logs a warning and returns the raw
            // string — mirror that fallback rather than dropping the value.
            match serde_json::from_str(&s[CONTENT_JSON_PREFIX.len()..]) {
                Ok(v) => Some(v),
                Err(_) => Some(Value::String(s)),
            }
        }
        Some(rusqlite::types::Value::Text(s)) => Some(Value::String(s)),
        Some(rusqlite::types::Value::Integer(i)) => Some(json!(i)),
        Some(rusqlite::types::Value::Real(f)) => Some(json!(f)),
        // Blob or stale NULL: keep None for NULL, stringify blobs loosely.
        Some(rusqlite::types::Value::Blob(b)) => {
            Some(Value::String(String::from_utf8_lossy(&b).into_owned()))
        }
        Some(rusqlite::types::Value::Null) => None,
    }
}

pub(crate) fn encode_display_metadata(display_metadata: Option<&Value>) -> Option<String> {
    match display_metadata {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => match serde_json::from_str::<Value>(s) {
            Ok(parsed) if parsed.is_object() => Some(parsed.to_string()),
            _ => None,
        },
        Some(v) if v.is_object() => Some(v.to_string()),
        _ => None,
    }
}

pub(crate) fn decode_display_metadata(raw: Option<&str>) -> Option<Value> {
    match raw {
        None => None,
        Some(s) => {
            let meta = serde_json::from_str::<Value>(s).ok()?;
            // Rows written before the encode guard are double-encoded.
            let meta = match meta {
                Value::String(s2) => serde_json::from_str::<Value>(&s2).ok()?,
                other => other,
            };
            if meta.is_object() {
                Some(meta)
            } else {
                None
            }
        }
    }
}

fn parse_tool_calls(tool_calls: Option<&Value>) -> Option<Value> {
    // tool_calls may arrive as a list (live agent) or a JSON string
    // (import/export). Parse first so JSON strings aren't double-encoded.
    match tool_calls {
        None => None,
        Some(Value::String(s)) => serde_json::from_str::<Value>(s).ok(),
        Some(v) => Some(v.clone()),
    }
}

pub(crate) fn truthy(value: Option<&Value>) -> bool {
    match value {
        None => false,
        Some(Value::Null) => false,
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(m)) => !m.is_empty(),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => !(n.as_i64() == Some(0) || n.as_f64() == Some(0.0)),
    }
}

pub(crate) fn scrub_surrogates(s: Option<String>) -> Option<String> {
    // Upstream replaces lone UTF-16 surrogates with U+FFFD before binding.
    // Rust strings are always valid UTF-8, so this is a no-op; kept as a
    // named seam for parity-reading.
    s
}

// ── session rows ────────────────────────────────────────────────────────────

const SESSION_SELECT: &str = "SELECT s.*, \
     COALESCE(sp.prompt, s.system_prompt) AS _system_prompt_resolved \
     FROM sessions s \
     LEFT JOIN system_prompts sp ON sp.hash = s.system_prompt_hash ";

pub(crate) fn session_row(row: &Row<'_>) -> rusqlite::Result<SessionRow> {
    Ok(SessionRow {
        id: row.get("id")?,
        source: row.get("source")?,
        user_id: row.get("user_id")?,
        session_key: row.get("session_key")?,
        chat_id: row.get("chat_id")?,
        chat_type: row.get("chat_type")?,
        thread_id: row.get("thread_id")?,
        model: row.get("model")?,
        model_config: row.get("model_config")?,
        system_prompt: row.get("_system_prompt_resolved")?,
        system_prompt_hash: row.get("system_prompt_hash")?,
        parent_session_id: row.get("parent_session_id")?,
        started_at: row.get("started_at")?,
        ended_at: row.get("ended_at")?,
        end_reason: row.get("end_reason")?,
        message_count: row.get("message_count")?,
        tool_call_count: row.get("tool_call_count")?,
        cwd: row.get("cwd")?,
        git_branch: row.get("git_branch")?,
        git_repo_root: row.get("git_repo_root")?,
        title: row.get("title")?,
        profile_name: row.get("profile_name")?,
        archived: row.get::<_, i64>("archived")? != 0,
    })
}

fn make_session_row(db: &SessionDB, id: &str) -> rusqlite::Result<Option<SessionRow>> {
    let conn = db.writer_conn();
    let sql = format!("{} WHERE s.id = ?", SESSION_SELECT);
    let row = conn
        .query_row(&sql, rusqlite::params![id], session_row)
        .optional()?;
    Ok(row)
}

// ── sessions CRUD ───────────────────────────────────────────────────────────

// ── helper: referenced system-prompt cleanup ────────────────────────────────

/// Delete stored system prompts no session references anymore.
///
/// PARITY: hermes_state.py _delete_unreferenced_system_prompts @ b9aa928
/// (2189–2196)
pub(crate) fn delete_unreferenced_system_prompts(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM system_prompts \
         WHERE NOT EXISTS (\
             SELECT 1 FROM sessions \
             WHERE sessions.system_prompt_hash = system_prompts.hash\
         )",
        [],
    )?;
    Ok(())
}

// ── session-row upsert (shared by create_session / token accounting) ────────

/// Upsert a session row (mirrors upstream `_insert_session_row`'s `_do`:
/// INSERT with ON CONFLICT enrichment + parent backfill + compression-fork
/// origin inheritance). Connection-level so the token writer's dedicated
/// connection and `update_token_counts` reuse the identical body.
///
/// PARITY: hermes_state.py _insert_session_row @ b9aa928 (3137–3302)
pub(crate) fn insert_session_row_on(
    conn: &Connection,
    session_id: &str,
    source: &str,
    opts: &NewSession,
) -> Result<(), WriteError> {
    let _ = source; // mirror-signature seam: used by callers for the row source
    {
        let system_prompt_hash = schema::store_system_prompt(conn, opts.system_prompt.clone())?;
        // Python: `json.dumps(model_config) if model_config else None`
        // (an empty dict is falsy and stores NULL).
        let model_config = opts
            .model_config
            .as_ref()
            .filter(|v| truthy(Some(*v)))
            .map(|v| v.to_string());
        conn.execute(
                "INSERT INTO sessions (
                   id, source, user_id, session_key, chat_id, chat_type, thread_id,
                   model, model_config, system_prompt, system_prompt_hash,
                   parent_session_id, cwd, profile_name, git_repo_root, started_at
                )
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?)
                   ON CONFLICT(id) DO UPDATE SET
                       model = COALESCE(sessions.model, excluded.model),
                       model_config = COALESCE(sessions.model_config, excluded.model_config),
                       system_prompt_hash = COALESCE(
                           sessions.system_prompt_hash,
                           excluded.system_prompt_hash
                       ),
                       system_prompt = CASE
                           WHEN sessions.system_prompt_hash IS NULL
                                AND excluded.system_prompt_hash IS NOT NULL
                           THEN NULL
                           ELSE sessions.system_prompt
                       END,
                       session_key = COALESCE(sessions.session_key, excluded.session_key),
                       chat_id = COALESCE(sessions.chat_id, excluded.chat_id),
                       chat_type = COALESCE(sessions.chat_type, excluded.chat_type),
                       thread_id = COALESCE(sessions.thread_id, excluded.thread_id),
                       parent_session_id = COALESCE(sessions.parent_session_id, excluded.parent_session_id),
                       cwd = COALESCE(sessions.cwd, excluded.cwd),
                       profile_name = COALESCE(sessions.profile_name, excluded.profile_name),
                       git_repo_root = COALESCE(sessions.git_repo_root, excluded.git_repo_root)",
                rusqlite::params![
                    session_id,
                    source,
                    opts.user_id,
                    opts.session_key,
                    opts.chat_id,
                    opts.chat_type,
                    opts.thread_id,
                    opts.model,
                    model_config,
                    system_prompt_hash,
                    opts.parent_session_id,
                    opts.cwd,
                    opts.profile_name,
                    opts.git_repo_root,
                    now(),
                ],
            )?;
        if system_prompt_hash.is_some() {
            delete_unreferenced_system_prompts(conn)?;
        }
        if opts.parent_session_id.is_some() {
            // Backfill cwd / git_repo_root / git_branch / profile_name
            // from the parent row (#64709, cross-profile jump bug).
            conn.execute(
                "UPDATE sessions
                       SET cwd = COALESCE(sessions.cwd,
                                 (SELECT p.cwd FROM sessions p
                                   WHERE p.id = sessions.parent_session_id)),
                           git_repo_root = COALESCE(sessions.git_repo_root,
                                           (SELECT p.git_repo_root FROM sessions p
                                             WHERE p.id = sessions.parent_session_id)),
                           git_branch = COALESCE(sessions.git_branch,
                                        (SELECT p.git_branch FROM sessions p
                                          WHERE p.id = sessions.parent_session_id)),
                           profile_name = COALESCE(sessions.profile_name,
                                          (SELECT p.profile_name FROM sessions p
                                            WHERE p.id = sessions.parent_session_id))
                     WHERE id = ? AND parent_session_id IS NOT NULL",
                rusqlite::params![session_id],
            )?;
            // Compression-fork origin inheritance (#59527): only when
            // the parent already ended with end_reason='compression'.
            conn.execute(
                "UPDATE sessions
                       SET user_id = COALESCE(sessions.user_id,
                                     (SELECT p.user_id FROM sessions p
                                       WHERE p.id = sessions.parent_session_id)),
                           session_key = COALESCE(sessions.session_key,
                                         (SELECT p.session_key FROM sessions p
                                           WHERE p.id = sessions.parent_session_id)),
                           chat_id = COALESCE(sessions.chat_id,
                                     (SELECT p.chat_id FROM sessions p
                                       WHERE p.id = sessions.parent_session_id)),
                           chat_type = COALESCE(sessions.chat_type,
                                       (SELECT p.chat_type FROM sessions p
                                         WHERE p.id = sessions.parent_session_id)),
                           thread_id = COALESCE(sessions.thread_id,
                                       (SELECT p.thread_id FROM sessions p
                                         WHERE p.id = sessions.parent_session_id)),
                           display_name = COALESCE(sessions.display_name,
                                          (SELECT p.display_name FROM sessions p
                                            WHERE p.id = sessions.parent_session_id)),
                           origin_json = COALESCE(sessions.origin_json,
                                         (SELECT p.origin_json FROM sessions p
                                           WHERE p.id = sessions.parent_session_id))
                     WHERE id = ? AND parent_session_id IS NOT NULL
                       AND EXISTS (
                           SELECT 1 FROM sessions p
                           WHERE p.id = sessions.parent_session_id
                             AND p.end_reason = 'compression'
                       )",
                rusqlite::params![session_id],
            )?;
        }
    }
    Ok(())
}

impl SessionDB {
    /// Create a new session record. Returns the session_id.
    /// PARITY: SessionDB.create_session @ b9aa928 (3304–3308)
    pub fn create_session(
        &self,
        session_id: &str,
        source: &str,
        opts: &NewSession,
    ) -> Result<String, WriteError> {
        let f = |conn: &Connection| -> Result<String, WriteError> {
            insert_session_row_on(conn, session_id, source, opts)?;
            Ok(session_id.to_string())
        };
        // Session-row creation is transcript-critical.
        self.execute_write(&f, Some(SessionDB::TRANSCRIPT_WRITE_PATIENCE_S))
    }

    /// Get a session by ID (system prompt resolved), or None.
    /// PARITY: SessionDB.get_session @ b9aa928 (5554–5572)
    pub fn get_session(&self, session_id: &str) -> Result<Option<SessionRow>, WriteError> {
        // Upstream drains queued token deltas here first so readers see
        // exact totals even while the background writer is mid-backlog
        // (hermes_state.py get_session @ 5559).
        let _ = self.flush_token_counts(5.0);
        make_session_row(self, session_id).map_err(WriteError::Sqlite)
    }

    /// Resolve an exact or uniquely prefixed session ID.
    /// PARITY: SessionDB.resolve_session_id @ b9aa928 (5573–5598)
    pub fn resolve_session_id(
        &self,
        session_id_or_prefix: &str,
    ) -> Result<Option<String>, WriteError> {
        if let Some(exact) = self.get_session(session_id_or_prefix)? {
            return Ok(Some(exact.id));
        }
        let escaped = crate::common::escape_like(session_id_or_prefix);
        let conn = self.writer_conn();
        let mut stmt = conn
            .prepare("SELECT id FROM sessions WHERE id LIKE ? ESCAPE '\\' ORDER BY started_at DESC LIMIT 2")
            .map_err(WriteError::Sqlite)?;
        let matches: Vec<String> = stmt
            .query_map(rusqlite::params![format!("{}%", escaped)], |r| r.get(0))
            .map_err(WriteError::Sqlite)?
            .collect::<Result<_, _>>()
            .map_err(WriteError::Sqlite)?;
        if matches.len() == 1 {
            Ok(Some(matches[0].clone()))
        } else {
            Ok(None)
        }
    }

    /// Mark a session as ended. First end_reason wins.
    /// PARITY: SessionDB.end_session @ b9aa928 (3895–3912)
    pub fn end_session(&self, session_id: &str, end_reason: &str) -> Result<(), WriteError> {
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET ended_at = ?, end_reason = ? \
                 WHERE id = ? AND ended_at IS NULL",
                rusqlite::params![now(), end_reason, session_id],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Clear ended_at/end_reason so a session can be resumed.
    /// PARITY: SessionDB.reopen_session @ b9aa928 (3913–3920)
    pub fn reopen_session(&self, session_id: &str) -> Result<(), WriteError> {
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET ended_at = NULL, end_reason = NULL WHERE id = ?",
                rusqlite::params![session_id],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Durably mark a session as ended by an intentional reset boundary.
    /// PARITY: SessionDB.promote_to_session_reset @ b9aa928 (3922–3969)
    pub fn promote_to_session_reset(
        &self,
        session_id: &str,
        reason: &str,
    ) -> Result<bool, WriteError> {
        if session_id.is_empty() {
            return Ok(false);
        }
        let now = now();
        let f = |conn: &Connection| -> Result<bool, WriteError> {
            let affected = conn.execute(
                "UPDATE sessions SET ended_at = ?, end_reason = ? \
                 WHERE id = ? AND (ended_at IS NULL \
                 OR end_reason IN ('agent_close', 'ws_orphan_reap'))",
                rusqlite::params![now, reason, session_id],
            )?;
            Ok(affected > 0)
        };
        Ok(self.execute_write(&f, None).unwrap_or(false))
    }

    /// Persist the session working directory + git metadata.
    /// PARITY: SessionDB.update_session_cwd @ b9aa928 (3971–4014)
    pub fn update_session_cwd(
        &self,
        session_id: &str,
        cwd: &str,
        git_branch: Option<&str>,
        git_repo_root: Option<&str>,
        replace_git_meta: bool,
    ) -> Result<(), WriteError> {
        if session_id.is_empty() || cwd.is_empty() {
            return Ok(());
        }
        let branch = git_branch.map(str::trim).filter(|b| !b.is_empty());
        let repo_root = git_repo_root.map(str::trim).filter(|r| !r.is_empty());

        let mut sets = vec!["cwd = ?".to_string()];
        let mut params: Vec<rusqlite::types::Value> =
            vec![rusqlite::types::Value::Text(cwd.to_string())];
        if branch.is_some() || replace_git_meta {
            sets.push("git_branch = ?".to_string());
            params.push(
                branch
                    .map(|s| rusqlite::types::Value::Text(s.to_string()))
                    .unwrap_or(rusqlite::types::Value::Null),
            );
        }
        if repo_root.is_some() || replace_git_meta {
            sets.push("git_repo_root = ?".to_string());
            params.push(
                repo_root
                    .map(|s| rusqlite::types::Value::Text(s.to_string()))
                    .unwrap_or(rusqlite::types::Value::Null),
            );
        }
        params.push(rusqlite::types::Value::Text(session_id.to_string()));

        let f = move |conn: &Connection| -> Result<(), WriteError> {
            let sql = format!("UPDATE sessions SET {} WHERE id = ?", sets.join(", "));
            conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    // ── titles ──────────────────────────────────────────────────────────────

    pub const MAX_TITLE_LENGTH: usize = 100;

    /// Validate and sanitize a session title.
    /// PARITY: SessionDB.sanitize_title @ b9aa928 (5599–5645)
    pub fn sanitize_title(title: &str) -> Result<Option<String>, String> {
        if title.is_empty() {
            return Ok(None);
        }
        // Rust strings are always valid UTF-8: the surrogate scrub is a no-op.
        // Remove ASCII control chars (keep \t \n \r for collapse below).
        let cleaned: String = title
            .chars()
            .filter(|&c| {
                let cp = c as u32;
                !(cp <= 0x08
                    || cp == 0x0b
                    || cp == 0x0c
                    || (0x0e..=0x1f).contains(&cp)
                    || cp == 0x7f
                    // Problematic Unicode controls
                    || (0x200b..=0x200f).contains(&cp)
                    || (0x2028..=0x202e).contains(&cp)
                    || (0x2060..=0x2069).contains(&cp)
                    || cp == 0xfeff
                    || cp == 0xfffc
                    || (0xfff9..=0xfffb).contains(&cp))
            })
            .collect();
        // Collapse internal whitespace runs and strip.
        let collapsed: String = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
        if collapsed.is_empty() {
            return Ok(None);
        }
        if collapsed.chars().count() > SessionDB::MAX_TITLE_LENGTH {
            return Err(format!(
                "Title too long ({} chars, max {})",
                collapsed.chars().count(),
                SessionDB::MAX_TITLE_LENGTH
            ));
        }
        Ok(Some(collapsed))
    }

    fn compression_ancestor(
        conn: &Connection,
        ancestor_id: &str,
        descendant_id: &str,
    ) -> rusqlite::Result<bool> {
        if ancestor_id.is_empty() || descendant_id.is_empty() || ancestor_id == descendant_id {
            return Ok(false);
        }
        let edge = crate::common::_compression_child_sql("child");
        let sql = format!(
            "WITH RECURSIVE ancestors(id) AS (
                SELECT ?
                UNION
                SELECT parent.id
                FROM ancestors a
                JOIN sessions child ON child.id = a.id
                JOIN sessions parent ON parent.id = child.parent_session_id
                WHERE {}
            )
            SELECT 1 FROM ancestors WHERE id = ? AND id != ? LIMIT 1",
            edge
        );
        let row = conn
            .query_row(
                &sql,
                rusqlite::params![descendant_id, ancestor_id, descendant_id],
                |_| Ok(()),
            )
            .optional()?;
        Ok(row.is_some())
    }

    fn set_session_title_inner(
        &self,
        session_id: &str,
        title: &str,
        only_if_empty: bool,
    ) -> Result<bool, WriteError> {
        let title = SessionDB::sanitize_title(title).map_err(WriteError::ValueError)?;

        let f = |conn: &Connection| -> Result<i64, WriteError> {
            if only_if_empty {
                let current: Option<Option<String>> = conn
                    .query_row(
                        "SELECT title FROM sessions WHERE id = ?",
                        rusqlite::params![session_id],
                        |r| r.get::<_, Option<String>>(0),
                    )
                    .optional()?;
                match current {
                    // Missing row, or a title already set -> nothing to do.
                    None => return Ok(0),
                    Some(Some(_)) => return Ok(0),
                    Some(None) => {}
                }
            }

            if let Some(t) = title.as_deref() {
                // Check uniqueness (allow the same session to keep its own title)
                let conflict: Option<String> = conn
                    .query_row(
                        "SELECT id FROM sessions WHERE title = ? AND id != ?",
                        rusqlite::params![t, session_id],
                        |r| r.get(0),
                    )
                    .optional()?;
                if let Some(conflict_id) = conflict {
                    if SessionDB::compression_ancestor(conn, &conflict_id, session_id)? {
                        conn.execute(
                            "UPDATE sessions SET title = NULL WHERE id = ?",
                            rusqlite::params![conflict_id],
                        )?;
                    } else {
                        return Err(WriteError::ValueError(format!(
                            "Title '{}' is already in use by session {}",
                            t, conflict_id
                        )));
                    }
                }
            }
            let predicate = if only_if_empty {
                " AND title IS NULL"
            } else {
                ""
            };
            let sql = format!("UPDATE sessions SET title = ? WHERE id = ?{}", predicate);
            let affected = conn.execute(&sql, rusqlite::params![title, session_id])?;
            Ok(affected as i64)
        };

        let rowcount = self.execute_write(&f, None)?;
        Ok(rowcount > 0)
    }

    /// Set or update a session's title.
    /// PARITY: SessionDB.set_session_title @ b9aa928 (5741–5750)
    pub fn set_session_title(&self, session_id: &str, title: &str) -> Result<bool, WriteError> {
        self.set_session_title_inner(session_id, title, false)
    }

    /// Set an auto-generated title only when the current title is NULL.
    /// PARITY: SessionDB.set_auto_title_if_empty @ b9aa928 (5751–5759)
    pub fn set_auto_title_if_empty(
        &self,
        session_id: &str,
        title: &str,
    ) -> Result<bool, WriteError> {
        self.set_session_title_inner(session_id, title, true)
    }

    /// Get the title for a session, or None.
    /// PARITY: SessionDB.get_session_title @ b9aa928 (5760–5768)
    pub fn get_session_title(&self, session_id: &str) -> Result<Option<String>, WriteError> {
        let conn = self.writer_conn();
        let row: Option<String> = conn
            .query_row(
                "SELECT title FROM sessions WHERE id = ?",
                rusqlite::params![session_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(row)
    }

    /// Look up a session by exact title.
    /// PARITY: SessionDB.get_session_by_title @ b9aa928 (5945–5960)
    pub fn get_session_by_title(&self, title: &str) -> Result<Option<SessionRow>, WriteError> {
        let conn = self.writer_conn();
        let sql = format!("{} WHERE s.title = ?", SESSION_SELECT);
        let row = conn
            .query_row(&sql, rusqlite::params![title], session_row)
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(row)
    }

    /// Resolve a title to a session ID, preferring the latest in a lineage.
    /// PARITY: SessionDB.resolve_session_by_title @ b9aa928 (5959–5989)
    pub fn resolve_session_by_title(&self, title: &str) -> Result<Option<String>, WriteError> {
        let exact = self.get_session_by_title(title)?;
        let escaped = crate::common::escape_like(title);
        let conn = self.writer_conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, started_at FROM sessions \
                      WHERE title LIKE ? ESCAPE '\\' ORDER BY started_at DESC",
            )
            .map_err(WriteError::Sqlite)?;
        let numbered: Vec<String> = stmt
            .query_map(rusqlite::params![format!("{} #%", escaped)], |r| r.get(0))
            .map_err(WriteError::Sqlite)?
            .collect::<Result<_, _>>()
            .map_err(WriteError::Sqlite)?;
        if !numbered.is_empty() {
            Ok(Some(numbered[0].clone()))
        } else if let Some(exact) = exact {
            Ok(Some(exact.id))
        } else {
            Ok(None)
        }
    }

    /// Generate the next title in a lineage ("my session" → "my session #2").
    /// PARITY: SessionDB.get_next_title_in_lineage @ b9aa928 (5988–6023)
    pub fn get_next_title_in_lineage(&self, base_title: &str) -> Result<String, WriteError> {
        let base = match base_title.rfind(" #") {
            Some(idx) => {
                let suffix = &base_title[idx + 2..];
                if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
                    &base_title[..idx]
                } else {
                    base_title
                }
            }
            None => base_title,
        };

        let escaped = crate::common::escape_like(base);
        let conn = self.writer_conn();
        let mut stmt = conn
            .prepare("SELECT title FROM sessions WHERE title = ? OR title LIKE ? ESCAPE '\\'")
            .map_err(WriteError::Sqlite)?;
        let existing: Vec<String> = stmt
            .query_map(rusqlite::params![base, format!("{} #%", escaped)], |r| {
                r.get(0)
            })
            .map_err(WriteError::Sqlite)?
            .collect::<Result<_, _>>()
            .map_err(WriteError::Sqlite)?;
        if existing.is_empty() {
            return Ok(base.to_string());
        }
        let mut max_num: i64 = 1;
        for t in &existing {
            if let Some(idx) = t.rfind(" #") {
                let suffix = &t[idx + 2..];
                if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
                    if let Ok(n) = suffix.parse::<i64>() {
                        max_num = max_num.max(n);
                    }
                }
            }
        }
        Ok(format!("{} #{}", base, max_num + 1))
    }

    // ── message CRUD ────────────────────────────────────────────────────────

    fn check_transcript_write_guards(
        &self,
        conn: &Connection,
        session_id: &str,
        compression_lock_holder: Option<&str>,
    ) -> Result<(), WriteError> {
        // Live compression lock held by a different writer -> transient busy.
        let active_holder: Option<String> = conn
            .query_row(
                "SELECT holder FROM compression_locks \
                 WHERE session_id = ? AND expires_at > ?",
                rusqlite::params![session_id, now()],
                |r| r.get(0),
            )
            .optional()?;
        if active_holder.is_some() && active_holder.as_deref() != compression_lock_holder {
            return Err(WriteError::CompressionInProgress(session_id.to_string()));
        }
        // Session already closed by compression -> permanent rejection.
        let (ended_at, end_reason): (Option<f64>, Option<String>) = conn
            .query_row(
                "SELECT ended_at, end_reason FROM sessions WHERE id = ?",
                rusqlite::params![session_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .unwrap_or((None, None));
        if ended_at.is_some() && end_reason.as_deref() == Some("compression") {
            return Err(WriteError::CompressionSessionClosed(session_id.to_string()));
        }
        Ok(())
    }

    /// Append a message to a session. Returns the message row ID.
    /// PARITY: SessionDB.append_message @ b9aa928 (6611–6758)
    #[allow(clippy::too_many_arguments)]
    pub fn append_message(
        &self,
        session_id: &str,
        m: &MessageInput,
        compression_lock_holder: Option<&str>,
    ) -> Result<i64, WriteError> {
        let display_metadata_json = encode_display_metadata(m.display_metadata.as_ref());
        let reasoning_details_json = m
            .reasoning_details
            .as_ref()
            .filter(|v| truthy(Some(*v)))
            .map(|v| v.to_string());
        let codex_items_json = m
            .codex_reasoning_items
            .as_ref()
            .filter(|v| truthy(Some(*v)))
            .map(|v| v.to_string());
        let codex_message_items_json = m
            .codex_message_items
            .as_ref()
            .filter(|v| truthy(Some(*v)))
            .map(|v| v.to_string());
        let tool_calls = parse_tool_calls(m.tool_calls.as_ref()).filter(|v| truthy(Some(v)));
        let tool_calls_json = tool_calls.as_ref().map(|v| v.to_string());
        let stored_content = encode_content(m.content.as_ref());

        let message_timestamp = match m.timestamp {
            Some(ts) => ts,
            None => now(),
        };

        let num_tool_calls: i64 = match &tool_calls {
            Some(Value::Array(a)) => a.len() as i64,
            Some(_) => 1,
            None => 0,
        };

        let f = {
            let session_id = session_id.to_string();
            let tool_name = scrub_surrogates(m.tool_name.clone());
            let reasoning = scrub_surrogates(m.reasoning.clone());
            let reasoning_content = scrub_surrogates(m.reasoning_content.clone());
            let api_content = scrub_surrogates(m.api_content.clone());
            let display_kind = scrub_surrogates(m.display_kind.clone());
            move |conn: &Connection| -> Result<i64, WriteError> {
                self.check_transcript_write_guards(conn, &session_id, compression_lock_holder)?;
                conn.execute(
                    "INSERT INTO messages (session_id, role, content, tool_call_id,
                       tool_calls, tool_name, effect_disposition, timestamp, token_count, finish_reason,
                       reasoning, reasoning_content, reasoning_details, codex_reasoning_items,
                       codex_message_items, platform_message_id, observed, active, api_content, display_kind, display_metadata)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    rusqlite::params![
                        session_id,
                        m.role,
                        stored_content,
                        m.tool_call_id,
                        tool_calls_json,
                        tool_name,
                        m.effect_disposition,
                        message_timestamp,
                        m.token_count,
                        m.finish_reason,
                        reasoning,
                        reasoning_content,
                        reasoning_details_json,
                        codex_items_json,
                        codex_message_items_json,
                        m.platform_message_id,
                        if m.observed { 1 } else { 0 },
                        1,
                        api_content,
                        display_kind,
                        display_metadata_json,
                    ],
                )?;
                let msg_id = conn.last_insert_rowid();
                if num_tool_calls > 0 {
                    conn.execute(
                        "UPDATE sessions SET message_count = message_count + 1, \
                         tool_call_count = tool_call_count + ? WHERE id = ?",
                        rusqlite::params![num_tool_calls, session_id],
                    )?;
                } else {
                    conn.execute(
                        "UPDATE sessions SET message_count = message_count + 1 WHERE id = ?",
                        rusqlite::params![session_id],
                    )?;
                }
                Ok(msg_id)
            }
        };
        self.execute_write(&f, Some(SessionDB::TRANSCRIPT_WRITE_PATIENCE_S))
    }

    /// Append multiple messages atomically in ONE write transaction.
    /// PARITY: SessionDB.append_messages_batch @ b9aa928 (6758–6831)
    pub fn append_messages_batch(
        &self,
        session_id: &str,
        messages: &[MessageInput],
        compression_lock_holder: Option<&str>,
        chunk_rows: Option<usize>,
    ) -> Result<usize, WriteError> {
        if messages.is_empty() {
            return Ok(0);
        }
        if let Some(chunk_rows) = chunk_rows {
            if messages.len() > chunk_rows {
                let mut total = 0usize;
                for chunk in messages.chunks(chunk_rows) {
                    total += self.append_messages_batch(
                        session_id,
                        chunk,
                        compression_lock_holder,
                        None,
                    )?;
                }
                return Ok(total);
            }
        }

        let f = move |conn: &Connection| -> Result<usize, WriteError> {
            self.check_transcript_write_guards(conn, session_id, compression_lock_holder)?;
            let (inserted, tool_calls_total) =
                Self::insert_message_rows(conn, session_id, messages)?;
            if tool_calls_total > 0 {
                conn.execute(
                    "UPDATE sessions SET message_count = message_count + ?, \
                       tool_call_count = tool_call_count + ? WHERE id = ?",
                    rusqlite::params![inserted, tool_calls_total, session_id],
                )?;
            } else {
                conn.execute(
                    "UPDATE sessions SET message_count = message_count + ? WHERE id = ?",
                    rusqlite::params![inserted, session_id],
                )?;
            }
            Ok(inserted)
        };
        self.execute_write(&f, Some(SessionDB::TRANSCRIPT_WRITE_PATIENCE_S))
    }

    /// Insert *messages* as fresh active rows. Runs inside the caller's write
    /// transaction. Returns `(inserted_count, tool_call_count)`.
    /// PARITY: SessionDB._insert_message_rows @ b9aa928 (7073–7170)
    pub(crate) fn insert_message_rows(
        conn: &Connection,
        session_id: &str,
        messages: &[MessageInput],
    ) -> Result<(usize, i64), WriteError> {
        let mut now_ts = now();
        let mut inserted = 0usize;
        let mut tool_calls_total: i64 = 0;
        for m in messages {
            let role = m.role.clone();
            let message_timestamp = m.timestamp.unwrap_or(now_ts);
            let reasoning = if role == "assistant" {
                scrub_surrogates(m.reasoning.clone())
            } else {
                None
            };
            let reasoning_content = if role == "assistant" {
                scrub_surrogates(m.reasoning_content.clone())
            } else {
                None
            };
            let reasoning_details = if role == "assistant" {
                m.reasoning_details.as_ref()
            } else {
                None
            };
            let codex_reasoning_items = if role == "assistant" {
                m.codex_reasoning_items.as_ref()
            } else {
                None
            };
            let codex_message_items = if role == "assistant" {
                m.codex_message_items.as_ref()
            } else {
                None
            };
            let reasoning_details_json = reasoning_details
                .filter(|v| truthy(Some(*v)))
                .map(|v| v.to_string());
            let codex_items_json = codex_reasoning_items
                .filter(|v| truthy(Some(*v)))
                .map(|v| v.to_string());
            let codex_message_items_json = codex_message_items
                .filter(|v| truthy(Some(*v)))
                .map(|v| v.to_string());
            let tool_calls = parse_tool_calls(m.tool_calls.as_ref()).filter(|v| truthy(Some(v)));
            let tool_calls_json = tool_calls.as_ref().map(|v| v.to_string());
            // platform_message_id (new name) or message_id (yuanbao).
            let platform_msg_id = m
                .platform_message_id
                .clone()
                .or_else(|| m.message_id.clone());
            let api_content = scrub_surrogates(m.api_content.clone());
            let display_kind = scrub_surrogates(m.display_kind.clone());
            let tool_name = scrub_surrogates(m.tool_name.clone());

            conn.execute(
                "INSERT INTO messages (session_id, role, content, tool_call_id,
                   tool_calls, tool_name, effect_disposition, timestamp, token_count, finish_reason,
                   reasoning, reasoning_content, reasoning_details, codex_reasoning_items,
                   codex_message_items, platform_message_id, observed, active, api_content, display_kind, display_metadata)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    session_id,
                    m.role,
                    encode_content(m.content.as_ref()),
                    m.tool_call_id,
                    tool_calls_json,
                    tool_name,
                    m.effect_disposition,
                    message_timestamp,
                    m.token_count,
                    m.finish_reason,
                    reasoning,
                    reasoning_content,
                    reasoning_details_json,
                    codex_items_json,
                    codex_message_items_json,
                    platform_msg_id,
                    if m.observed { 1 } else { 0 },
                    1,
                    api_content,
                    display_kind,
                    encode_display_metadata(m.display_metadata.as_ref()),
                ],
            )?;
            inserted += 1;
            if let Some(tc) = &tool_calls {
                tool_calls_total += match tc {
                    Value::Array(a) => a.len() as i64,
                    _ => 1,
                };
            }
            // Upstream monotonic bump: max(now_ts + 1e-6, msg_ts + 1e-6).
            now_ts = (now_ts + 1e-6).max(message_timestamp + 1e-6);
        }
        Ok((inserted, tool_calls_total))
    }

    /// Load messages for a session in insertion order.
    /// PARITY: SessionDB.get_messages @ b9aa928 (7349–7401)
    pub fn get_messages(
        &self,
        session_id: &str,
        include_inactive: bool,
        limit: Option<i64>,
        offset: i64,
    ) -> Result<Vec<StoredMessage>, WriteError> {
        let active_clause = if include_inactive {
            ""
        } else {
            " AND active = 1"
        };
        let mut sql = format!(
            "SELECT * FROM messages WHERE session_id = ?{} ORDER BY id",
            active_clause
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(session_id.to_string())];
        if limit.is_some() || offset != 0 {
            sql += " LIMIT ? OFFSET ?";
            params.push(Box::new(limit.unwrap_or(-1)));
            params.push(Box::new(offset));
        }
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|x| x as &dyn rusqlite::ToSql)),
                |r| {
                    let content: Option<rusqlite::types::Value> = r.get("content")?;
                    let tool_calls: Option<String> = r.get("tool_calls")?;
                    let display_metadata: Option<String> = r.get("display_metadata")?;
                    Ok(StoredMessage {
                        id: r.get("id")?,
                        session_id: r.get("session_id")?,
                        role: r.get("role")?,
                        content: decode_content(content),
                        tool_call_id: r.get("tool_call_id")?,
                        tool_calls: match tool_calls.as_deref() {
                            Some(s) if !s.is_empty() => {
                                match serde_json::from_str::<Value>(s) {
                                    Ok(v) => Some(v),
                                    // get_messages falls back to [] on decode error
                                    Err(_) => Some(Value::Array(vec![])),
                                }
                            }
                            _ => None,
                        },
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
                        observed: r.get::<_, i64>("observed")? != 0,
                        active: r.get::<_, i64>("active")? != 0,
                        compacted: r.get::<_, i64>("compacted")? != 0,
                        api_content: r.get("api_content")?,
                        display_kind: r.get("display_kind")?,
                        display_metadata: decode_display_metadata(display_metadata.as_deref()),
                    })
                },
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows)
    }

    /// Row id of the most recent active message with *role*, or None.
    /// PARITY: SessionDB.latest_message_row_id @ b9aa928 (7013–7047)
    pub fn latest_message_row_id(
        &self,
        session_id: &str,
        role: &str,
        offset: i64,
        require_text: bool,
    ) -> Result<Option<i64>, WriteError> {
        if session_id.is_empty() || !matches!(role, "user" | "assistant") || offset < 0 {
            return Ok(None);
        }
        let text_filter = if require_text {
            "AND content IS NOT NULL AND TRIM(content) != '' "
        } else {
            ""
        };
        let conn = self.writer_conn();
        let row: Option<i64> = conn
            .query_row(
                &format!(
                    "SELECT id FROM messages WHERE session_id = ? AND role = ? \
                     AND active = 1 {}ORDER BY id DESC LIMIT 1 OFFSET ?",
                    text_filter
                ),
                rusqlite::params![session_id, role, offset],
                |r| r.get(0),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(row)
    }

    /// Row id of the most recent active user message, or None.
    /// PARITY: SessionDB.latest_user_message_row_id @ b9aa928 (7047–7055)
    pub fn latest_user_message_row_id(&self, session_id: &str) -> Result<Option<i64>, WriteError> {
        self.latest_message_row_id(session_id, "user", 0, true)
    }

    /// Role of the active message at *row_id* in *session_id*, or None.
    /// PARITY: SessionDB.get_message_role @ b9aa928 (7056–7073)
    pub fn get_message_role(
        &self,
        session_id: &str,
        row_id: i64,
    ) -> Result<Option<String>, WriteError> {
        if session_id.is_empty() {
            return Ok(None);
        }
        let conn = self.writer_conn();
        let row: Option<String> = conn
            .query_row(
                "SELECT role FROM messages WHERE id = ? AND session_id = ? AND active = 1",
                rusqlite::params![row_id, session_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(row)
    }
}

// ── dict-shaped rows (export/portability) ───────────────────────────────────

/// Full sessions row as a JSON object (all columns; `_system_prompt_resolved`
/// folded into `system_prompt`), mirroring `_session_row_dict`.
pub(crate) fn fold_session_dict(mut v: serde_json::Value) -> serde_json::Value {
    if let Some(resolved) = v.get("_system_prompt_resolved").cloned() {
        v.as_object_mut().unwrap().remove("_system_prompt_resolved");
        if let Some(obj) = v.as_object_mut() {
            if obj.contains_key("system_prompt") {
                obj.insert("system_prompt".to_string(), resolved);
            }
        }
    }
    v
}

impl SessionDB {
    /// Full session row as a JSON object with the system prompt resolved —
    /// the exact `get_session` dict shape (`_session_row_dict`).
    pub fn get_session_dict(
        &self,
        session_id: &str,
    ) -> Result<Option<serde_json::Value>, WriteError> {
        let sql = format!("{} WHERE s.id = ?", SESSION_SELECT);
        let conn = self.writer_conn();
        let row = conn
            .query_row(
                &sql,
                rusqlite::params![session_id],
                super::portability::row_to_value,
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(row.map(fold_session_dict))
    }

    /// Messages rows as JSON objects exactly like `get_messages` returns
    /// them upstream (decoded content/tool_calls/display_metadata; raw
    /// 0/1 ints for observed/active/compacted), used by export/portability.
    /// PARITY: hermes_state.py SessionDB.get_messages @ b9aa928 (7349–7401)
    pub fn get_messages_dicts(
        &self,
        session_id: &str,
        include_inactive: bool,
        limit: Option<i64>,
        offset: i64,
    ) -> Result<Vec<serde_json::Value>, WriteError> {
        let active_clause = if include_inactive {
            ""
        } else {
            " AND active = 1"
        };
        let mut sql = format!(
            "SELECT * FROM messages WHERE session_id = ?{} ORDER BY id",
            active_clause
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(session_id.to_string())];
        if limit.is_some() || offset != 0 {
            sql += " LIMIT ? OFFSET ?";
            params.push(Box::new(limit.unwrap_or(-1)));
            params.push(Box::new(offset));
        }
        let conn = self.writer_conn();
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|x| x as &dyn rusqlite::ToSql)),
                super::portability::message_row_to_value,
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        Ok(rows)
    }
}

impl SessionDB {
    /// Load a window of messages anchored on a specific message id.
    ///
    /// PARITY: hermes_state.py SessionDB.get_messages_around @ b9aa928
    /// (7401–7474). Returns `{window, messages_before, messages_after}`;
    /// empty window when the anchor is not a real id in the session.
    pub fn get_messages_around(
        &self,
        session_id: &str,
        around_message_id: i64,
        window: i64,
    ) -> Result<serde_json::Value, WriteError> {
        let window = window.max(0);
        let conn = self.writer_conn();
        let anchor_exists: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM messages WHERE id = ? AND session_id = ? LIMIT 1",
                rusqlite::params![around_message_id, session_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        if anchor_exists.is_none() {
            return Ok(serde_json::json!({
                "window": [],
                "messages_before": 0,
                "messages_after": 0,
            }));
        }
        // before: id <= anchor DESC limit window+1; after: id > anchor ASC
        // limit window. Final order id ASC.
        let mut before_stmt = conn
            .prepare(
                "SELECT * FROM messages WHERE session_id = ? AND id <= ? ORDER BY id DESC LIMIT ?",
            )
            .map_err(WriteError::Sqlite)?;
        let before = before_stmt
            .query_map(
                rusqlite::params![session_id, around_message_id, window + 1],
                super::portability::message_row_to_value,
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        let mut after_stmt = conn
            .prepare(
                "SELECT * FROM messages WHERE session_id = ? AND id > ? ORDER BY id ASC LIMIT ?",
            )
            .map_err(WriteError::Sqlite)?;
        let after = after_stmt
            .query_map(
                rusqlite::params![session_id, around_message_id, window],
                super::portability::message_row_to_value,
            )
            .map_err(WriteError::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(WriteError::Sqlite)?;
        // before is DESC (window+1 rows incl. the anchor); reverse to ASC and
        // concatenate after rows. The result is the full anchored window.
        let mut rows = before;
        rows.reverse();
        let before_rows = rows.len();
        rows.extend(after);
        let after_rows_selected = rows.len() - before_rows;
        // Counts mirror upstream's LIMIT-cap semantics: messages_before is
        // `window` unless the anchor sits near the session head; likewise
        // messages_after near the tail.
        let messages_before = if before_rows >= (window as usize + 1) {
            window
        } else {
            (before_rows as i64) - 1 // minus the anchor itself
        };
        let messages_after = if after_rows_selected >= window as usize {
            window
        } else {
            after_rows_selected as i64
        };
        Ok(serde_json::json!({
            "window": rows,
            "messages_before": messages_before.max(0),
            "messages_after": messages_after.max(0),
        }))
    }
}
