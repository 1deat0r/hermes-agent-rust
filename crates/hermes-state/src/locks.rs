//! Compression lock lifecycle + compression publication/recovery.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   _compression_lock_holder_process_is_dead      (94–138)
//!   find_live_compression_child                   (3665–3710)
//!   reopen_orphaned_compression_session           (3711–3791)
//!   publish_compression_child                     (3792–3880)
//!   refresh_compression_lock / try_acquire /
//!   release / get_compression_lock_holder          (4312–4516)

use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::{Connection, OptionalExtension};

use crate::common;
use crate::schema;
use crate::state::now;

use super::crud::{MessageInput, SessionRow};
use super::state::{SessionDB, WriteError};

/// `(?:^|:)pid=(\d+)(?::|$)` — the structured holder id convention
/// (`pid=<n>:...`) used by conversation_compression.
static HOLDER_PID_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:^|:)pid=(\d+)(?::|$)").unwrap());

/// True only when a structured lock holder's local PID is gone.
///
/// PARITY: hermes_state.py _compression_lock_holder_process_is_dead @ b9aa928.
/// Upstream prefers psutil when present and falls back to os.kill(pid, 0) on
/// POSIX without psutil; this Rust build always uses the POSIX libc probe
/// (the psutil fast path is unavailable here, and the conservative fallback
/// semantics are identical). Windows hosts stay TTL-only, exactly like
/// upstream's `os.name == "nt"` early return.
fn compression_lock_holder_process_is_dead(holder: &str) -> bool {
    let Some(caps) = HOLDER_PID_RE.captures(holder) else {
        return false;
    };
    let Ok(pid) = caps.get(1).unwrap().as_str().parse::<i32>() else {
        return false;
    };
    if pid <= 0 {
        return false;
    }
    if pid as u32 == std::process::id() {
        // Same-process holder (e.g. another thread's live lease): never
        // self-reclaim — the lease refresher and release path own it.
        return false;
    }
    #[cfg(windows)]
    {
        // bpo-14484: os.kill(pid, 0) is not a no-op probe on Windows. Without
        // psutil a Windows host stays TTL-only; the lease TTL remains the
        // recovery path.
        let _ = pid;
        return false;
    }
    #[cfg(not(windows))]
    {
        // os.kill(pid, 0) semantics via libc: ESRCH => no such process.
        let r = unsafe { libc::kill(pid, 0) };
        if r == 0 {
            return false;
        }
        let err = std::io::Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::ESRCH) => true,
            // PermissionError / EINVAL / any other doubt: keep the lease
            // until TTL expiry (PID reuse must never steal a live lease).
            _ => false,
        }
    }
}

impl SessionDB {
    /// Extend the compression lock lease if `holder` still owns it.
    ///
    /// PARITY: hermes_state.py SessionDB.refresh_compression_lock @ b9aa928.
    /// Ownership is decided by the `holder` column alone, deliberately NOT by
    /// `expires_at`: a live owner whose refresher thread was starved past its
    /// own TTL must be able to revive its still-unclaimed row.
    pub fn refresh_compression_lock(
        &self,
        session_id: &str,
        holder: &str,
        ttl_seconds: f64,
    ) -> bool {
        if session_id.is_empty() || holder.is_empty() {
            return false;
        }
        let sid_log = session_id.to_string();
        let expires_at = now() + ttl_seconds;
        let f = move |conn: &Connection| -> Result<bool, WriteError> {
            let changed = conn.execute(
                "UPDATE compression_locks SET expires_at = ? \
                 WHERE session_id = ? AND holder = ?",
                rusqlite::params![expires_at, session_id, holder],
            )?;
            Ok(changed > 0)
        };
        match self.execute_write(&f, None) {
            Ok(v) => v,
            Err(WriteError::Sqlite(e)) => {
                log_warn(&format!(
                    "refresh_compression_lock({}) failed: {}",
                    sid_log, e
                ));
                false
            }
            Err(_) => false,
        }
    }

    /// Try to atomically acquire the compression lock for `session_id`.
    ///
    /// PARITY: hermes_state.py SessionDB.try_acquire_compression_lock @
    /// b9aa928. Expired locks and dead structured holders are reclaimed in
    /// the same BEGIN IMMEDIATE transaction as the INSERT OR IGNORE.
    pub fn try_acquire_compression_lock(
        &self,
        session_id: &str,
        holder: &str,
        ttl_seconds: f64,
    ) -> bool {
        if session_id.is_empty() {
            return false;
        }
        let now_ts = now();
        let expires_at = now_ts + ttl_seconds;
        let sid_log = session_id.to_string();
        let session_id = session_id.to_string();
        let holder = holder.to_string();
        let f = move |conn: &Connection| -> Result<(bool, Option<String>), WriteError> {
            let mut reclaimed_holder = None;
            let row = conn
                .query_row(
                    "SELECT holder, expires_at FROM compression_locks \
                     WHERE session_id = ?",
                    rusqlite::params![session_id],
                    |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<f64>>(1)?)),
                )
                .optional()?;
            if let Some((current_holder, current_expires_at)) = row {
                let expired = match current_expires_at {
                    Some(exp) => exp < now_ts,
                    None => false,
                };
                if expired
                    || compression_lock_holder_process_is_dead(
                        current_holder.as_deref().unwrap_or(""),
                    )
                {
                    conn.execute(
                        "DELETE FROM compression_locks \
                         WHERE session_id = ? AND holder = ?",
                        rusqlite::params![session_id, current_holder],
                    )?;
                    reclaimed_holder = current_holder;
                }
            }
            // INSERT OR IGNORE returns no rowcount difference — verify
            // ownership via SELECT.
            conn.execute(
                "INSERT OR IGNORE INTO compression_locks \
                 (session_id, holder, acquired_at, expires_at) \
                 VALUES (?, ?, ?, ?)",
                rusqlite::params![session_id, holder, now_ts, expires_at],
            )?;
            let row_holder: Option<String> = conn
                .query_row(
                    "SELECT holder FROM compression_locks WHERE session_id = ?",
                    rusqlite::params![session_id],
                    |r| r.get(0),
                )
                .optional()?;
            let acquired = row_holder.as_deref() == Some(holder.as_str());
            Ok((acquired, reclaimed_holder))
        };
        match self.execute_write(&f, None) {
            Ok((acquired, reclaimed_holder)) => {
                if let Some(reclaimed) = reclaimed_holder {
                    log_warn(&format!(
                        "Reclaimed stale compression lock for session={} (holder={})",
                        sid_log, reclaimed
                    ));
                }
                acquired
            }
            Err(WriteError::Sqlite(e)) => {
                log_warn(&format!(
                    "try_acquire_compression_lock({}) failed: {}",
                    sid_log, e
                ));
                // Fail open: returning False makes the caller skip
                // compression, which is safe when the lock subsystem breaks.
                false
            }
            Err(_) => false,
        }
    }

    /// Release the compression lock for `session_id` iff we own it.
    ///
    /// PARITY: hermes_state.py SessionDB.release_compression_lock @ b9aa928.
    /// Idempotent; the `holder` check prevents a late-returning compressor
    /// from clobbering a fresh lock held by someone else.
    pub fn release_compression_lock(&self, session_id: &str, holder: &str) {
        if session_id.is_empty() {
            return;
        }
        let sid_log = session_id.to_string();
        let f = move |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "DELETE FROM compression_locks \
                 WHERE session_id = ? AND holder = ?",
                rusqlite::params![session_id, holder],
            )?;
            Ok(())
        };
        match self.execute_write(&f, None) {
            Ok(()) => {}
            Err(WriteError::Sqlite(e)) => {
                log_warn(&format!(
                    "release_compression_lock({}) failed: {}",
                    sid_log, e
                ));
            }
            Err(_) => {}
        }
    }

    /// Return the current (non-expired) holder for `session_id`, or None.
    ///
    /// PARITY: hermes_state.py SessionDB.get_compression_lock_holder @
    /// b9aa928. Diagnostic helper — not used by the locking protocol.
    pub fn get_compression_lock_holder(&self, session_id: &str) -> Option<String> {
        if session_id.is_empty() {
            return None;
        }
        let now_ts = now();
        let conn = self.writer_conn();
        conn.query_row(
            "SELECT holder FROM compression_locks \
             WHERE session_id = ? AND expires_at >= ?",
            rusqlite::params![session_id, now_ts],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
    }

    /// Return the unique live direct child of a compression-ended session.
    ///
    /// PARITY: hermes_state.py SessionDB.find_live_compression_child @
    /// b9aa928. Multiple children are ambiguous and fail closed.
    pub fn find_live_compression_child(
        &self,
        parent_session_id: &str,
    ) -> Result<Option<SessionRow>, WriteError> {
        if parent_session_id.is_empty() {
            return Ok(None);
        }
        let conn = self.writer_conn();
        let parent = conn
            .query_row(
                "SELECT ended_at, end_reason FROM sessions WHERE id = ?",
                rusqlite::params![parent_session_id],
                |r| Ok((r.get::<_, Option<f64>>(0)?, r.get::<_, Option<String>>(1)?)),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        let is_compression_ended = match parent {
            Some((ended_at, end_reason)) => {
                ended_at.is_some() && end_reason.as_deref() == Some("compression")
            }
            None => false,
        };
        if !is_compression_ended {
            return Ok(None);
        }
        let filter = common::_non_continuation_child_filter_sql("s.");
        let sql = format!(
            "SELECT s.*, \
                COALESCE(sp.prompt, s.system_prompt) AS _system_prompt_resolved \
             FROM sessions s \
             LEFT JOIN system_prompts sp ON sp.hash = s.system_prompt_hash \
             WHERE s.parent_session_id = ? \
               AND s.ended_at IS NULL \
             {} \
             ORDER BY s.started_at ASC \
             LIMIT 2",
            filter
        );
        let mut stmt = conn.prepare(&sql).map_err(WriteError::Sqlite)?;
        let mut rows = stmt
            .query_map(
                rusqlite::params![parent_session_id, parent_session_id, parent_session_id],
                super::crud::session_row,
            )
            .map_err(WriteError::Sqlite)?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().transpose().map_err(WriteError::Sqlite)? {
            out.push(r);
            if out.len() > 1 {
                return Ok(None);
            }
        }
        Ok(out.into_iter().next())
    }

    /// Reopen a compression parent only when no continuation was published.
    ///
    /// PARITY: hermes_state.py SessionDB.reopen_orphaned_compression_session
    /// @ b9aa928. Deliberately conservative: an active compression lease or
    /// any canonical child means the lineage is owned by another path, so the
    /// caller fails closed instead of reopening the parent.
    // Upstream has NO try/except here: sqlite3.Error propagates to the
    // caller (an interrupted recovery aborts the turn loop). Mirror that by
    // returning Result instead of collapsing to false.
    pub fn reopen_orphaned_compression_session(
        &self,
        session_id: &str,
    ) -> Result<bool, WriteError> {
        if session_id.is_empty() {
            return Ok(false);
        }
        let filter = common::_non_continuation_child_filter_sql("");
        let session_id = session_id.to_string();
        let f = move |conn: &Connection| -> Result<bool, WriteError> {
            let parent = conn
                .query_row(
                    "SELECT ended_at, end_reason FROM sessions WHERE id = ?",
                    rusqlite::params![session_id],
                    |r| Ok((r.get::<_, Option<f64>>(0)?, r.get::<_, Option<String>>(1)?)),
                )
                .optional()?;
            let is_orphan = match parent {
                Some((ended_at, end_reason)) => {
                    ended_at.is_some() && end_reason.as_deref() == Some("compression")
                }
                None => false,
            };
            if !is_orphan {
                return Ok(false);
            }
            // Treat any direct non-branch/non-delegate/non-tool child as a
            // continuation, regardless of its current ended state.
            let sql = format!(
                "SELECT 1 FROM sessions \
                 WHERE parent_session_id = ? {} \
                 LIMIT 1",
                filter
            );
            let child: Option<i64> = conn
                .query_row(
                    &sql,
                    rusqlite::params![session_id, session_id, session_id],
                    |r| r.get(0),
                )
                .optional()?;
            if child.is_some() {
                return Ok(false);
            }
            // refresh_compression_lock() deliberately lets an owner revive its
            // own expired row. Reclaim that row inside this write transaction
            // before reopening (see upstream comment about rowcount==1).
            let now_ts = now();
            let lock_row = conn
                .query_row(
                    "SELECT holder, expires_at FROM compression_locks \
                     WHERE session_id = ?",
                    rusqlite::params![session_id],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<f64>>(1)?)),
                )
                .optional()?;
            if let Some((holder, expires_at)) = lock_row {
                let active = match expires_at {
                    Some(exp) => exp >= now_ts,
                    None => true,
                };
                if active {
                    return Ok(false);
                }
                let deleted = conn.execute(
                    "DELETE FROM compression_locks \
                     WHERE session_id = ? AND holder = ? AND expires_at = ?",
                    rusqlite::params![session_id, holder, expires_at],
                )?;
                if deleted != 1 {
                    return Ok(false);
                }
            }
            let updated = conn.execute(
                "UPDATE sessions SET ended_at = NULL, end_reason = NULL \
                 WHERE id = ? AND ended_at IS NOT NULL \
                   AND end_reason = 'compression'",
                rusqlite::params![session_id],
            )?;
            Ok(updated == 1)
        };
        self.execute_write(&f, None)
    }

    /// Atomically close a parent and publish its durable compression child.
    ///
    /// PARITY: hermes_state.py SessionDB.publish_compression_child @
    /// b9aa928. The parent closure, child row, and compacted handoff become
    /// visible in one transaction.
    // Upstream is a keyword-only function; keep the flat positional mirror.
    #[allow(clippy::too_many_arguments)]
    pub fn publish_compression_child(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        source: &str,
        messages: &[MessageInput],
        model: Option<&str>,
        model_config: Option<&serde_json::Value>,
        system_prompt: Option<&str>,
        cwd: Option<&str>,
        profile_name: Option<&str>,
        compression_lock_holder: Option<&str>,
        require_compression_lease: bool,
    ) -> Result<(), WriteError> {
        let parent_session_id = parent_session_id.to_string();
        let child_session_id = child_session_id.to_string();
        let source = source.to_string();
        let model = model.map(str::to_string);
        // Python: `json.dumps(model_config) if model_config else None`
        // (an empty dict is falsy and stores NULL) — same convention as
        // create_session.
        let model_config = model_config
            .filter(|v| super::crud::truthy(Some(v)))
            .map(|v| v.to_string());
        let messages = messages.to_vec();

        let f = move |conn: &Connection| -> Result<(), WriteError> {
            if require_compression_lease {
                let lock_row = conn
                    .query_row(
                        "SELECT holder, expires_at FROM compression_locks \
                         WHERE session_id = ?",
                        rusqlite::params![parent_session_id],
                        |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<f64>>(1)?)),
                    )
                    .optional()?;
                let lease_valid = match lock_row {
                    Some((Some(holder), Some(expires_at))) => {
                        compression_lock_holder.is_some_and(|h| h == holder) && expires_at > now()
                    }
                    _ => false,
                };
                if !lease_valid {
                    return Err(WriteError::CompressionBusy(format!(
                        "Compression lease lost before publication: {}",
                        parent_session_id
                    )));
                }
            }
            let parent = conn
                .query_row(
                    "SELECT ended_at, cwd, git_branch, git_repo_root,
                            user_id, session_key, chat_id, chat_type,
                            thread_id, display_name, origin_json, profile_name
                     FROM sessions WHERE id = ?",
                    rusqlite::params![parent_session_id],
                    |r| {
                        Ok((
                            r.get::<_, Option<f64>>(0)?,
                            r.get::<_, Option<String>>(1)?,
                            r.get::<_, Option<String>>(2)?,
                            r.get::<_, Option<String>>(3)?,
                            r.get::<_, Option<String>>(4)?,
                            r.get::<_, Option<String>>(5)?,
                            r.get::<_, Option<String>>(6)?,
                            r.get::<_, Option<String>>(7)?,
                            r.get::<_, Option<String>>(8)?,
                            r.get::<_, Option<String>>(9)?,
                            r.get::<_, Option<String>>(10)?,
                            r.get::<_, Option<String>>(11)?,
                        ))
                    },
                )
                .optional()?;
            let Some(parent) = parent else {
                return Err(WriteError::Runtime(format!(
                    "Compression parent not found: {}",
                    parent_session_id
                )));
            };
            if parent.0.is_some() {
                return Err(WriteError::Runtime(format!(
                    "Compression parent already ended: {}",
                    parent_session_id
                )));
            }
            if messages.is_empty() {
                return Err(WriteError::Runtime(
                    "Compression child handoff must not be empty".to_string(),
                ));
            }
            let system_prompt_hash =
                schema::store_system_prompt(conn, system_prompt.map(str::to_string))?;

            conn.execute(
                "INSERT INTO sessions (
                   id, source, model, model_config, system_prompt,
                   system_prompt_hash,
                   parent_session_id, cwd, git_branch, git_repo_root,
                   profile_name, user_id, session_key, chat_id, chat_type,
                   thread_id, display_name, origin_json, started_at
                ) VALUES (?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    child_session_id,
                    source,
                    model,
                    model_config,
                    system_prompt_hash,
                    parent_session_id,
                    cwd.map(str::to_string).or(parent.1.clone()),
                    parent.2,
                    parent.3,
                    // Same inheritance contract as _insert_session_row's
                    // compression-fork backfill (#59527).
                    profile_name.map(str::to_string).or(parent.11),
                    parent.4,
                    parent.5,
                    parent.6,
                    parent.7,
                    parent.8,
                    parent.9,
                    parent.10,
                    now(),
                ],
            )?;
            let (total_messages, total_tool_calls) =
                SessionDB::insert_message_rows(conn, &child_session_id, &messages)?;
            conn.execute(
                "UPDATE sessions SET message_count = ?, tool_call_count = ? WHERE id = ?",
                rusqlite::params![total_messages, total_tool_calls, child_session_id],
            )?;
            let updated = conn.execute(
                "UPDATE sessions SET ended_at = ?, end_reason = 'compression' \
                 WHERE id = ? AND ended_at IS NULL",
                rusqlite::params![now(), parent_session_id],
            )?;
            if updated != 1 {
                return Err(WriteError::Runtime(format!(
                    "Compression parent changed during publication: {}",
                    parent_session_id
                )));
            }
            Ok(())
        };
        self.execute_write(&f, Some(SessionDB::TRANSCRIPT_WRITE_PATIENCE_S))
    }
}

fn log_warn(msg: &str) {
    // Hermes logging surfaces land in hermes-logging; keep this crate's
    // diagnostics to eprintln until the logging seam is wired in P2.
    eprintln!("[hermes-state] WARN: {}", msg);
}

// ── orphaned-compression finalization (#20001) ─────────────────────────────

impl SessionDB {
    /// Mark orphaned compression continuation sessions as ended. Child
    /// sessions that were never finalized (parent ended with 'compression',
    /// child has messages but no end_reason/ended_at and api_call_count=0)
    /// get end_reason='orphaned_compression'. Non-destructive.
    ///
    /// PARITY: SessionDB.finalize_orphaned_compression_sessions @ b9aa928
    /// (5515–5550)
    pub fn finalize_orphaned_compression_sessions(&self) -> Result<i64, WriteError> {
        let cutoff = crate::state::now() - 604800.0; // 7 days
        let f = |conn: &rusqlite::Connection| -> Result<i64, WriteError> {
            let now = crate::state::now();
            let rowcount = conn.execute(
                "UPDATE sessions \
                 SET ended_at = ?, end_reason = 'orphaned_compression' \
                 WHERE api_call_count = 0 \
                   AND end_reason IS NULL \
                   AND ended_at IS NULL \
                   AND started_at < ? \
                   AND parent_session_id IS NOT NULL \
                   AND EXISTS ( \
                       SELECT 1 FROM sessions p \
                       WHERE p.id = sessions.parent_session_id \
                         AND p.end_reason = 'compression' \
                         AND p.ended_at IS NOT NULL \
                   ) \
                   AND EXISTS ( \
                       SELECT 1 FROM messages m \
                       WHERE m.session_id = sessions.id \
                   )",
                rusqlite::params![now, cutoff],
            )?;
            Ok(rowcount as i64)
        };
        self.execute_write(&f, None)
    }
}
