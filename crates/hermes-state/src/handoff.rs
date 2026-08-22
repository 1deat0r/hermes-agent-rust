//! Cross-platform session handoff state machine.
//!
//! State machine (on the `sessions` table):
//!   None       — no handoff in flight
//!   "pending"  — CLI requested handoff, gateway hasn't picked it up yet
//!   "running"  — gateway is processing (session switch + synthetic turn)
//!   "completed"— gateway successfully delivered the synthetic turn
//!   "failed"   — gateway hit an error; reason in handoff_error
//!
//! PARITY: hermes_state.py @ b9aa928 (9881–9982)

use rusqlite::{Connection, OptionalExtension, Row};
use serde_json::Value;

use crate::crud;
use crate::state::{SessionDB, WriteError};

/// The handoff-state triple returned by `get_handoff_state`.
#[derive(Debug, Clone, Default)]
pub struct HandoffState {
    pub state: Option<String>,
    pub platform: Option<String>,
    pub error: Option<String>,
}

impl HandoffState {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<HandoffState> {
        Ok(HandoffState {
            state: row.get("handoff_state")?,
            platform: row.get("handoff_platform")?,
            error: row.get("handoff_error")?,
        })
    }

    /// The `{"state", "platform", "error"}` dict upstream returns.
    pub fn to_value(&self) -> Value {
        serde_json::json!({
            "state": self.state,
            "platform": self.platform,
            "error": self.error,
        })
    }
}

impl SessionDB {
    /// Mark a session as pending handoff to the given platform.
    ///
    /// PARITY: SessionDB.request_handoff @ b9aa928 (9890–9908)
    pub fn request_handoff(&self, session_id: &str, platform: &str) -> Result<bool, WriteError> {
        let session_id = session_id.to_string();
        let platform = platform.to_string();
        let f = |conn: &Connection| -> Result<bool, WriteError> {
            let rowcount = conn.execute(
                "UPDATE sessions \
                 SET handoff_state = 'pending', \
                     handoff_platform = ?, \
                     handoff_error = NULL \
                 WHERE id = ? AND (handoff_state IS NULL \
                                   OR handoff_state IN ('completed', 'failed'))",
                rusqlite::params![platform, session_id],
            )?;
            Ok(rowcount > 0)
        };
        self.execute_write(&f, None)
    }

    /// Read the current handoff state for a session (fail-open: a missing
    /// row or read error returns None, matching upstream's bare except).
    ///
    /// PARITY: SessionDB.get_handoff_state @ b9aa928 (9909–9930)
    pub fn get_handoff_state(&self, session_id: &str) -> Option<HandoffState> {
        let conn = self.writer_conn();
        conn.query_row(
            "SELECT handoff_state, handoff_platform, handoff_error \
             FROM sessions WHERE id = ?",
            rusqlite::params![session_id.to_string()],
            HandoffState::from_row,
        )
        .optional()
        .ok()
        .flatten()
    }

    /// All sessions in handoff_state='pending', oldest first (the gateway
    /// watcher's poll surface). Fail-open to [].
    ///
    /// PARITY: SessionDB.list_pending_handoffs @ b9aa928 (9931–9948)
    pub fn list_pending_handoffs(&self) -> Vec<Value> {
        let conn = self.writer_conn();
        let result = conn
            .prepare(
                "SELECT s.*, \
                 COALESCE(sp.prompt, s.system_prompt) AS _system_prompt_resolved \
                 FROM sessions s \
                 LEFT JOIN system_prompts sp ON sp.hash = s.system_prompt_hash \
                 WHERE s.handoff_state = 'pending' \
                 ORDER BY s.started_at ASC",
            )
            .and_then(|mut stmt| {
                let rows = stmt.query_map([], crate::portability::row_to_value)?;
                rows.collect::<Result<Vec<_>, _>>()
            });
        match result {
            Ok(rows) => rows.into_iter().map(crud::fold_session_dict).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Atomically transition pending → running. True if claimed.
    ///
    /// PARITY: SessionDB.claim_handoff @ b9aa928 (9949–9959)
    pub fn claim_handoff(&self, session_id: &str) -> Result<bool, WriteError> {
        let session_id = session_id.to_string();
        let f = |conn: &Connection| -> Result<bool, WriteError> {
            let rowcount = conn.execute(
                "UPDATE sessions SET handoff_state = 'running' \
                 WHERE id = ? AND handoff_state = 'pending'",
                rusqlite::params![session_id],
            )?;
            Ok(rowcount > 0)
        };
        self.execute_write(&f, None)
    }

    /// Mark a handoff as completed (clears error).
    ///
    /// PARITY: SessionDB.complete_handoff @ b9aa928 (9960–9969)
    pub fn complete_handoff(&self, session_id: &str) -> Result<(), WriteError> {
        let session_id = session_id.to_string();
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET handoff_state = 'completed', \
                 handoff_error = NULL WHERE id = ?",
                rusqlite::params![session_id],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Mark a handoff as failed and record the reason (truncated to 500
    /// chars like upstream `error[:500]`).
    ///
    /// PARITY: SessionDB.fail_handoff @ b9aa928 (9970–9981)
    pub fn fail_handoff(&self, session_id: &str, error: &str) -> Result<(), WriteError> {
        let session_id = session_id.to_string();
        let error: String = error.chars().take(500).collect();
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET handoff_state = 'failed', \
                 handoff_error = ? WHERE id = ?",
                rusqlite::params![error, session_id],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }
}
