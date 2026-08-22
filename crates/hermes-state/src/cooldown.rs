//! Compression-failure cooldown + anti-thrash counters on sessions.
//!
//! The cooldown columns (`compression_failure_cooldown_until` /
//! `compression_failure_error`) gate retry of failed compressions; the
//! streak/count columns (`compression_fallback_streak` /
//! `compression_ineffective_count`) are the durable half of the built-in
//! compressor's anti-thrash guard, persisted so a fresh compressor bound to
//! a resumed session inherits an armed/tripped guard across restarts
//! (#54923).
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   record_compression_failure_cooldown    (4040–4063)
//!   get_compression_failure_cooldown       (4065–4100)
//!   get_compression_failure_cooldown_row   (4102–4139)
//!   restore_compression_failure_cooldown_row (4141–4186)
//!   clear_compression_failure_cooldown     (4188–4206)
//!   get/set_compression_fallback_streak    (4208–4244)
//!   get/set_compression_ineffective_count  (4246–4280)

use rusqlite::OptionalExtension;
use serde_json::{json, Value};

use crate::state::{now, SessionDB, WriteError};

impl SessionDB {
    /// Persist the active compression-failure cooldown for a session.
    /// Fail-open: write errors are logged and swallowed.
    ///
    /// PARITY: SessionDB.record_compression_failure_cooldown @ b9aa928
    /// (4040–4063)
    pub fn record_compression_failure_cooldown(
        &self,
        session_id: &str,
        cooldown_until: f64,
        error: Option<&str>,
    ) {
        if session_id.is_empty() {
            return;
        }
        let sid = session_id.to_string();
        let error = error.map(str::to_string);
        let f = |conn: &rusqlite::Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET compression_failure_cooldown_until = ?, \
                 compression_failure_error = ? WHERE id = ?",
                rusqlite::params![cooldown_until, error, sid],
            )?;
            Ok(())
        };
        if let Err(e) = self.execute_write(&f, None) {
            eprintln!(
                "[hermes-state] WARN: record_compression_failure_cooldown({}) failed: {}",
                session_id, e
            );
        }
    }

    /// Return the active (unexpired) compression-failure cooldown for
    /// ``session_id``, or None when absent/expired.
    ///
    /// PARITY: SessionDB.get_compression_failure_cooldown @ b9aa928
    /// (4065–4100)
    pub fn get_compression_failure_cooldown(
        &self,
        session_id: &str,
    ) -> Result<Option<Value>, WriteError> {
        if session_id.is_empty() {
            return Ok(None);
        }
        let conn = self.writer_conn();
        let row = conn
            .query_row(
                "SELECT compression_failure_cooldown_until, compression_failure_error \
                 FROM sessions WHERE id = ?",
                rusqlite::params![session_id],
                |r| {
                    Ok((
                        r.get::<_, Option<f64>>(0)?,
                        r.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        let Some((cooldown_until, error)) = row else {
            return Ok(None);
        };
        let Some(cooldown_until) = cooldown_until else {
            return Ok(None);
        };
        if cooldown_until <= now() {
            return Ok(None);
        }
        Ok(Some(json!({
            "cooldown_until": cooldown_until,
            "remaining_seconds": cooldown_until - now(),
            "error": error,
        })))
    }

    /// Return the exact stored cooldown columns without expiry filtering.
    ///
    /// PARITY: SessionDB.get_compression_failure_cooldown_row @ b9aa928
    /// (4102–4139)
    pub fn get_compression_failure_cooldown_row(
        &self,
        session_id: &str,
    ) -> Result<Value, WriteError> {
        if session_id.is_empty() {
            return Ok(json!({
                "session_exists": false,
                "cooldown_until": Value::Null,
                "error": Value::Null,
            }));
        }
        let conn = self.writer_conn();
        let row = conn
            .query_row(
                "SELECT compression_failure_cooldown_until, compression_failure_error \
                 FROM sessions WHERE id = ?",
                rusqlite::params![session_id],
                |r| {
                    Ok((
                        r.get::<_, Option<f64>>(0)?,
                        r.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        let Some((cooldown_until, error)) = row else {
            return Ok(json!({
                "session_exists": false,
                "cooldown_until": Value::Null,
                "error": Value::Null,
            }));
        };
        Ok(json!({
            "session_exists": true,
            "cooldown_until": cooldown_until,
            "error": error,
        }))
    }

    /// Restore and verify an exact cooldown-row snapshot. Unlike the
    /// ordinary record/clear helpers, this transactional rollback API
    /// deliberately propagates write and verification failures.
    ///
    /// PARITY: SessionDB.restore_compression_failure_cooldown_row @ b9aa928
    /// (4141–4186)
    pub fn restore_compression_failure_cooldown_row(
        &self,
        session_id: &str,
        snapshot: &Value,
    ) -> Result<(), WriteError> {
        let expected_exists = snapshot
            .get("session_exists")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !expected_exists {
            let actual = self.get_compression_failure_cooldown_row(session_id)?;
            if actual
                .get("session_exists")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return Err(WriteError::Runtime(
                    "cannot restore absent compression cooldown row: session now exists"
                        .to_string(),
                ));
            }
            return Ok(());
        }

        let deadline = snapshot.get("cooldown_until").and_then(Value::as_f64);
        let error = snapshot
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_string);
        let sid = session_id.to_string();
        let f = |conn: &rusqlite::Connection| -> Result<(), WriteError> {
            let rowcount = conn.execute(
                "UPDATE sessions SET compression_failure_cooldown_until = ?, \
                 compression_failure_error = ? WHERE id = ?",
                rusqlite::params![deadline, error, sid],
            )?;
            if rowcount != 1 {
                return Err(WriteError::Runtime(format!(
                    "compression cooldown rollback session missing: {session_id}"
                )));
            }
            Ok(())
        };
        self.execute_write(&f, None)?;

        let actual = self.get_compression_failure_cooldown_row(session_id)?;
        let expected = json!({
            "session_exists": true,
            "cooldown_until": deadline,
            "error": error,
        });
        if actual != expected {
            return Err(WriteError::Runtime(format!(
                "compression cooldown rollback verification failed: \
                 expected={expected}, actual={actual}"
            )));
        }
        Ok(())
    }

    /// Clear any persisted compression-failure cooldown. Fail-open: write
    /// errors are logged and swallowed.
    ///
    /// PARITY: SessionDB.clear_compression_failure_cooldown @ b9aa928
    /// (4188–4206)
    pub fn clear_compression_failure_cooldown(&self, session_id: &str) {
        if session_id.is_empty() {
            return;
        }
        let sid = session_id.to_string();
        let f = |conn: &rusqlite::Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET compression_failure_cooldown_until = NULL, \
                 compression_failure_error = NULL WHERE id = ?",
                rusqlite::params![sid],
            )?;
            Ok(())
        };
        if let Err(e) = self.execute_write(&f, None) {
            eprintln!(
                "[hermes-state] WARN: clear_compression_failure_cooldown({}) failed: {}",
                session_id, e
            );
        }
    }

    /// Return the persisted deterministic-fallback streak (clamped ≥ 0).
    ///
    /// PARITY: SessionDB.get_compression_fallback_streak @ b9aa928
    /// (4208–4226)
    pub fn get_compression_fallback_streak(&self, session_id: &str) -> Result<i64, WriteError> {
        if session_id.is_empty() {
            return Ok(0);
        }
        let conn = self.writer_conn();
        let value: Option<i64> = conn
            .query_row(
                "SELECT compression_fallback_streak FROM sessions WHERE id = ?",
                rusqlite::params![session_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(value.unwrap_or(0).max(0))
    }

    /// Persist the deterministic-fallback streak for one session (`streak`
    /// normalized to ≥ 0).
    ///
    /// PARITY: SessionDB.set_compression_fallback_streak @ b9aa928
    /// (4232–4244)
    pub fn set_compression_fallback_streak(
        &self,
        session_id: &str,
        streak: i64,
    ) -> Result<(), WriteError> {
        if session_id.is_empty() {
            return Ok(());
        }
        let normalized = streak.max(0);
        let sid = session_id.to_string();
        let f = |conn: &rusqlite::Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET compression_fallback_streak = ? WHERE id = ?",
                rusqlite::params![normalized, sid],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Return the persisted ineffective-compaction strike count (clamped
    /// ≥ 0).
    ///
    /// PARITY: SessionDB.get_compression_ineffective_count @ b9aa928
    /// (4246–4275)
    pub fn get_compression_ineffective_count(&self, session_id: &str) -> Result<i64, WriteError> {
        if session_id.is_empty() {
            return Ok(0);
        }
        let conn = self.writer_conn();
        let value: Option<i64> = conn
            .query_row(
                "SELECT compression_ineffective_count FROM sessions WHERE id = ?",
                rusqlite::params![session_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(WriteError::Sqlite)?;
        Ok(value.unwrap_or(0).max(0))
    }

    /// Persist the ineffective-compaction strike count for one session
    /// (`count` normalized to ≥ 0).
    ///
    /// PARITY: SessionDB.set_compression_ineffective_count @ b9aa928
    /// (4277–4280)
    pub fn set_compression_ineffective_count(
        &self,
        session_id: &str,
        count: i64,
    ) -> Result<(), WriteError> {
        if session_id.is_empty() {
            return Ok(());
        }
        let normalized = count.max(0);
        let sid = session_id.to_string();
        let f = |conn: &rusqlite::Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET compression_ineffective_count = ? WHERE id = ?",
                rusqlite::params![normalized, sid],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }
}
