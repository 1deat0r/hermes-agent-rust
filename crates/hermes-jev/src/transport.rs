//! Shared System One HTTP transport.
//!
//! Ports the transport shared by live `agent/jev_choice.py`,
//! `agent/compression_scored_prune.py`, `agent/turn_context.py`, and
//! `agent/auxiliary_client.py`: `POST {SYSTEMONE_URL}` with
//! `{"model": "jev-latest", "state": ..., "questions": {...}}`,
//! `Authorization: Bearer <TYPESAFE_API_KEY>` read from the environment
//! at call time, retries on 429/529/503, raise on every other failure.
//! The key is never logged, printed, or stored.

use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::env;
use std::time::Duration;
use thiserror::Error;

use crate::questions::Question;

/// Evaluation endpoint (TypeSafe docs `api.md`).
pub const SYSTEMONE_URL: &str = "https://api.typesafe.ai/v1/systemone";
/// Flagship System One model.
pub const JEV_MODEL: &str = "jev-latest";
/// Max options per Choice question (API rejects more).
pub const MAX_CHOICE_OPTIONS: usize = 255;
/// Max levels per Score question (API accepts up to 10).
pub const MAX_SCORE_LEVELS: usize = 10;
/// Env var carrying the key, read at call time.
pub const API_KEY_ENV: &str = "TYPESAFE_API_KEY";
/// Request timeout (live code uses 25s).
pub const REQUEST_TIMEOUT_SECS: u64 = 25;
/// Statuses worth one more attempt (live code retries 3 attempts).
pub const RETRY_STATUSES: [u16; 3] = [429, 529, 503];
/// Max POST attempts per request.
pub const MAX_ATTEMPTS: u32 = 3;

/// Transport failure. The key value is never included in any message.
#[derive(Debug, Error, PartialEq)]
pub enum JevError {
    /// No `TYPESAFE_API_KEY` in the environment; caller falls back.
    #[error("TYPESAFE_API_KEY is not configured; caller falls back")]
    MissingKey,
    /// HTTP client unavailable; caller falls back.
    #[error("HTTP client unavailable; caller falls back")]
    NoTransport,
    /// Connection or response-read failure; caller falls back.
    #[error("model connection failed; caller falls back")]
    Connection,
    /// Non-retryable HTTP status; caller falls back.
    #[error("jev returned HTTP {0}; caller falls back")]
    HttpStatus(u16),
    /// Response had no usable `answers` map; caller falls back.
    #[error("jev response is missing answers; caller falls back")]
    BadResponse,
    /// Question map has no usable ids (empty criteria / levels).
    #[error("question has no criteria ids; caller falls back")]
    EmptyQuestion,
}

/// Read the key at call time; `None` when unset or blank.
pub fn api_key() -> Option<String> {
    match env::var(API_KEY_ENV) {
        Ok(key) if !key.trim().is_empty() => Some(key),
        _ => None,
    }
}

/// One blocking System One request; returns the raw `answers` map.
///
/// Sleeps are bounded exponential backoff (0.5s, 1s) exactly like the
/// live code. Never logs headers or the key.
pub fn post_system_one(
    state: &Value,
    questions: &BTreeMap<String, Question>,
    model: &str,
) -> Result<Map<String, Value>, JevError> {
    let key = api_key().ok_or(JevError::MissingKey)?;
    let body_questions: Map<String, Value> = questions
        .iter()
        .map(|(id, q)| (id.clone(), q.to_json()))
        .collect();
    let body = serde_json::json!({
        "model": model,
        "state": state,
        "questions": Value::Object(body_questions),
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|_| JevError::NoTransport)?;
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        let response = client
            .post(SYSTEMONE_URL)
            .header("Authorization", format!("Bearer {key}"))
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .map_err(|_| JevError::Connection)?;
        let status = response.status().as_u16();
        if RETRY_STATUSES.contains(&status) && attempt < MAX_ATTEMPTS {
            std::thread::sleep(Duration::from_millis(500 * 2_u64.pow(attempt - 1)));
            continue;
        }
        if !(200..300).contains(&status) {
            return Err(JevError::HttpStatus(status));
        }
        let text = response.text().map_err(|_| JevError::BadResponse)?;
        let parsed: Value = serde_json::from_str(&text).map_err(|_| JevError::BadResponse)?;
        return parsed
            .get("answers")
            .and_then(Value::as_object)
            .cloned()
            .ok_or(JevError::BadResponse);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyless_post_returns_missing_key_without_network() {
        temp_env_remove(API_KEY_ENV, || {
            let questions = BTreeMap::new();
            assert_eq!(
                post_system_one(&Value::Null, &questions, JEV_MODEL),
                Err(JevError::MissingKey)
            );
        });
    }

    /// Run `f` with `var` removed, restoring any prior value after.
    fn temp_env_remove(var: &str, f: impl FnOnce()) {
        let prior = env::var(var).ok();
        unsafe {
            env::remove_var(var);
        }
        f();
        if let Some(value) = prior {
            unsafe {
                env::set_var(var, value);
            }
        }
    }
}
