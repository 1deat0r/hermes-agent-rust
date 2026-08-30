//! Local index of text we've sent via `sendRichMessage` (Bot API 10.1).
//!
//! PARITY: `gateway/rich_sent_store.py` @ b9aa928 (whole module).
//!
//! Telegram does NOT echo a rich message's content back in
//! `reply_to_message` when a user replies to it (verified: `.text`/`.caption`
//! empty, `.api_kwargs` None). So replies to the launchd briefings / any rich
//! send arrive with no quotable text and the agent is blind to what was
//! referenced.
//!
//! Fix: remember `message_id -> text` at send time, look it up by
//! `reply_to_id` on inbound. This module is the single source of truth for
//! that index.
//!
//! Best-effort and dependency-free: every operation swallows errors and
//! degrades to a no-op / `None` so it can never break a send or an inbound
//! message.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use serde_json::{json, Map, Value};

use hermes_constants::get_hermes_home;

const MAX_ENTRIES: usize = 1000;
const MAX_TEXT_CHARS: usize = 2000;

/// PARITY: `_store_path` (upstream lines 21-26). Resolved via
/// `get_hermes_home()` so the active profile override is honored.
pub fn store_path() -> PathBuf {
    let home = get_hermes_home();
    home.join("state").join("rich_sent_index.json")
}

/// PARITY: `_key` (upstream line 29) — Python f-string `str()` of both ids.
fn key(chat_id: i64, message_id: i64) -> String {
    format!("{chat_id}:{message_id}")
}

/// Persist `text` for `(chat_id, message_id)`. No-op on any failure.
///
/// PARITY: `record` (upstream lines 32-57). Guards mirror Python falsiness:
/// empty text, `None` message_id, or `None` chat_id return before any disk
/// touch. The store file is loaded leniently (missing/corrupt → empty dict,
/// non-dict → empty dict), the entry is written, the map is trimmed
/// oldest-by-timestamp (stable sort → insertion order breaks ties) past
/// [`MAX_ENTRIES`], and the file is replaced atomically via a
/// `<path>.tmp.<pid>` sibling.
pub fn record(chat_id: Option<i64>, message_id: Option<i64>, text: Option<&str>) {
    let text = match text {
        Some(text) if !text.is_empty() => text,
        _ => return,
    };
    let (chat_id, message_id) = match (chat_id, message_id) {
        (Some(chat_id), Some(message_id)) => (chat_id, message_id),
        _ => return,
    };
    let path = store_path();
    let result = (|| -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut data: Map<String, Value> = match fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<Value>(&raw) {
                // FileNotFoundError/ValueError → {}; non-dict (isinstance
                // check) → {}.
                Ok(Value::Object(map)) => map,
                _ => Map::new(),
            },
            Err(_) => Map::new(),
        };
        data.insert(
            key(chat_id, message_id),
            json!({
                "t": text.chars().take(MAX_TEXT_CHARS).collect::<String>(),
                "ts": hermes_time::now().timestamp(),
            }),
        );
        // Trim oldest by timestamp when over cap.
        if data.len() > MAX_ENTRIES {
            let mut by_ts: Vec<(String, i64)> = data
                .iter()
                .map(|(k, v)| (k.clone(), v.get("ts").and_then(Value::as_i64).unwrap_or(0)))
                .collect();
            // Python `sorted(..., key=...)` is stable: equal timestamps keep
            // insertion (JSON document) order.
            by_ts.sort_by_key(|(_, ts)| *ts);
            for (k, _) in by_ts.into_iter().take(data.len() - MAX_ENTRIES) {
                data.remove(&k);
            }
        }
        let tmp = path.with_file_name(format!(
            "{}.tmp.{}",
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            std::process::id()
        ));
        {
            let mut fh = fs::File::create(&tmp)?;
            // `json.dump(..., ensure_ascii=False)` — UTF-8 out.
            fh.write_all(
                serde_json::to_string(&Value::Object(data))
                    .unwrap_or_default()
                    .as_bytes(),
            )?;
        }
        fs::rename(&tmp, &path)?; // atomic; tolerates concurrent writers racing
        Ok(())
    })();
    // `except Exception: return` — a failed record is a silent no-op.
    let _ = result;
}

/// Return stored text for `(chat_id, message_id)` or `None`.
///
/// PARITY: `lookup` (upstream lines 60-72). `FileNotFoundError` /
/// `ValueError` (unreadable or corrupt JSON) and the `AttributeError` from a
/// non-dict document all return `None`; a falsy stored `t` (empty string or
/// JSON null) also yields `None` (Python `entry.get("t") or None`).
pub fn lookup(chat_id: Option<i64>, message_id: Option<i64>) -> Option<String> {
    let (chat_id, message_id) = match (chat_id, message_id) {
        (Some(chat_id), Some(message_id)) => (chat_id, message_id),
        _ => return None,
    };
    let raw = fs::read_to_string(store_path()).ok()?;
    let data: Value = serde_json::from_str(&raw).ok()?;
    let entry = data.get(key(chat_id, message_id))?;
    if let Value::Object(entry) = entry {
        let text = entry.get("t")?;
        return match text {
            Value::String(text) if !text.is_empty() => Some(text.clone()),
            _ => None,
        };
    }
    None
}
