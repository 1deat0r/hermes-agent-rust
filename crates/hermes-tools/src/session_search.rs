//! Session search tool — long-term conversation recall with discovery /
//! scroll / read / browse shapes.
//!
//! PARITY: tools/session_search_tool.py @ b9aa928 (1,161 LOC, ported 1:1
//! for the state-crate-facing surface).
//!
//! DEFERRED SEAMS (documented): cross-profile DB reading
//! (`_resolve_profile_db` / `_locate_session_db`) needs the hermes_cli
//! profiles crate (P3); until then `profile` is rejected with the upstream
//! error shape and the bare-id `@session:id` link form is emitted.


use chrono::{DateTime, Local};
use serde_json::{json, Value};

use hermes_state::state::SessionDB;
use rusqlite::OptionalExtension;
use crate::registry::{registry, tool_error, CheckFn, ToolHandler, ToolResult};
use crate::ansi_strip::strip_ansi;

const HIDDEN_SESSION_SOURCES: [&str; 3] = ["kanban", "subagent", "tool"];
const DEMOTED_SESSION_SOURCES: [&str; 1] = ["cron"];
const DISCOVER_SCAN_LIMIT: i64 = 300;
const DISCOVER_SEARCH_FIELDS: [&str; 7] = [
    "id", "session_id", "role", "snippet", "source", "model", "session_started",
];
const COMPACTION_PREFIXES: [&str; 2] = ["[CONTEXT COMPACTION", "[CONTEXT SUMMARY]:"];
const LINK_MAX_CONTENT_LEN: usize = 1200;
const WINDOW_MAX_CONTENT_LEN: usize = 4000;

fn format_timestamp(ts: Option<&Value>) -> String {
    let Some(v) = ts else { return "unknown".to_string() };
    let f = |secs: f64| -> String {
        let secs = secs.floor() as i64;
        let dt: DateTime<Local> = DateTime::from_timestamp(secs, 0)
            .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap())
            .with_timezone(&Local);
        dt.format("%B %d, %Y at %I:%M %p").to_string()
    };
    match v {
        Value::Null => "unknown".to_string(),
        Value::Number(n) => n.as_f64().map(f).unwrap_or_else(|| v.to_string()),
        Value::String(s) => {
            let numeric = s.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-');
            if numeric && !s.is_empty() {
                s.parse::<f64>().ok().map(f).unwrap_or_else(|| s.clone())
            } else {
                s.clone()
            }
        }
        _ => v.to_string(),
    }
}

fn is_compaction_summary(content: &str) -> bool {
    let stripped = content.trim_start();
    COMPACTION_PREFIXES.iter().any(|p| stripped.starts_with(p))
}

fn resolve_to_parent(db: &SessionDB, session_id: &str) -> (String, bool) {
    let mut cur = session_id.to_string();
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut has_compression = false;
    while !cur.is_empty() && !visited.contains(&cur) {
        visited.insert(cur.clone());
        let Ok(Some(s)) = db.get_session(&cur) else { break };
        if s.end_reason.as_deref() == Some("compression") {
            has_compression = true;
        }
        let Some(parent) = s.parent_session_id else { break };
        cur = parent;
    }
    (cur, has_compression)
}

fn resolve_lineage(db: &SessionDB, session_id: &str) -> String {
    resolve_to_parent(db, session_id).0
}

fn is_compression_ended(db: &SessionDB, session_id: &str) -> bool {
    if session_id.is_empty() {
        return false;
    }
    db.get_session(session_id)
        .ok()
        .flatten()
        .map(|s| s.end_reason.as_deref() == Some("compression"))
        .unwrap_or(false)
}

fn get_message_storage_state(db: &SessionDB, message_id: i64) -> Option<(String, i64, i64)> {
    let conn = db.writer_conn();
    conn.query_row(
        "SELECT session_id, active, compacted FROM messages WHERE id = ?",
        rusqlite::params![message_id],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)),
    )
    .optional()
    .ok()
    .flatten()
}

fn is_compacted_message(db: &SessionDB, message_id: i64) -> bool {
    get_message_storage_state(db, message_id)
        .map(|(_, active, compacted)| active == 0 && compacted == 1)
        .unwrap_or(false)
}

fn annotate_rebuild_status(db: &SessionDB, payload: &mut serde_json::Map<String, Value>) {
    let Some(status) = db.fts_rebuild_status() else { return };
    if let Some(percent) = status.get("percent").and_then(Value::as_f64) {
        payload.insert(
            "index_rebuild".to_string(),
            json!({
                "percent": percent,
                "note": format!(
                    "The search index is rebuilding in the background ({percent}% done). Results from older messages may be incomplete until it finishes."
                ),
            }),
        );
    }
}

fn order_for_recall(raw_results: Vec<Value>) -> Vec<Value> {
    let mut results = raw_results;
    results.sort_by_key(|r| {
        let source = r.get("source").and_then(Value::as_str).unwrap_or("");
        if DEMOTED_SESSION_SOURCES.contains(&source) {
            1
        } else {
            0
        }
    });
    results
}

fn shape_message(m: &Value, anchor_id: Option<i64>, max_content_len: Option<usize>) -> Value {
    let mut raw = m.get("content").cloned().unwrap_or(Value::Null);
    if let Value::String(s) = &raw {
        if s.contains('\x1b') {
            raw = Value::String(strip_ansi(s));
        }
    }
    let (content, truncated, original_chars) = match (max_content_len, &raw) {
        (Some(max), Value::String(s)) if s.chars().count() > max => {
            let cut: String = s.chars().take(max).collect();
            (Value::String(format!("{cut}…")), true, Some(s.chars().count()))
        }
        _ => (raw, false, None),
    };
    let mut entry = serde_json::Map::new();
    entry.insert("id".to_string(), m.get("id").cloned().unwrap_or(Value::Null));
    entry.insert("role".to_string(), m.get("role").cloned().unwrap_or(Value::Null));
    entry.insert("content".to_string(), Value::Null);
    entry.insert("timestamp".to_string(), Value::Null);
    if let Some(tn) = m.get("tool_name") {
        if !tn.is_null() {
            entry.insert("tool_name".to_string(), tn.clone());
        }
    }
    if let Some(tc) = m.get("tool_calls") {
        if !tc.is_null() {
            entry.insert("tool_calls".to_string(), tc.clone());
        }
    }
    if let Some(id) = anchor_id {
        if m.get("id").and_then(Value::as_i64) == Some(id) {
            entry.insert("anchor".to_string(), json!(true));
        }
    }
    entry.insert("content".to_string(), content);
    if truncated {
        entry.insert("content_truncated".to_string(), json!(true));
        entry.insert("original_content_chars".to_string(), json!(original_chars));
    }
    if let Some(ts) = m.get("timestamp").and_then(Value::as_f64) {
        entry.insert("timestamp".to_string(), json!(ts));
    }
    Value::Object(entry)
}

fn session_link(session_id: &str, _profile: Option<&str>) -> String {
    // Profile segment deferred until the hermes_cli profiles crate lands.
    format!("@session:{session_id}")
}

/// READ shape: dump a whole session by id (head + tail when large).
fn read_session(db: &SessionDB, session_id: &str, link_profile: Option<&str>) -> String {
    let meta = match db.get_session(session_id) {
        Ok(Some(m)) => m,
        _ => {
            return tool_error(format!("session_id not found: {session_id}"), &[("success".to_string(), json!(false))]);
        }
    };
    let rows = match db.get_messages(session_id, false, None, 0) {
        Ok(rows) => rows,
        Err(e) => {
            return tool_error(format!("failed to load session: {e}"), &[("success".to_string(), json!(false))]);
        }
    };
    let shaped: Vec<Value> = rows.iter().map(|m| shape_message(&message_to_value(m), None, None)).collect();
    let total = shaped.len();
    let head = 20usize;
    let tail = 10usize;
    let truncated = total > head + tail;
    let window = if truncated {
        let mut w: Vec<Value> = Vec::new();
        w.extend(shaped[..head].iter().cloned());
        w.extend(shaped[total - tail..].iter().cloned());
        w
    } else {
        shaped
    };
    let mut response = serde_json::Map::new();
    response.insert("success".to_string(), json!(true));
    response.insert("mode".to_string(), json!("read"));
    response.insert("session_id".to_string(), json!(session_id));
    response.insert("link".to_string(), json!(session_link(session_id, link_profile)));
    response.insert(
        "session_meta".to_string(),
        json!({
            "when": format_timestamp(Some(&json!(meta.started_at))),
            "source": meta.source,
            "model": meta.model,
            "title": meta.title,
        }),
    );
    response.insert("message_count".to_string(), json!(total));
    response.insert("truncated".to_string(), json!(truncated));
    response.insert("messages".to_string(), Value::Array(window));
    if truncated {
        response.insert(
            "message".to_string(),
            json!(format!(
                "Session has {total} messages; showing first {head} + last {tail}. Pass around_message_id (any id above) to scroll the middle."
            )),
        );
    }
    serde_json::to_string(&Value::Object(response)).expect("json")
}

fn message_to_value(m: &hermes_state::crud::StoredMessage) -> Value {
    json!({
        "id": m.id,
        "role": m.role,
        "content": m.content,
        "tool_name": m.tool_name,
        "tool_calls": m.tool_calls,
        "tool_call_id": m.tool_call_id,
        "timestamp": m.timestamp,
    })
}

/// BROWSE shape: metadata for recent sessions.
fn list_recent_sessions(
    db: &SessionDB,
    limit: i64,
    current_session_id: Option<&str>,
    link_profile: Option<&str>,
) -> String {
    let sessions = db
        .list_sessions_rich(&hermes_state::rich::RichListParams {
            limit: limit + 5,
            exclude_sources: HIDDEN_SESSION_SOURCES.iter().map(|s| s.to_string()).collect(),
            order_by_last_active: true,
            ..Default::default()
        })
        .unwrap_or_default();
    let current_root = current_session_id.map(|sid| resolve_lineage(db, sid));
    let mut results: Vec<Value> = Vec::new();
    for s in &sessions {
        let sid = s.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        if let Some(root) = &current_root {
            if &sid == root || Some(sid.as_str()) == current_session_id {
                continue;
            }
        }
        if s.get("parent_session_id").map(|v| !v.is_null()).unwrap_or(false) {
            continue;
        }
        results.push(json!({
            "session_id": sid,
            "link": session_link(&sid, link_profile),
            "title": s.get("title").and_then(Value::as_str).map(|t| t.to_string()).or(Some(String::new())),
            "source": s.get("source").and_then(Value::as_str).unwrap_or(""),
            "started_at": s.get("started_at"),
            "last_active": s.get("last_active"),
            "message_count": s.get("message_count").and_then(Value::as_i64).unwrap_or(0),
            "preview": s.get("preview").and_then(Value::as_str).unwrap_or(""),
        }));
        if results.len() as i64 >= limit {
            break;
        }
    }
    serde_json::to_string(&json!({
        "success": true,
        "mode": "browse",
        "results": results,
        "count": results.len(),
        "message": format!("Showing {} most recent sessions. Pass a query= to search, or session_id+around_message_id to scroll.", results.len()),
    }))
    .expect("json")
}

/// SCROLL shape: window centered on an anchor.
fn scroll(
    db: &SessionDB,
    session_id: &str,
    around_message_id: i64,
    window: i64,
    current_session_id: Option<&str>,
) -> String {
    let session_id = session_id.trim().to_string();
    if session_id.is_empty() {
        return tool_error("scroll requires session_id", &[("success".to_string(), json!(false))]);
    }
    let window = window.clamp(1, 20);

    let anchor_state = get_message_storage_state(db, around_message_id);
    let owning_session_id = anchor_state.as_ref().map(|(sid, _, _)| sid.clone());

    if let Some(current) = current_session_id {
        let anchor_session = owning_session_id.as_deref().unwrap_or(&session_id);
        let a_root = resolve_lineage(db, anchor_session);
        let c_root = resolve_lineage(db, current);
        if !a_root.is_empty() && !c_root.is_empty() && a_root == c_root {
            let anchor_active = anchor_state.as_ref().map(|(_, a, _)| *a).unwrap_or(0);
            let anchor_compacted = anchor_state.as_ref().map(|(_, _, c)| *c).unwrap_or(0);
            let is_compacted_anchor = anchor_active == 0 && anchor_compacted == 1;
            let is_inactive_non_compacted = anchor_active == 0 && anchor_compacted != 1;
            let is_compression_history =
                !is_inactive_non_compacted && is_compression_ended(db, anchor_session);
            if !(is_compacted_anchor || is_compression_history) {
                return tool_error(
                    "scroll rejected: anchor lives in the current session lineage (already in your active context)",
                    &[("success".to_string(), json!(false))],
                );
            }
        }
    }

    let session_meta = db.get_session(&session_id).ok().flatten();
    if session_meta.is_none() {
        return tool_error(format!("session_id not found: {session_id}"), &[("success".to_string(), json!(false))]);
    }

    let mut view = db.get_messages_around(&session_id, around_message_id, window).unwrap_or_default();
    let mut messages = view.get("window").and_then(Value::as_array).cloned().unwrap_or_default();

    let mut session_id = session_id;
    let mut session_meta = session_meta;
    let mut rebind_warning: Option<String> = None;
    if messages.is_empty() {
        if let Some(owning) = &owning_session_id {
            if owning != &session_id {
                let a_root = resolve_lineage(db, &session_id);
                let o_root = resolve_lineage(db, owning);
                if !a_root.is_empty() && !o_root.is_empty() && a_root == o_root {
                    if let Ok(rebind_view) = db.get_messages_around(owning, around_message_id, window) {
                        let rebound = rebind_view.get("window").and_then(Value::as_array).cloned().unwrap_or_default();
                        if !rebound.is_empty() {
                            view = rebind_view;
                            messages = rebound;
                            rebind_warning = Some(format!(
                                "around_message_id {around_message_id} lives in {owning} (child of {session_id}); rebound transparently"
                            ));
                            if let Ok(Some(m2)) = db.get_session(owning) {
                                session_meta = Some(m2);
                            }
                            session_id = owning.clone();
                        }
                    }
                }
            }
        }
    }

    if messages.is_empty() {
        return tool_error(
            format!("around_message_id {around_message_id} not in session_id {session_id}"),
            &[("success".to_string(), json!(false))],
        );
    }

    let meta = session_meta.expect("meta");
    let mut response = serde_json::Map::new();
    response.insert("success".to_string(), json!(true));
    response.insert("mode".to_string(), json!("scroll"));
    response.insert("session_id".to_string(), json!(session_id));
    response.insert("around_message_id".to_string(), json!(around_message_id));
    response.insert(
        "session_meta".to_string(),
        json!({
            "when": format_timestamp(Some(&json!(meta.started_at))),
            "source": meta.source,
            "model": meta.model,
            "title": meta.title,
        }),
    );
    response.insert("window".to_string(), json!(window));
    response.insert(
        "messages".to_string(),
        Value::Array(messages.iter().map(|m| shape_message(m, Some(around_message_id), None)).collect()),
    );
    response.insert(
        "messages_before".to_string(),
        json!(view.get("messages_before").and_then(Value::as_i64).unwrap_or(0)),
    );
    response.insert(
        "messages_after".to_string(),
        json!(view.get("messages_after").and_then(Value::as_i64).unwrap_or(0)),
    );
    if let Some(w) = rebind_warning {
        response.insert("warning".to_string(), json!(w));
    }
    serde_json::to_string(&Value::Object(response)).expect("json")
}

fn normalize_title_query(query: &str) -> String {
    query.trim().trim_matches('`').trim_matches('\'').trim_matches('"').to_string()
}

fn message_shape_from_rows(rows: &[Value]) -> Vec<Value> {
    rows.iter().map(|m| shape_message(m, None, None)).collect()
}

/// DISCOVERY shape: FTS5 + anchored window + bookends per hit.
fn discover(
    db: &SessionDB,
    query: &str,
    role_filter: Option<&[String]>,
    limit: i64,
    sort: Option<&str>,
    current_session_id: Option<&str>,
    link_profile: Option<&str>,
) -> String {
    let role_list: Vec<String> = match role_filter {
        Some(list) if !list.is_empty() => list.to_vec(),
        _ => vec!["user".to_string(), "assistant".to_string()],
    };
    let current_lineage_root = current_session_id.map(|sid| resolve_lineage(db, sid));
    let title_result = title_match_result(db, query, current_lineage_root.as_deref());

    let raw_results = db
        .search_messages(
            query,
            None,
            Some(&HIDDEN_SESSION_SOURCES.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
            Some(&role_list),
            DISCOVER_SCAN_LIMIT,
            0,
            sort,
            false,
            Some(&DISCOVER_SEARCH_FIELDS.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
        )
        .unwrap_or_default();

    let raw_results = order_for_recall(raw_results);

    if raw_results.is_empty() && title_result.is_none() {
        let mut payload = serde_json::Map::new();
        payload.insert("success".to_string(), json!(true));
        payload.insert("mode".to_string(), json!("discover"));
        payload.insert("query".to_string(), json!(query));
        payload.insert("results".to_string(), json!([]));
        payload.insert("count".to_string(), json!(0));
        payload.insert("message".to_string(), json!("No matching sessions found."));
        annotate_rebuild_status(db, &mut payload);
        return serde_json::to_string(&Value::Object(payload)).expect("json");
    }

    let mut seen_sessions: Vec<(String, Value, bool)> = Vec::new(); // (root, row, title_only)
    let mut results: Vec<Value> = Vec::new();

    if let Some(mut title_entry) = title_result {
        let title_lineage = title_entry.get("_lineage_root").and_then(Value::as_str).map(|s| s.to_string());
        if let Some(lg) = &title_lineage {
            seen_sessions.push((lg.clone(), Value::Null, true));
        }
        title_entry.as_object_mut().unwrap().remove("_lineage_root");
        results.push(title_entry);
    }

    for r in &raw_results {
        if seen_sessions.len() >= limit as usize {
            break;
        }
        let raw_sid = r.get("session_id").and_then(Value::as_str).unwrap_or("").to_string();
        let (resolved_sid, _) = resolve_to_parent(db, &raw_sid);
        let is_compacted = r.get("id").and_then(Value::as_i64).map(|id| is_compacted_message(db, id)).unwrap_or(false);
        let is_ended = is_compression_ended(db, &raw_sid);
        if let Some(root) = &current_lineage_root {
            if &resolved_sid == root && !(is_ended || is_compacted) {
                continue;
            }
        }
        if let Some(cur) = current_session_id {
            if raw_sid == cur && !is_compacted {
                continue;
            }
        }
        if !seen_sessions.iter().any(|(root, _, _)| root == &resolved_sid) {
            let mut row = r.clone();
            row.as_object_mut().unwrap().insert("_lineage_root".to_string(), json!(resolved_sid));
            seen_sessions.push((resolved_sid.clone(), row, false));
        }
        if seen_sessions.len() >= limit as usize {
            break;
        }
    }

    for (lineage_root, match_info, title_only) in &seen_sessions {
        if *title_only {
            continue;
        }
        let hit_sid = match_info.get("session_id").and_then(Value::as_str).unwrap_or(lineage_root).to_string();
        let msg_id = match_info.get("id").and_then(Value::as_i64);
        let Some(msg_id) = msg_id else { continue };
        let view = match db.get_anchored_view(&hit_sid, msg_id, 5, 3, None) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let session_meta = db.get_session(lineage_root).ok().flatten();

        let bookend_start: Vec<Value> = view
            .get("bookend_start")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|m| !is_compaction_summary(m.get("content").and_then(Value::as_str).unwrap_or("")))
            .map(|m| shape_message(&m, None, Some(LINK_MAX_CONTENT_LEN)))
            .collect();
        let window_messages: Vec<Value> = view
            .get("window")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|m| shape_message(m, Some(msg_id), Some(WINDOW_MAX_CONTENT_LEN)))
            .collect();
        let bookend_end: Vec<Value> = view
            .get("bookend_end")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|m| !is_compaction_summary(m.get("content").and_then(Value::as_str).unwrap_or("")))
            .map(|m| shape_message(&m, None, Some(LINK_MAX_CONTENT_LEN)))
            .collect();

        let meta = session_meta.or_else(|| db.get_session(&hit_sid).ok().flatten());
        let mut entry = serde_json::Map::new();
        entry.insert("session_id".to_string(), json!(hit_sid));
        let started_ts: Option<Value> = meta.as_ref().map(|m| json!(m.started_at));
        entry.insert(
            "when".to_string(),
            json!(format_timestamp(started_ts.as_ref().or_else(|| match_info.get("session_started")))),
        );
        entry.insert(
            "source".to_string(),
            json!(meta.as_ref().map(|m| m.source.clone()).or_else(|| match_info.get("source").and_then(Value::as_str).map(|s| s.to_string())).unwrap_or_else(|| "unknown".to_string())),
        );
        entry.insert(
            "model".to_string(),
            json!(meta.as_ref().and_then(|m| m.model.clone()).or_else(|| match_info.get("model").and_then(Value::as_str).map(|s| s.to_string())).unwrap_or_else(|| "unknown".to_string())),
        );
        entry.insert("title".to_string(), meta.as_ref().and_then(|m| m.title.clone()).map(Value::String).unwrap_or(Value::Null));
        entry.insert("matched_role".to_string(), match_info.get("role").cloned().unwrap_or(Value::Null));
        entry.insert("match_message_id".to_string(), json!(msg_id));
        entry.insert("snippet".to_string(), match_info.get("snippet").cloned().unwrap_or(Value::String(String::new())));
        entry.insert("bookend_start".to_string(), Value::Array(bookend_start));
        entry.insert("messages".to_string(), Value::Array(window_messages));
        entry.insert("bookend_end".to_string(), Value::Array(bookend_end));
        entry.insert(
            "messages_before".to_string(),
            json!(view.get("messages_before").and_then(Value::as_i64).unwrap_or(0)),
        );
        entry.insert(
            "messages_after".to_string(),
            json!(view.get("messages_after").and_then(Value::as_i64).unwrap_or(0)),
        );
        if lineage_root != &hit_sid {
            entry.insert("parent_session_id".to_string(), json!(lineage_root));
        }
        let completion = serde_json::to_value(&entry).expect("json");
        results.push(adjust_entry_link(completion, link_profile));
    }

    let results_len = results.len();
    let sessions_searched = seen_sessions.len();
    let mut payload = serde_json::Map::new();
    payload.insert("success".to_string(), json!(true));
    payload.insert("mode".to_string(), json!("discover"));
    payload.insert("query".to_string(), json!(query));
    payload.insert("results".to_string(), Value::Array(results));
    payload.insert("count".to_string(), json!(results_len));
    payload.insert("sessions_searched".to_string(), json!(sessions_searched));
    annotate_rebuild_status(db, &mut payload);
    serde_json::to_string(&Value::Object(payload)).expect("json")
}

fn adjust_entry_link(mut entry: Value, link_profile: Option<&str>) -> Value {
    let sid = entry
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if let Some(obj) = entry.as_object_mut() {
        obj.insert("link".to_string(), json!(session_link(&sid, link_profile)));
    }
    entry
}

fn title_match_result(
    db: &SessionDB,
    query: &str,
    current_lineage_root: Option<&str>,
) -> Option<Value> {
    let title_query = normalize_title_query(query);
    if title_query.is_empty() {
        return None;
    }
    let session_id = db.resolve_session_by_title(&title_query).ok().flatten()?;
    let lineage_root = resolve_lineage(db, &session_id);
    if let Some(cur) = current_lineage_root {
        if !cur.is_empty() && lineage_root == cur {
            return None;
        }
    }
    let session_meta = db
        .get_session(&lineage_root)
        .ok()
        .flatten()
        .or_else(|| db.get_session(&session_id).ok().flatten())?;
    let source = session_meta.source.as_str();
    if HIDDEN_SESSION_SOURCES.contains(&source) {
        return None;
    }
    let messages = db.get_messages(&session_id, false, None, 0).unwrap_or_default();
    let anchor_id = messages.first().map(|m| m.id);
    let view = anchor_id
        .and_then(|aid| db.get_anchored_view(&session_id, aid, 5, 3, None).ok())
        .unwrap_or_default();
    let window_rows: Vec<Value> = messages.iter().map(message_to_value).collect();
    let mut entry = serde_json::Map::new();
    entry.insert("session_id".to_string(), json!(session_id));
    entry.insert(
        "when".to_string(),
        json!(format_timestamp(Some(&json!(session_meta.started_at)))),
    );
    entry.insert("source".to_string(), json!(session_meta.source));
    entry.insert("model".to_string(), json!(session_meta.model.clone().unwrap_or_else(|| "unknown".to_string())));
    entry.insert("title".to_string(), json!(session_meta.title.clone().unwrap_or_else(|| title_query.clone())));
    entry.insert("matched_role".to_string(), json!("session_title"));
    entry.insert("match_message_id".to_string(), anchor_id.map(Value::from).unwrap_or(Value::Null));
    entry.insert("snippet".to_string(), json!(format!("Session title matched: {}", session_meta.title.clone().unwrap_or_else(|| title_query.clone()))));
    entry.insert(
        "bookend_start".to_string(),
        Value::Array(
            view.get("bookend_start").and_then(Value::as_array).cloned()
                .map(|rows| message_shape_from_rows(rows.as_slice()))
                .unwrap_or_else(|| window_rows[..3.min(window_rows.len())].iter().map(|m| shape_message(m, None, None)).collect()),
        ),
    );
    entry.insert(
        "messages".to_string(),
        Value::Array(
            view.get("window").and_then(Value::as_array).cloned()
                .map(|rows| rows.iter().map(|m| shape_message(m, anchor_id, None)).collect())
                .unwrap_or_else(|| window_rows[..5.min(window_rows.len())].iter().map(|m| shape_message(m, anchor_id, None)).collect()),
        ),
    );
    entry.insert(
        "bookend_end".to_string(),
        Value::Array(
            view.get("bookend_end").and_then(Value::as_array).cloned()
                .map(|rows| message_shape_from_rows(rows.as_slice()))
                .unwrap_or_else(|| {
                    if window_rows.is_empty() { vec![] } else {
                        window_rows[window_rows.len() - 3..].iter().map(|m| shape_message(m, None, None)).collect()
                    }
                }),
        ),
    );
    entry.insert("messages_before".to_string(), json!(view.get("messages_before").and_then(Value::as_i64).unwrap_or(0)));
    entry.insert(
        "messages_after".to_string(),
        json!(view.get("messages_after").and_then(Value::as_i64).unwrap_or_else(|| (window_rows.len().saturating_sub(5)) as i64)),
    );
    entry.insert("_lineage_root".to_string(), json!(lineage_root));
    if lineage_root != session_id {
        entry.insert("parent_session_id".to_string(), json!(lineage_root));
    }
    Some(Value::Object(entry))
}

/// Single-shape tool: mode inferred from which args are set.
// `clippy::too_many_arguments` allowed: the parameter set mirrors the
// upstream keyword surface 1:1 (matching the toolsets precedent).
#[allow(clippy::too_many_arguments)]
pub fn session_search(
    db: Option<&SessionDB>,
    query: &str,
    role_filter: Option<&str>,
    limit: i64,
    session_id: Option<&str>,
    around_message_id: Option<i64>,
    window: i64,
    sort: Option<&str>,
    profile: Option<&str>,
    current_session_id: Option<&str>,
) -> String {
    // The db is always provided by the handler (the upstream lazily opens
    // SessionDB() when None — deferred to the caller seam).
    let Some(db) = db else {
        return tool_error("Session database not available", &[("success".to_string(), json!(false))]);
    };

    let _ = profile; // cross-profile DB seam deferred (hermes_cli profiles)

    // Normalise a raw @session:<profile>/<id> value passed as session_id.
    let mut session_id = session_id.map(|s| s.to_string());
    if let Some(sid) = &session_id {
        if let Some((_emb, id)) = sid.split_once('/') {
            if !id.is_empty() {
                session_id = Some(id.to_string());
            }
        }
    }

    // Scroll shape takes precedence.
    if let Some(sid) = session_id.clone() {
        if !sid.trim().is_empty() {
            if let Some(anchor) = around_message_id {
                return scroll(db, &sid, anchor, window, current_session_id);
            }
            // Read shape: session_id with no anchor.
            if let Some(sid) = session_id {
                return read_session(db, &sid, None);
            }
        }
    }

    // Limit clamp [1, 10].
    let limit = limit.clamp(1, 10);

    // Browse shape: no query -> recent sessions.
    if query.trim().is_empty() {
        return list_recent_sessions(db, limit, current_session_id, None);
    }

    // Parse role_filter.
    let role_list: Option<Vec<String>> = role_filter.map(|rf| {
        let rf = rf.trim().to_string();
        if rf.is_empty() {
            Vec::new()
        } else {
            rf.split(',').map(|r| r.trim().to_string()).filter(|r| !r.is_empty()).collect()
        }
    });

    // Normalise sort.
    let sort_norm: Option<&str> = sort.and_then(|s| {
        let c = s.trim().to_lowercase();
        if c == "newest" || c == "oldest" {
            Some(if c == "newest" { "newest" } else { "oldest" })
        } else {
            None
        }
    });

    discover(db, query.trim(), role_list.as_deref(), limit, sort_norm, current_session_id, None)
}

pub struct SessionSearchCheck;
impl CheckFn for SessionSearchCheck {
    fn check(&self) -> bool {
        // Requires the SQLite state database home to exist.
        crate::session_search_check_expr()
    }
}

struct SessionSearchHandler;
impl ToolHandler for SessionSearchHandler {
    fn call(&self, args: Value, _: Option<&str>, _: Option<&str>) -> ToolResult {
        let query = args.get("query").and_then(Value::as_str).unwrap_or("");
        let role_filter = args.get("role_filter").and_then(Value::as_str);
        let limit = args.get("limit").and_then(Value::as_i64).unwrap_or(3);
        let session_id = args.get("session_id").and_then(Value::as_str);
        let around_message_id = args.get("around_message_id").and_then(Value::as_i64);
        let window = args.get("window").and_then(Value::as_i64).unwrap_or(5);
        let sort = args.get("sort").and_then(Value::as_str);
        let profile = args.get("profile").and_then(Value::as_str);
        ToolResult::Text(session_search(
            None, query, role_filter, limit, session_id, around_message_id, window, sort, profile, None,
        ))
    }
}

pub static SESSION_SEARCH_SCHEMA: once_cell::sync::Lazy<Value> = once_cell::sync::Lazy::new(|| {
    json!({
        "name": "session_search",
        "description": "Search past sessions stored in the local session DB, or scroll inside one. FTS5-backed retrieval over the SQLite message store. No LLM calls — every shape returns actual messages from the DB.\n\nFOUR CALLING SHAPES\n\n  1) DISCOVERY — pass `query`.\n  2) SCROLL — pass `session_id` + `around_message_id`.\n  3) READ — pass `session_id` only (no around_message_id).\n  4) BROWSE — no args.",
        "parameters": {
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search query (discovery shape)."},
                "limit": {"type": "integer", "description": "Discovery shape only. Max sessions to return (default 3, max 10).", "default": 3},
                "sort": {"type": "string", "enum": ["newest", "oldest"], "description": "Discovery shape only. Temporal bias on top of FTS5 ranking."},
                "session_id": {"type": "string", "description": "Scroll/read shape. Session to read inside."},
                "around_message_id": {"type": "integer", "description": "Scroll shape. Message id to center the window on."},
                "window": {"type": "integer", "description": "Scroll shape only. Messages on each side of the anchor. Clamped to [1, 20]. Default 5.", "default": 5},
                "role_filter": {"type": "string", "description": "Optional. Comma-separated roles to include."},
                "profile": {"type": "string", "description": "Optional. Read sessions from another profile (read-only). Deferred in this port."}
            },
            "required": []
        }
    })
});

/// Register the session_search tool.
pub fn register_session_search() {
    registry()
        .register(
            "session_search",
            "session_search",
            SESSION_SEARCH_SCHEMA.clone(),
            std::sync::Arc::new(SessionSearchHandler),
            Some(std::sync::Arc::new(SessionSearchCheck)),
            Some("check_session_search_requirements"),
            vec![],
            None,
            Some("🔍".to_string()),
            None,
            None,
            None,
            false,
        )
        .expect("register session_search");
}
