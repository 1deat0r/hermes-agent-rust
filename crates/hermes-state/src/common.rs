//! hermes_state_common.py — shared SQL constants and query builders for the
//! SessionDB family, plus the skill-scaffolding recognition subset pulled in
//! from agent/skill_commands.py (preview shaping depends on it).
// PARITY: hermes_state_common.py @ b9aa928 (extract: tools/gen_state_common_constants.py)
// PARITY: agent/skill_commands.py subset (SKILL_SCAFFOLD_SQL_LIKE,
//         SKILL_EXCERPT_JOINT, describe_skill_invocation,
//         extract_user_instruction_from_skill_message + helpers) @ b9aa928 —
//         inlined here because agent/ is a Phase 2 crate; when the agent crate
//         lands, re-home these helpers there and depend on it (see PLAN §5).

#[path = "common_constants.rs"]
mod constants;
pub use constants::*;

// ── Preview shaping ─────────────────────────────────────────────────────────

pub const _PREVIEW_HEAD_CHARS: usize = 63;
pub const _PREVIEW_SCAFFOLD_WINDOW: usize = 400;
pub const _PREVIEW_MAX_CHARS: usize = 60;

/// Escape SQL LIKE wildcards so operator/session-derived text matches
/// literally. Pair with `ESCAPE '\'` in the clause.
// PARITY: hermes_state_common.py escape_like @ b9aa928
pub fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub const _PREVIEW_CONTENT_SQL: &str =
    "REPLACE(REPLACE(m.content, X'0A', ' '), X'0D', ' ')";

pub fn _preview_scaffolded_sql() -> String {
    format!("m.content LIKE '{}'", crate::skill::SKILL_SCAFFOLD_SQL_LIKE)
}

/// The shared `_preview_raw` SELECT expression, interpolated by every listing
/// query.
// PARITY: hermes_state_common.py _PREVIEW_RAW_SELECT @ b9aa928
pub fn _preview_raw_select() -> String {
    format!(
        concat!(
            "CASE WHEN {scaffolded}",
            " AND LENGTH(m.content) > {window2}",
            " THEN SUBSTR({sql}, 1, {window})",
            " || '{joint}'",
            " || SUBSTR({sql}, -{window})",
            " WHEN {scaffolded}",
            " THEN SUBSTR({sql}, 1, {window2})",
            " ELSE SUBSTR({sql}, 1, {head}) END",
        ),
        scaffolded = _preview_scaffolded_sql(),
        sql = _PREVIEW_CONTENT_SQL,
        joint = crate::skill::SKILL_EXCERPT_JOINT,
        window = _PREVIEW_SCAFFOLD_WINDOW,
        window2 = _PREVIEW_SCAFFOLD_WINDOW * 2,
        head = _PREVIEW_HEAD_CHARS,
    )
}

/// Turn a `_preview_raw` column into the short preview callers show.
// PARITY: hermes_state_common.py _shape_preview @ b9aa928
pub fn _shape_preview(raw: impl Into<String>) -> String {
    let text = raw.into();
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    let described = crate::skill::describe_skill_invocation(text, " — ");
    let base = match described {
        Some(d) => d,
        None => text
            .split(crate::skill::SKILL_EXCERPT_JOINT)
            .next()
            .unwrap_or("")
            .to_string(),
    };
    if base.len() > _PREVIEW_MAX_CHARS {
        let mut out: String = base.chars().take(_PREVIEW_MAX_CHARS).collect();
        out.push_str("...");
        out
    } else {
        base
    }
}

// ── Child-session classification SQL ────────────────────────────────────────

/// `_BRANCH_CHILD_SQL` — a child counts as a /branch (kept visible, never
/// cascade-deleted) if it carries the stable marker OR the legacy end_reason
/// heuristic holds.
// PARITY: hermes_state_common.py _BRANCH_CHILD_SQL @ b9aa928
pub fn _branch_child_sql(alias: &str) -> String {
    format!(
        concat!(
            "json_extract(COALESCE({a}.model_config, '{{}}'), '$._branched_from') IS NOT NULL",
            " OR EXISTS (SELECT 1 FROM sessions p",
            "            WHERE p.id = {a}.parent_session_id",
            "            AND p.end_reason = 'branched'",
            "            AND {a}.started_at >= p.ended_at)",
        ),
        a = alias
    )
}

/// `_COMPRESSION_CHILD_SQL`
// PARITY: hermes_state_common.py _COMPRESSION_CHILD_SQL @ b9aa928
pub fn _compression_child_sql(alias: &str) -> String {
    format!(
        concat!(
            "EXISTS (SELECT 1 FROM sessions p",
            "        WHERE p.id = {a}.parent_session_id",
            "        AND p.end_reason = 'compression')",
        ),
        a = alias
    )
}

/// `_LISTABLE_CHILD_SQL` — rows that surface in pickers: roots + branch
/// children (subagent runs and compression continuations stay hidden).
// PARITY: hermes_state_common.py _LISTABLE_CHILD_SQL @ b9aa928
pub fn _listable_child_sql() -> String {
    format!("(s.parent_session_id IS NULL OR {})", _branch_child_sql("s"))
}

/// Subagent runs (cascade-delete targets), not branches or compression tips.
// PARITY: hermes_state_common.py _ephemeral_child_sql @ b9aa928
pub fn _ephemeral_child_sql(alias: &str) -> String {
    let branch = _branch_child_sql(alias);
    let compression = _compression_child_sql(alias);
    format!(
        concat!(
            "({alias}.parent_session_id IS NOT NULL",
            " AND NOT ({branch})",
            " AND NOT ({compression}))",
        ),
        alias = alias,
        branch = branch,
        compression = compression,
    )
}

/// SQL expression for session recency used by list/status surfaces.
// PARITY: hermes_state_common.py _sql_session_last_active @ b9aa928
pub fn _sql_session_last_active(alias: &str) -> String {
    let msg_max = format!(
        concat!(
            "(SELECT MAX(_act_m.timestamp) FROM messages _act_m ",
            "WHERE _act_m.session_id = {alias}.id)",
        ),
        alias = alias
    );
    format!(
        concat!(
            "COALESCE(",
            "(SELECT MAX(_act_v.v) FROM (",
            "SELECT {alias}.last_activity_at AS v ",
            "UNION ALL ",
            "SELECT {msg_max}",
            ") _act_v), ",
            "{alias}.started_at)",
        ),
        alias = alias,
        msg_max = msg_max
    )
}

/// Same freshest-of expression keyed by a session-id SQL expression.
// PARITY: hermes_state_common.py _sql_session_last_active_by_id @ b9aa928
pub fn _sql_session_last_active_by_id(session_id_expr: &str) -> String {
    let msg_max = format!(
        concat!(
            "(SELECT MAX(_act_m.timestamp) FROM messages _act_m ",
            "WHERE _act_m.session_id = {sid})",
        ),
        sid = session_id_expr
    );
    let activity = format!(
        concat!(
            "(SELECT last_activity_at FROM sessions _act_s ",
            "WHERE _act_s.id = {sid})",
        ),
        sid = session_id_expr
    );
    let started = format!(
        concat!(
            "(SELECT started_at FROM sessions _act_s ",
            "WHERE _act_s.id = {sid})",
        ),
        sid = session_id_expr
    );
    format!(
        concat!(
            "COALESCE(",
            "(SELECT MAX(_act_v.v) FROM (",
            "SELECT {activity} AS v ",
            "UNION ALL ",
            "SELECT {msg_max}",
            ") _act_v), ",
            "{started})",
        ),
        activity = activity,
        msg_max = msg_max,
        started = started
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_constants_are_byte_identical_to_upstream_common() {
        assert_eq!(SCHEMA_VERSION, 25);
        assert_eq!(FTS_STORAGE_VERSION, 1);
        assert_eq!(MAX_FTS5_QUERY_CHARS, 2048);
        assert_eq!(FTS_CJK_STALE_KEY, "fts_cjk_stale");
        assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS schema_version"));
        assert!(SCHEMA_SQL.contains("compression_ineffective_count INTEGER NOT NULL DEFAULT 0"));
        assert!(SCHEMA_SQL.contains("idx_messages_assistant_calls_by_session"));
        assert!(DEFERRED_INDEX_SQL.contains("idx_messages_session_active"));
        assert!(FTS_SQL.contains("CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5("));
        assert!(FTS_SQL.contains("content='messages'"));
        assert!(FTS_TRIGRAM_SQL.contains("tokenize='trigram'"));
        assert!(LEGACY_FTS_SQL.contains("DELETE FROM messages_fts WHERE rowid = old.id;"));
        assert!(LEGACY_FTS_TRIGRAM_SQL.contains("tokenize='trigram'"));
        assert_eq!(_FTS_TRIGGERS.len(), 6);
        assert_eq!(_FTS_CJK_TRIGGERS.len(), 3);
    }

    #[test]
    fn escape_like_escapes_wildcards_and_backslash() {
        assert_eq!(escape_like("hello"), "hello");
        assert_eq!(escape_like("50%_off"), "50\\%\\_off");
        assert_eq!(escape_like(r"a\b"), r"a\\b");
        assert_eq!(escape_like(""), "");
    }

    #[test]
    fn child_sql_builders_round_trip() {
        assert!(crate::common::_branch_child_sql("s").contains("$._branched_from"));
        assert!(crate::common::_compression_child_sql("s").contains("end_reason = 'compression'"));
        let listable = crate::common::_listable_child_sql();
        assert!(listable.starts_with("(s.parent_session_id IS NULL OR "));
        let ephem = crate::common::_ephemeral_child_sql("s");
        assert!(ephem.starts_with("(s.parent_session_id IS NOT NULL"));
        assert!(ephem.contains("NOT (json_extract(COALESCE(s.model_config"));
    }

    #[test]
    fn last_active_builders_reference_expected_columns() {
        let la = crate::common::_sql_session_last_active("s");
        assert!(la.contains("last_activity_at"));
        assert!(la.contains("MAX(_act_m.timestamp)"));
        assert!(la.contains("started_at"));
        let by_id = crate::common::_sql_session_last_active_by_id("'abc'");
        assert!(by_id.contains("_act_s.id = 'abc'"));
        assert!(by_id.contains("MAX(_act_m.timestamp)"));
    }

    #[test]
    fn parity_with_upstream_golden_state_common() {
        // Oracle: outputs of the actual upstream functions (generated by
        // tools/golden_state_common.py from hermes_state_common.py @ b9aa928).
        let golden: serde_json::Value = serde_json::from_str(
            include_str!("../../../upstream/golden_state_common.json"),
        ).expect("golden fixture");
        let samples: Vec<String> = golden["samples"]
            .as_array().unwrap().iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        let shapes: Vec<&str> = golden["shape_preview"]
            .as_array().unwrap().iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        for (i, sample) in samples.iter().enumerate() {
            assert_eq!(_shape_preview(sample), shapes[i], "shape_preview case {i}");
        }

        let describes: Vec<Option<&str>> = golden["describe"]
            .as_array().unwrap().iter()
            .map(|v| v.as_str())
            .collect();
        for (i, sample) in samples.iter().enumerate() {
            assert_eq!(
                crate::skill::describe_skill_invocation(sample, " — ").as_deref(),
                describes[i],
                "describe case {i}",
            );
            assert_eq!(
                crate::skill::extract_user_instruction_from_skill_message(sample).as_deref(),
                golden["extract"].as_array().unwrap()[i].as_str(),
                "extract case {i}",
            );
        }

        let el: Vec<&str> = golden["escape_like"].as_array().unwrap().iter()
            .map(|v| v.as_str().unwrap()).collect();
        for (i, inp) in ["plain", "50%_", r"a\b_c.d%", "", "%%%", "_"].iter().enumerate() {
            assert_eq!(escape_like(inp), el[i], "escape_like case {i}");
        }

        // SQL builders byte-equality against upstream format() results.
        assert_eq!(_branch_child_sql("s"), golden["branch_s"].as_str().unwrap());
        assert_eq!(_compression_child_sql("s"), golden["compression_s"].as_str().unwrap());
        assert_eq!(_listable_child_sql(), golden["listable"].as_str().unwrap());
        assert_eq!(_ephemeral_child_sql("s"), golden["ephemeral_s"].as_str().unwrap());
        assert_eq!(_sql_session_last_active("s"), golden["last_active_s"].as_str().unwrap());
        assert_eq!(
            _sql_session_last_active_by_id("'abc'"),
            golden["last_active_by_id"].as_str().unwrap()
        );
        assert_eq!(_PREVIEW_CONTENT_SQL, golden["preview_content_sql"].as_str().unwrap());
        assert_eq!(
            _preview_scaffolded_sql(),
            golden["preview_scaffolded_sql"].as_str().unwrap()
        );
        assert_eq!(_preview_raw_select(), golden["preview_raw_select"].as_str().unwrap());
    }

}
