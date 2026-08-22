//! Session meta / model surfaces — model_config JSON patches, model switches,
//! runtime locks, billing route, and the YOLO bypass flag.
//!
//! `_merge_model_config_json` is shared from rewrite.rs so every writer
//! keeps lineage markers (`_branched_from` / `_delegate_from`) alive.
//!
//! PARITY: hermes_state.py @ b9aa928 —
//!   update_session_meta                  (4599–4619)
//!   update_system_prompt                 (4621–4633)
//!   update_session_model                 (4635–4670)
//!   patch_session_model_config           (4724–4743)
//!   get_session_model_config_value       (4745–4761)
//!   update_session_runtime_lock          (4763–4804)
//!   set_session_yolo / session_yolo_enabled (4806–4849)
//!   update_session_billing_route         (4851–4886)

use rusqlite::Connection;
use serde_json::{Map, Value};

use crate::crud::delete_unreferenced_system_prompts;
use crate::rewrite::merge_model_config_json;
use crate::schema::store_system_prompt;
use crate::state::{now, SessionDB, WriteError};

impl SessionDB {
    /// Update model_config and optionally model for an existing session.
    ///
    /// PARITY: SessionDB.update_session_meta @ b9aa928 (4599–4619)
    pub fn update_session_meta(
        &self,
        session_id: &str,
        model_config_json: &str,
        model: Option<&str>,
    ) -> Result<(), WriteError> {
        // Barrier against queued token deltas — see update_session_model.
        self.flush_token_counts(5.0);
        let sid = session_id.to_string();
        let model_config_json = model_config_json.to_string();
        let model = model.map(str::to_string);
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET model_config = ?, model = COALESCE(?, model) WHERE id = ?",
                rusqlite::params![model_config_json, model, sid],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Store the full assembled system prompt snapshot.
    ///
    /// PARITY: SessionDB.update_system_prompt @ b9aa928 (4621–4633)
    pub fn update_system_prompt(
        &self,
        session_id: &str,
        system_prompt: Option<&str>,
    ) -> Result<(), WriteError> {
        let sid = session_id.to_string();
        let system_prompt = system_prompt.map(str::to_string);
        let f = |conn: &Connection| -> Result<(), WriteError> {
            let system_prompt_hash = store_system_prompt(conn, system_prompt.clone())?;
            conn.execute(
                "UPDATE sessions SET system_prompt_hash = ?, system_prompt = NULL WHERE id = ?",
                rusqlite::params![system_prompt_hash, sid],
            )?;
            delete_unreferenced_system_prompts(conn)?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Update the model for a session after a mid-session switch.
    ///
    /// PARITY: SessionDB.update_session_model @ b9aa928 (4635–4670)
    pub fn update_session_model(&self, session_id: &str, model: &str) -> Result<(), WriteError> {
        // A /model switch must land after queued deltas so a still-queued
        // pre-switch delta cannot resurrect the old model/provider via the
        // first_accounted_route overwrite.
        self.flush_token_counts(5.0);
        let sid = session_id.to_string();
        let model = model.to_string();
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET \
                 model = ?, \
                 model_config = CASE \
                     WHEN model_config IS NULL THEN NULL \
                     WHEN json_valid(model_config) \
                         THEN json_remove(model_config, '$.browser_model_lock') \
                     ELSE model_config \
                 END, \
                 system_prompt = NULL, \
                 system_prompt_hash = NULL \
                 WHERE id = ?",
                rusqlite::params![model, sid],
            )?;
            delete_unreferenced_system_prompts(conn)?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Merge `patch` into a session's model_config JSON atomically.
    ///
    /// PARITY: SessionDB.patch_session_model_config @ b9aa928 (4724–4743)
    pub fn patch_session_model_config(
        &self,
        session_id: &str,
        patch: &Map<String, Value>,
    ) -> Result<(), WriteError> {
        if session_id.is_empty() || patch.is_empty() {
            return Ok(());
        }
        let sid = session_id.to_string();
        let patch = patch.clone();
        let f = |conn: &Connection| -> Result<(), WriteError> {
            let merged = merge_model_config_json(conn, &sid, &patch, false)?;
            conn.execute(
                "UPDATE sessions SET model_config = ? WHERE id = ?",
                rusqlite::params![merged, sid],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Read one key out of a session's model_config JSON (tolerant parse).
    ///
    /// PARITY: SessionDB.get_session_model_config_value @ b9aa928
    /// (4745–4761)
    pub fn get_session_model_config_value(
        &self,
        session_id: &str,
        key: &str,
        default: Option<Value>,
    ) -> Result<Value, WriteError> {
        let session = self.get_session(session_id)?;
        let raw = session.and_then(|s| s.model_config);
        let config = match raw {
            Some(raw) if !raw.trim().is_empty() => {
                serde_json::from_str::<Value>(&raw)
                    .ok()
                    .and_then(|v| v.as_object().cloned())
                    .unwrap_or_default()
            }
            _ => Map::new(),
        };
        Ok(config.get(key).cloned().unwrap_or(default.unwrap_or(Value::Null)))
    }

    /// Persist a Browser / API client runtime lock without clobbering
    /// lineage markers.
    ///
    /// PARITY: SessionDB.update_session_runtime_lock @ b9aa928 (4763–4804)
    #[allow(clippy::too_many_arguments)]
    pub fn update_session_runtime_lock(
        &self,
        session_id: &str,
        model: Option<&str>,
        provider: Option<&str>,
        model_options: Option<&Map<String, Value>>,
        route_source: Option<&str>,
        confirmed: bool,
    ) -> Result<(), WriteError> {
        let lock = serde_json::json!({
            "provider": provider.unwrap_or(""),
            "model": model.unwrap_or(""),
            "model_options": model_options.cloned().unwrap_or_default(),
            "route_source": route_source.unwrap_or(""),
            "confirmed": confirmed,
            "updated_at": now(),
        });
        let sid = session_id.to_string();
        let model = model.map(str::to_string);
        let f = |conn: &Connection| -> Result<(), WriteError> {
            let mut patch = Map::new();
            patch.insert("browser_model_lock".to_string(), lock.clone());
            let merged = merge_model_config_json(conn, &sid, &patch, false)?;
            conn.execute(
                "UPDATE sessions SET \
                 model_config = ?, \
                 model = COALESCE(?, model), \
                 system_prompt = NULL, \
                 system_prompt_hash = NULL \
                 WHERE id = ?",
                rusqlite::params![merged, model, sid],
            )?;
            delete_unreferenced_system_prompts(conn)?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Persist the per-session YOLO bypass flag into `model_config`.
    ///
    /// PARITY: SessionDB.set_session_yolo @ b9aa928 (4806–4832)
    pub fn set_session_yolo(&self, session_id: &str, enabled: bool) -> Result<(), WriteError> {
        if session_id.is_empty() {
            return Ok(());
        }
        let sid = session_id.to_string();
        let f = |conn: &Connection| -> Result<(), WriteError> {
            let mut patch = Map::new();
            patch.insert("yolo_mode".to_string(), Value::Bool(enabled));
            let merged = merge_model_config_json(conn, &sid, &patch, false)?;
            conn.execute(
                "UPDATE sessions SET model_config = ? WHERE id = ?",
                rusqlite::params![merged, sid],
            )?;
            Ok(())
        };
        self.execute_write(&f, None)
    }

    /// Read the persisted YOLO flag off a session row dict. Returns False on
    /// any parse failure — resume must never enable the bypass by accident.
    ///
    /// PARITY: SessionDB.session_yolo_enabled @ b9aa928 (4834–4849)
    pub fn session_yolo_enabled(session_meta: Option<&Value>) -> bool {
        let Some(raw) = session_meta.and_then(|m| m.get("model_config")) else {
            return false;
        };
        let parsed = match raw {
            Value::String(s) => {
                if s.trim().is_empty() {
                    return false;
                }
                match serde_json::from_str::<Value>(s) {
                    Ok(v) => v,
                    Err(_) => return false,
                }
            }
            other => other.clone(),
        };
        parsed
            .get("yolo_mode")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    /// Unconditionally update the billing provider/base_url for a session.
    ///
    /// PARITY: SessionDB.update_session_billing_route @ b9aa928 (4851–4886)
    pub fn update_session_billing_route(
        &self,
        session_id: &str,
        provider: &str,
        base_url: &str,
        billing_mode: Option<&str>,
    ) -> Result<(), WriteError> {
        // Barrier against queued token deltas — see update_session_model.
        self.flush_token_counts(5.0);
        let sid = session_id.to_string();
        let provider = provider.to_string();
        let base_url = base_url.to_string();
        let billing_mode = billing_mode.map(str::to_string);
        let f = |conn: &Connection| -> Result<(), WriteError> {
            conn.execute(
                "UPDATE sessions SET \
                 billing_provider = ?, \
                 billing_base_url = ?, \
                 billing_mode = COALESCE(?, billing_mode), \
                 system_prompt = NULL, \
                 system_prompt_hash = NULL \
                 WHERE id = ?",
                rusqlite::params![provider, base_url, billing_mode, sid],
            )?;
            delete_unreferenced_system_prompts(conn)?;
            Ok(())
        };
        self.execute_write(&f, None)
    }
}
