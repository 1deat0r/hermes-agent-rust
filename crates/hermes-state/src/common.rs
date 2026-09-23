//! hermes_state_common.py — shared constants, SQL builders, taxonomy, and
//! the cross-process FTS rebuild admission authority for the SessionDB
//! family, plus the skill-scaffolding recognition subset pulled in from
//! agent/skill_commands.py (preview shaping depends on it).
//!
//! PARITY: hermes_state_common.py @ 5d59366 (re-certified 2026-09-23;
//!         constants extracted verbatim via tools/gen_state_common_constants.py,
//!         byte oracle: tools/golden_state_common.py →
//!         upstream/golden_state_common.json).
// PARITY: agent/skill_commands.py subset (SKILL_SCAFFOLD_SQL_LIKE,
//         SKILL_EXCERPT_JOINT, describe_skill_invocation,
//         extract_user_instruction_from_skill_message + helpers) @ 5d59366 —
//         inlined here because agent/ is a Phase 2 crate; when the agent crate
//         lands, re-home these helpers there and depend on it (see PLAN §5).

#[path = "common_constants.rs"]
mod constants;
pub use constants::*;

// Cross-process FTS rebuild admission lives in its own file (flock +
// holder-record machinery); re-exported here under the upstream module's
// names so `hermes_state_common::fts_rebuild_admission` keeps its shape.
pub use crate::fts_lock::{
    fts_rebuild_admission, is_advisory_lock_contention, FtsRebuildAdmission,
};

use crate::compression_prefix::{
    LEGACY_SUMMARY_PREFIX, MERGED_PRIOR_CONTEXT_HEADER, MERGED_SUMMARY_DELIMITER,
    SUMMARY_END_MARKER, SUMMARY_PREFIX,
};

/// Accidental end reasons recovery treats as resumable — alias of the
/// generated tuple (public name used by portability/prune since the
/// adoption oracle landed).
// PARITY: `_RECOVERABLE_END_REASONS` (164) @ 5d59366.
pub const RECOVERABLE_END_REASONS: [&str; 4] = _RECOVERABLE_END_REASONS;

/// True when *reason* is an automatic-cleanup end stamp; compression-liveness
/// sites must call this instead of re-implementing the taxonomy (#88197).
///
/// PARITY: `is_automatic_end_reason` (179–185). Non-string / missing reasons
/// map to `None`/empty and return false.
pub fn is_automatic_end_reason(reason: Option<&str>) -> bool {
    matches!(reason, Some(r) if _AUTOMATIC_END_REASONS.contains(&r))
}

/// `sessions.<key>` for the profile whose state.db this process is touching.
///
/// Under a multiplexer a routed turn runs with a HERMES_HOME override while
/// the env bridge still holds the LAUNCH profile's value — an override read
/// therefore consults this profile's config.yaml directly. Unscoped: the env
/// bridge, as before. `None` when neither source sets the key.
///
/// PARITY: `routed_sessions_setting` (26–43). PORT SEAMS: upstream's lazy
/// `hermes_cli.config.load_config_readonly` import is the crate-local
/// `cfg::load_config_value` (same fail-open observable slice); exceptions
/// in the override branch map to `None` exactly like upstream's
/// `except Exception: return None`.
pub fn routed_sessions_setting(key: &str, env_var: &str) -> Option<String> {
    if hermes_constants::get_hermes_home_override().is_some() {
        let config_path = hermes_constants::get_config_path();
        let cfg = crate::cfg::load_config_value(&config_path)?;
        let sessions = cfg.get("sessions")?.as_mapping()?;
        sessions
            .get(&serde_yaml::Value::String(key.to_string()))?
            .as_str()
            .map(str::to_string)
    } else {
        std::env::var(env_var).ok()
    }
}

/// Escape SQL LIKE wildcards so operator/session-derived text matches
/// literally. Pair with `ESCAPE '\'` in the clause.
// PARITY: escape_like (46–49)
pub fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

// ── SQL fragment builders (57–104) ──────────────────────────────────────────

/// Double-quote an SQL string literal.
// PARITY: _sql_literal
pub fn _sql_literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// Non-throwing JSON marker lookup for a JSON TEXT column.
// PARITY: _sql_json_extract (61–68)
pub fn _sql_json_extract(expression: &str, path: &str) -> String {
    let safe_json = format!(
        "(CASE WHEN json_valid({expr}) THEN {expr} ELSE json_object() END)",
        expr = expression
    );
    format!("json_extract({}, {})", safe_json, _sql_literal(path))
}

const _SQL_WHITESPACE: &str = "CHAR(9) || CHAR(10) || CHAR(13) || CHAR(32)";

// PARITY: _sql_ltrim_whitespace / _sql_trim_whitespace (71–76)
pub fn _sql_ltrim_whitespace(expression: &str) -> String {
    format!("LTRIM({}, {})", expression, _SQL_WHITESPACE)
}

pub fn _sql_trim_whitespace(expression: &str) -> String {
    format!("TRIM({}, {})", expression, _SQL_WHITESPACE)
}

/// `expression` starts with any of *prefixes* (after whitespace trim).
// PARITY: _sql_starts_with (79–81)
pub fn _sql_starts_with(expression: &str, prefixes: &[&str]) -> String {
    let trimmed = _sql_ltrim_whitespace(expression);
    let ors: Vec<String> = prefixes
        .iter()
        .map(|p| {
            format!(
                "SUBSTR({}, 1, {}) = {}",
                trimmed,
                p.chars().count(),
                _sql_literal(p)
            )
        })
        .collect();
    format!("({})", ors.join(" OR "))
}

/// `m.content` after the first occurrence of *marker*.
// PARITY: _sql_after_marker (84–86)
pub fn _sql_after_marker(marker: &str) -> String {
    format!(
        "SUBSTR(m.content, INSTR(m.content, {}) + {})",
        _sql_literal(marker),
        marker.chars().count()
    )
}

// ── Preview shaping (21–140) ────────────────────────────────────────────────
// Scalar mirrors of _PREVIEW_HEAD_CHARS / _PREVIEW_SCAFFOLD_WINDOW /
// _PREVIEW_MAX_CHARS live in generated constants.

pub fn _preview_scaffolded_sql() -> String {
    format!("m.content LIKE '{}'", crate::skill::SKILL_SCAFFOLD_SQL_LIKE)
}

/// The long-form introduction shared by current and legacy summary prefixes
/// (everything before the first "Do NOT answer").
// PARITY: `_PREVIEW_LONG_FORM_PREFIX = SUMMARY_PREFIX.split("Do NOT answer", 1)[0]`
pub fn _preview_long_form_prefix() -> &'static str {
    SUMMARY_PREFIX.split("Do NOT answer").next().unwrap_or("")
}

pub fn _preview_summary_prefixes() -> [&'static str; 2] {
    [_preview_long_form_prefix(), LEGACY_SUMMARY_PREFIX]
}

// PARITY: _PREVIEW_STANDALONE_SUMMARY_SQL / _PREVIEW_MERGED_* (93–104)
pub fn _preview_standalone_summary_sql() -> String {
    _sql_starts_with("m.content", &_preview_summary_prefixes())
}

pub fn _preview_merged_after_sql() -> String {
    _sql_after_marker(MERGED_SUMMARY_DELIMITER)
}

pub fn _preview_merged_summary_sql() -> String {
    format!(
        "(INSTR(m.content, {}) > 0 AND {})",
        _sql_literal(MERGED_SUMMARY_DELIMITER),
        _sql_starts_with(&_preview_merged_after_sql(), &_preview_summary_prefixes())
    )
}

pub fn _preview_merged_prior_sql() -> String {
    _sql_trim_whitespace(&format!(
        "SUBSTR(m.content, 1, INSTR(m.content, {}) - 1)",
        _sql_literal(MERGED_SUMMARY_DELIMITER)
    ))
}

pub fn _preview_merged_prior_ltrimmed_sql() -> String {
    _sql_ltrim_whitespace(&_preview_merged_prior_sql())
}

pub fn _preview_merged_prior_unwrapped_sql() -> String {
    let ltrimmed = _preview_merged_prior_ltrimmed_sql();
    format!(
        "CASE WHEN SUBSTR({}, 1, {}) = {} THEN {} ELSE {} END",
        ltrimmed,
        MERGED_PRIOR_CONTEXT_HEADER.chars().count(),
        _sql_literal(MERGED_PRIOR_CONTEXT_HEADER),
        _sql_ltrim_whitespace(&format!(
            "SUBSTR({}, {})",
            ltrimmed,
            MERGED_PRIOR_CONTEXT_HEADER.chars().count() + 1
        )),
        _preview_merged_prior_sql(),
    )
}

pub fn _preview_force_user_remainder_sql() -> String {
    _sql_after_marker(SUMMARY_END_MARKER)
}

/// Pure compaction rows are ineligible; force-user-leading and merged
/// carriers only when authentic content survives. `display_kind="hidden"`
/// rows are model-facing scaffolding the gateway never paints.
// PARITY: _PREVIEW_ELIGIBLE_SQL (108–113)
pub fn _preview_eligible_sql() -> String {
    let standalone = _preview_standalone_summary_sql();
    let merged = _preview_merged_summary_sql();
    format!(
        "(COALESCE(m.display_kind, '') <> 'hidden' \
         AND ((NOT {standalone} AND NOT {merged}) \
         OR ({standalone} AND INSTR(m.content, {end}) > 0 \
         AND LENGTH({force}) > 0) \
         OR ({merged} AND LENGTH({prior}) > 0)))",
        standalone = standalone,
        merged = merged,
        end = _sql_literal(SUMMARY_END_MARKER),
        force = _sql_trim_whitespace(&_preview_force_user_remainder_sql()),
        prior = _sql_trim_whitespace(&_preview_merged_prior_unwrapped_sql()),
    )
}

/// The `_preview_raw` SELECT for every listing query (scaffolded rows:
/// head + tail around SKILL_EXCERPT_JOINT).
// PARITY: _PREVIEW_RAW_SELECT (116–123)
pub fn _preview_raw_select() -> String {
    let window = _PREVIEW_SCAFFOLD_WINDOW;
    let window2 = _PREVIEW_SCAFFOLD_WINDOW * 2;
    let head = _PREVIEW_HEAD_CHARS;
    let scaffolded = _preview_scaffolded_sql();
    let sql = _PREVIEW_CONTENT_SQL;
    let joint = crate::skill::SKILL_EXCERPT_JOINT;
    format!(
        "CASE WHEN {standalone} THEN {force} \
         WHEN {merged} THEN {prior} \
         WHEN {scaffolded} AND LENGTH(m.content) > {window2} \
         THEN SUBSTR({sql}, 1, {window}) || '{joint}' || SUBSTR({sql}, -{window}) \
         WHEN {scaffolded} THEN SUBSTR({sql}, 1, {window2}) \
         ELSE SUBSTR({sql}, 1, {head}) END",
        standalone = _preview_standalone_summary_sql(),
        force = _preview_force_user_remainder_sql(),
        merged = _preview_merged_summary_sql(),
        prior = _preview_merged_prior_unwrapped_sql(),
        scaffolded = scaffolded,
        sql = sql,
        joint = joint,
        window = window,
        window2 = window2,
        head = head,
    )
}

/// Turn a `_preview_raw` column into the short preview callers show.
// PARITY: _shape_preview (126–134)
pub fn _shape_preview(raw: impl Into<String>) -> String {
    let text = raw.into();
    let mut text = text.trim().to_string();
    if text.is_empty() {
        return String::new();
    }
    text = text.replace('\n', " ").replace('\r', " ");
    let described = crate::skill::describe_skill_invocation(&text, " — ");
    let base = match described {
        Some(d) => d,
        None => text
            .split(crate::skill::SKILL_EXCERPT_JOINT)
            .next()
            .unwrap_or("")
            .to_string(),
    };
    if base.chars().count() > _PREVIEW_MAX_CHARS {
        let mut out: String = base.chars().take(_PREVIEW_MAX_CHARS).collect();
        out.push_str("...");
        out
    } else {
        base
    }
}

/// Correlated `_preview_raw` column for a `sessions s` row.
// PARITY: _PREVIEW_RAW_SUBQUERY_SQL (138–140)
pub fn _preview_raw_subquery_sql() -> String {
    format!(
        "COALESCE((SELECT {sel} FROM messages m \
         WHERE m.session_id = s.id AND m.role = 'user' AND m.content IS NOT NULL AND {elig} \
         ORDER BY m.timestamp, m.id LIMIT 1), '') AS _preview_raw",
        sel = _preview_raw_select(),
        elig = _preview_eligible_sql(),
    )
}

// ── Child-session classification SQL (144–210) ──────────────────────────────

/// `/branch` child (kept visible, never cascade-deleted): stable marker OR
/// legacy end_reason heuristic.
// PARITY: _BRANCH_CHILD_SQL (145–147) — `{a}` template; multi-space runs
// (12 spaces before WHERE/AND) come from Python adjacent-string joins and
// are byte-significant in the golden.
pub fn _branch_child_sql(alias: &str) -> String {
    format!(
        "{marker} IS NOT NULL OR EXISTS (SELECT 1 FROM sessions p            WHERE p.id = {a}.parent_session_id            AND p.end_reason = 'branched'            AND {a}.started_at >= p.ended_at)",
        marker = _sql_json_extract(&format!("{}.model_config", alias), "$._branched_from"),
        a = alias,
    )
}

/// Compression-continuation child.
// PARITY: _COMPRESSION_CHILD_SQL (148–149) — 8-space runs before WHERE/AND.
pub fn _compression_child_sql(alias: &str) -> String {
    format!(
        "EXISTS (SELECT 1 FROM sessions p        WHERE p.id = {a}.parent_session_id        AND p.end_reason = 'compression')",
        a = alias
    )
}

/// Pre-marker reset-continuation heuristic — child rides its parent's exact
/// non-empty routing key and the parent ended at a reset boundary.
// PARITY: _legacy_reset_child_sql (188–194) — 12-space runs before WHERE/AND.
pub fn _legacy_reset_child_sql(alias: &str, reasons_sql: &str) -> String {
    format!(
        "EXISTS (SELECT 1 FROM sessions p            WHERE p.id = {a}.parent_session_id            AND p.end_reason IN ({reasons})            AND {a}.session_key IS NOT NULL            AND {a}.session_key != ''            AND {a}.session_key = p.session_key)",
        a = alias,
        reasons = reasons_sql,
    )
}

/// Stable `_reset_from` marker, or the same-key fallback for pre-marker rows.
// PARITY: _RESET_CHILD_SQL (199–200)
pub fn _reset_child_sql(alias: &str) -> String {
    format!(
        "{marker} IS NOT NULL OR {legacy}",
        marker = _sql_json_extract(&format!("{}.model_config", alias), "$._reset_from"),
        legacy = _legacy_reset_child_sql(alias, _RESET_END_REASONS_SQL),
    )
}

/// Picker-visible rows: roots + branch/reset children (not subagent runs or
/// compression continuations).
// PARITY: _LISTABLE_CHILD_SQL (203–204) — fixed `s.` alias in the template.
pub fn _listable_child_sql() -> String {
    format!(
        "(s.parent_session_id IS NULL OR {} OR {})",
        _branch_child_sql("s"),
        _reset_child_sql("s"),
    )
}

/// Subagent runs, not branch, reset, or compression children.
// PARITY: _ephemeral_child_sql (207–210)
pub fn _ephemeral_child_sql(alias: &str) -> String {
    format!(
        "({alias}.parent_session_id IS NOT NULL AND NOT ({branch}) \
         AND NOT ({compression}) AND NOT ({reset}))",
        alias = alias,
        branch = _branch_child_sql(alias),
        compression = _compression_child_sql(alias),
        reset = _reset_child_sql(alias),
    )
}

/// Non-continuation child filter used by the compression lineage guard
/// (find_live_compression_child / reopen_orphaned_compression_session): a
/// branched/delegated/tool child is NOT a canonical continuation of its
/// compression parent, so it must not block reopen or count as the unique
/// live child. The two marker bindings (parent_session_id) are added by the
/// caller — the snippet ends with a `!= ?` placeholder for each marker.
// PARITY: hermes_state.py _NON_CONTINUATION_CHILD_FILTER_SQL @ b9aa928 (3661)
pub fn _non_continuation_child_filter_sql(alias: &str) -> String {
    format!(
        concat!(
            "  AND COALESCE(json_extract(COALESCE({alias}model_config, '{{}}'),",
            " '$._branched_from'), '') != ?\n",
            "  AND COALESCE(json_extract(COALESCE({alias}model_config, '{{}}'),",
            " '$._delegate_from'), '') != ?\n",
            "  AND COALESCE({alias}source, '') != 'tool'\n",
        ),
        alias = alias,
    )
}

// ── Recency expressions (213–230) ───────────────────────────────────────────

/// Freshest of *activity* and the latest message timestamp for
/// *session_id_expr*, else *started*. Heartbeats are rate-limited so
/// `last_activity_at` can lag a newer message; never use it alone.
// PARITY: _sql_freshest_of (213–218) — single-line, byte-equal.
pub fn _sql_freshest_of(activity: &str, session_id_expr: &str, started: &str) -> String {
    let msg_max = format!(
        "(SELECT MAX(_act_m.timestamp) FROM messages _act_m WHERE _act_m.session_id = {sid})",
        sid = session_id_expr
    );
    format!(
        "COALESCE((SELECT MAX(_act_v.v) FROM (SELECT {activity} AS v UNION ALL SELECT {msg_max}) _act_v), {started})",
        activity = activity,
        msg_max = msg_max,
        started = started,
    )
}

/// Session recency expression for a `sessions {alias}` row.
// PARITY: _sql_session_last_active (221–223)
pub fn _sql_session_last_active(alias: &str) -> String {
    _sql_freshest_of(
        &format!("{}.last_activity_at", alias),
        &format!("{}.id", alias),
        &format!("{}.started_at", alias),
    )
}

/// Same freshest-of expression keyed by a session-id SQL expression.
// PARITY: _sql_session_last_active_by_id (226–230)
pub fn _sql_session_last_active_by_id(session_id_expr: &str) -> String {
    let activity = format!(
        "(SELECT last_activity_at FROM sessions _act_s WHERE _act_s.id = {sid})",
        sid = session_id_expr
    );
    let started = format!(
        "(SELECT started_at FROM sessions _act_s WHERE _act_s.id = {sid})",
        sid = session_id_expr
    );
    _sql_freshest_of(&activity, session_id_expr, &started)
}

// ── FTS tool-content high-water (262–272) ───────────────────────────────────

/// Bounded-index CASE for FTS trigger/rebuild content: new tool rows above
/// the high-water marker index a prefix; historical rows keep their exact
/// token stream so external-content delete stays valid.
// PARITY: _fts_indexed_content_sql (262–268) — triple-quoted f-string with
// embedded newlines + indentation (byte-significant in the golden).
pub fn _fts_indexed_content_sql(alias: &str) -> String {
    format!(
        "CASE WHEN {alias}.role = 'tool'
              AND {alias}.id > COALESCE((SELECT CAST(value AS INTEGER)
                                         FROM state_meta
                                         WHERE key = '{key}'), -1)
         THEN substr(COALESCE({alias}.content, ''), 1, {chars})
         ELSE {alias}.content END",
        alias = alias,
        key = FTS_TOOL_FULL_CONTENT_HIGH_WATER_KEY,
        chars = FTS_TOOL_CONTENT_PREFIX_CHARS,
    )
}

/// Trigram-index membership predicate over a `sessions` row — shared by the
/// view, the sync triggers, and deferred-backfill INSERT ... SELECTs so they
/// can never disagree about the index boundary.
// PARITY: fts_trigram_session_sql (786–796)
pub fn fts_trigram_session_sql(alias: &str) -> String {
    let q = if alias.is_empty() {
        String::new()
    } else {
        format!("{}.", alias)
    };
    let sources: Vec<String> = FTS_TRIGRAM_EXCLUDED_SOURCES
        .iter()
        .map(|src| format!("'{}'", src))
        .collect();
    format!(
        "{q}source NOT IN ({sources}) AND {marker} IS NULL",
        q = q,
        sources = sources.join(", "),
        marker = _sql_json_extract(&format!("{}model_config", q), "$._delegate_from"),
    )
}

// ── Row probes / bind helpers (278–312) ─────────────────────────────────────
// _ENDED_ROW_SQL / _COMPRESSION_LOCK_ROW_SQL are generated constants.

/// An ended-by-compression row (shared by the messages / compression mixins).
// PARITY: _ended_by_compression (293–294)
pub fn _ended_by_compression(row: Option<(Option<f64>, Option<String>)>) -> bool {
    match row {
        Some((ended_at, end_reason)) => {
            ended_at.is_some() && end_reason.as_deref() == Some("compression")
        }
        None => false,
    }
}

/// `?,?,?` for one bound parameter per element.
// PARITY: _placeholders (297–299)
pub fn _placeholders(count: usize) -> String {
    let mut out = String::new();
    for i in 0..count {
        if i > 0 {
            out.push(',');
        }
        out.push('?');
    }
    out
}

/// Placeholder string for a bound sequence.
pub fn _placeholders_of<T>(items: &[T]) -> String {
    _placeholders(items.len())
}

/// Split *ids* into IN-list chunks of at most `_SQL_IN_CHUNK` elements
/// (SQLite's bound-parameter cap).
// PARITY: _id_chunks (308–312)
pub fn id_chunks<T: Clone>(ids: impl IntoIterator<Item = T>) -> Vec<Vec<T>> {
    id_chunks_sized(ids, _SQL_IN_CHUNK)
}

pub fn id_chunks_sized<T: Clone>(ids: impl IntoIterator<Item = T>, size: usize) -> Vec<Vec<T>> {
    let items: Vec<T> = ids.into_iter().collect();
    if size == 0 {
        return if items.is_empty() {
            Vec::new()
        } else {
            vec![items]
        };
    }
    items.chunks(size).map(|c| c.to_vec()).collect()
}

/// `(st_dev, st_ino)` for *path*, or None. Zero-valued fields (some network
/// FS) would false-positive every replaced-file check, so they count as
/// unknown.
///
/// PARITY: stat_db_file_identity (278–285). PORT SEAMS: the POSIX branch is
/// the production path; on non-unix this returns `None` (unknown identity —
/// fail-safe for replaced-file checks: a replace simply isn't detected,
/// matching upstream's `st_ino == 0 → None` treatment) rather than pulling
/// Windows file-index APIs ahead of the Windows port wave.
pub fn stat_db_file_identity(path: &std::path::Path) -> Option<(i64, i64)> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let st = std::fs::metadata(path).ok()?;
        if st.dev() != 0 && st.ino() != 0 {
            Some((st.dev() as i64, st.ino() as i64))
        } else {
            None
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn golden() -> serde_json::Value {
        serde_json::from_str(include_str!("../../../upstream/golden_state_common.json"))
            .expect("golden fixture")
    }

    #[test]
    fn generated_scalar_consts_match_upstream() {
        assert_eq!(SCHEMA_VERSION, 30);
        assert_eq!(FTS_STORAGE_VERSION, 2);
        assert_eq!(MAX_FTS5_QUERY_CHARS, 2048);
        assert_eq!(FTS_CJK_STALE_KEY, "fts_cjk_stale");
        assert_eq!(FTS_STALE_KEY, "fts_stale");
        assert_eq!(FTS_REBUILD_DEFERRAL_KEY, "fts_rebuild_deferral");
        assert_eq!(FTS_TOOL_CONTENT_PREFIX_CHARS, 8192);
        assert_eq!(
            FTS_TOOL_FULL_CONTENT_HIGH_WATER_KEY,
            "fts_tool_full_content_high_water"
        );
        assert_eq!(AUTO_VACUUM_MIN_FREELIST_RATIO, 0.25);
        assert_eq!(_SQL_IN_CHUNK, 900);
        assert_eq!(_PREVIEW_HEAD_CHARS, 63);
        assert_eq!(_PREVIEW_SCAFFOLD_WINDOW, 400);
        assert_eq!(_PREVIEW_MAX_CHARS, 60);
        assert_eq!(_FTS_TRIGGERS.len(), 6);
        assert_eq!(_FTS_CJK_TRIGGERS.len(), 3);
        // EAGAIN/EWOULDBLOCK(11) EACCES(13) EDEADLK(35) on Linux — deduped.
        assert_eq!(_LOCK_CONTENTION_ERRNOS, [11, 13, 35]);
    }

    #[test]
    fn generated_sql_strings_match_upstream_bytes() {
        let g = golden();
        let sc = &g["string_consts"];
        assert_eq!(SCHEMA_SQL, sc["SCHEMA_SQL"].as_str().unwrap());
        assert_eq!(
            DEFERRED_INDEX_SQL,
            sc["DEFERRED_INDEX_SQL"].as_str().unwrap()
        );
        assert_eq!(FTS_SQL, sc["FTS_SQL"].as_str().unwrap());
        assert_eq!(FTS_TRIGRAM_SQL, sc["FTS_TRIGRAM_SQL"].as_str().unwrap());
        assert_eq!(LEGACY_FTS_SQL, sc["LEGACY_FTS_SQL"].as_str().unwrap());
        assert_eq!(
            LEGACY_FTS_TRIGRAM_SQL,
            sc["LEGACY_FTS_TRIGRAM_SQL"].as_str().unwrap()
        );
        assert_eq!(_ENDED_ROW_SQL, sc["_ENDED_ROW_SQL"].as_str().unwrap());
        assert_eq!(
            _COMPRESSION_LOCK_ROW_SQL,
            sc["_COMPRESSION_LOCK_ROW_SQL"].as_str().unwrap()
        );
        assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS conversation_generations"));
        assert!(SCHEMA_SQL.contains("gateway_hygiene_state"));
        assert!(DEFERRED_INDEX_SQL.contains("idx_messages_display_page"));
        assert!(FTS_SQL.contains("fts_tool_full_content_high_water"));
        assert!(FTS_TRIGRAM_SQL.contains("_delegate_from"));
    }

    #[test]
    fn escape_like_escapes_wildcards_and_backslash() {
        assert_eq!(escape_like("hello"), "hello");
        assert_eq!(escape_like("50%_off"), "50\\%\\_off");
        assert_eq!(escape_like(r"a\b"), r"a\\b");
        assert_eq!(escape_like(""), "");
    }

    #[test]
    fn taxonomy_predicate_matches_oracle() {
        // PARITY: TestAutomaticEndReasonPredicate::test_taxonomy.
        for reason in [
            "tui_shutdown",
            "ws_disconnect",
            "idle_timeout",
            "lru_evict",
            "ws_orphan_reap",
            "agent_close",
            "startup_orphan_reap",
            "superseded_by_resume",
        ] {
            assert!(is_automatic_end_reason(Some(reason)), "{}", reason);
        }
        for reason in [
            Some("compression"),
            Some("session_reset"),
            Some("session_switch"),
            Some("tui_close"),
            None,
            Some(""),
        ] {
            assert!(!is_automatic_end_reason(reason), "{:?}", reason);
        }
        assert_eq!(RECOVERABLE_END_REASONS.len(), 4);
        assert!(_BOUNDARY_END_REASONS.contains(&"new_session"));
        assert!(_BOUNDARY_END_REASONS.contains(&"session_reset"));
        assert!(!_BOUNDARY_END_REASONS.contains(&"tui_shutdown"));
        let g = golden();
        assert_eq!(
            g["boundary_end_reasons"],
            json_sorted(&_BOUNDARY_END_REASONS)
        );
        assert_eq!(
            g["automatic_end_reasons"],
            json_sorted(&_AUTOMATIC_END_REASONS)
        );
        assert_eq!(
            g["reset_end_reasons_sql"].as_str().unwrap(),
            _RESET_END_REASONS_SQL
        );
        assert_eq!(
            g["recoverable_end_reasons_sql"].as_str().unwrap(),
            _RECOVERABLE_END_REASONS_SQL
        );
    }

    fn json_sorted(values: &[&str]) -> serde_json::Value {
        let mut v: Vec<&str> = values.to_vec();
        v.sort_unstable();
        serde_json::json!(v)
    }

    #[test]
    fn child_sql_builders_round_trip() {
        let g = golden();
        assert_eq!(_branch_child_sql("s"), g["branch_s"].as_str().unwrap());
        assert_eq!(
            _compression_child_sql("s"),
            g["compression_s"].as_str().unwrap()
        );
        assert_eq!(_reset_child_sql("s"), g["reset_s"].as_str().unwrap());
        assert_eq!(
            _legacy_reset_child_sql("{a}", _RESET_END_REASONS_SQL),
            g["legacy_reset_a"].as_str().unwrap()
        );
        assert_eq!(_listable_child_sql(), g["listable"].as_str().unwrap());
        assert_eq!(
            _ephemeral_child_sql("s"),
            g["ephemeral_s"].as_str().unwrap()
        );
        let listable = _listable_child_sql();
        assert!(listable.contains("_reset_from"), "reset child now listable");
        let ephem = _ephemeral_child_sql("s");
        assert!(ephem.contains("NOT ("), "{}", ephem);
        assert_eq!(
            ephem.matches("_reset_from").count(),
            1,
            "ephemeral excludes reset children once"
        );
    }

    #[test]
    fn last_active_builders_match_golden_single_line() {
        let g = golden();
        assert_eq!(
            _sql_session_last_active("s"),
            g["last_active_s"].as_str().unwrap()
        );
        assert_eq!(
            _sql_session_last_active_by_id("'abc'"),
            g["last_active_by_id"].as_str().unwrap()
        );
        assert_eq!(
            _sql_freshest_of("s.last_activity_at", "s.id", "s.started_at"),
            g["freshest_of"].as_str().unwrap()
        );
    }

    #[test]
    fn sql_fragment_helpers_match_golden() {
        let g = golden();
        for (i, inp) in ["plain", "it's", "a''b", ""].iter().enumerate() {
            assert_eq!(_sql_literal(inp), g["sql_literal"][i].as_str().unwrap());
        }
        assert_eq!(
            _sql_after_marker("MARK"),
            g["sql_after_marker"].as_str().unwrap()
        );
        assert_eq!(
            _sql_starts_with("m.content", &["[A] ", "[B] "]),
            g["sql_starts_with"].as_str().unwrap()
        );
        assert_eq!(
            _sql_ltrim_whitespace("m.content"),
            g["sql_ltrim"].as_str().unwrap()
        );
        assert_eq!(
            _sql_trim_whitespace("m.content"),
            g["sql_trim"].as_str().unwrap()
        );
        assert_eq!(
            _sql_json_extract("{a}.model_config", "$._branched_from"),
            g["sql_json_extract"].as_str().unwrap()
        );
        assert_eq!(
            _fts_indexed_content_sql("s"),
            g["fts_indexed_content_s"].as_str().unwrap()
        );
        assert_eq!(
            fts_trigram_session_sql(""),
            g["fts_trigram_session_sql_empty"].as_str().unwrap()
        );
        assert_eq!(
            fts_trigram_session_sql("s"),
            g["fts_trigram_session_sql_s"].as_str().unwrap()
        );
        // PARITY: test_predicate_constants_agree — subagent+cron excluded.
        assert!(FTS_TRIGRAM_EXCLUDED_SOURCES.contains(&"subagent"));
        assert!(FTS_TRIGRAM_EXCLUDED_SOURCES.contains(&"cron"));
        let sql = fts_trigram_session_sql("s");
        assert!(sql.starts_with("s.source NOT IN ("), "{}", sql);
        assert!(sql.contains("s.model_config"), "{}", sql);
    }

    #[test]
    fn preview_suite_matches_golden() {
        let g = golden();
        assert_eq!(
            _PREVIEW_CONTENT_SQL,
            g["preview_content_sql"].as_str().unwrap()
        );
        assert_eq!(
            _preview_scaffolded_sql(),
            g["preview_scaffolded_sql"].as_str().unwrap()
        );
        assert_eq!(
            _preview_standalone_summary_sql(),
            g["preview_standalone_summary_sql"].as_str().unwrap()
        );
        assert_eq!(
            _preview_merged_after_sql(),
            g["preview_merged_after_sql"].as_str().unwrap()
        );
        assert_eq!(
            _preview_merged_summary_sql(),
            g["preview_merged_summary_sql"].as_str().unwrap()
        );
        assert_eq!(
            _preview_merged_prior_sql(),
            g["preview_merged_prior_sql"].as_str().unwrap()
        );
        assert_eq!(
            _preview_merged_prior_ltrimmed_sql(),
            g["preview_merged_prior_ltrimmed_sql"].as_str().unwrap()
        );
        assert_eq!(
            _preview_merged_prior_unwrapped_sql(),
            g["preview_merged_prior_unwrapped_sql"].as_str().unwrap()
        );
        assert_eq!(
            _preview_force_user_remainder_sql(),
            g["preview_force_user_remainder_sql"].as_str().unwrap()
        );
        assert_eq!(
            _preview_eligible_sql(),
            g["preview_eligible_sql"].as_str().unwrap()
        );
        assert_eq!(
            _preview_raw_select(),
            g["preview_raw_select"].as_str().unwrap()
        );
        assert_eq!(
            _preview_raw_subquery_sql(),
            g["preview_raw_subquery_sql"].as_str().unwrap()
        );
        assert_eq!(
            _preview_long_form_prefix(),
            g["preview_long_form_prefix"].as_str().unwrap()
        );

        let samples: Vec<&str> = g["samples"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        for (i, sample) in samples.iter().enumerate() {
            assert_eq!(
                _shape_preview(*sample),
                g["shape_preview"][i].as_str().unwrap(),
                "shape_preview case {i}"
            );
        }
    }

    #[test]
    fn skill_recognition_matches_golden() {
        let g = golden();
        let samples: Vec<&str> = g["samples"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        for (i, sample) in samples.iter().enumerate() {
            assert_eq!(
                crate::skill::describe_skill_invocation(sample, " — ").as_deref(),
                g["describe"][i].as_str(),
                "describe case {i}",
            );
            assert_eq!(
                crate::skill::extract_user_instruction_from_skill_message(sample).as_deref(),
                g["extract"][i].as_str(),
                "extract case {i}",
            );
        }
    }

    #[test]
    fn escape_like_matches_golden() {
        let g = golden();
        for (i, inp) in ["plain", "50%_", r"a\b_c.d%", "", "%%%", "_"]
            .iter()
            .enumerate()
        {
            assert_eq!(escape_like(inp), g["escape_like"][i].as_str().unwrap());
        }
    }

    #[test]
    fn placeholders_and_chunks_match_golden() {
        let g = golden();
        assert_eq!(_placeholders(0), g["placeholders"][0].as_str().unwrap());
        assert_eq!(_placeholders(3), g["placeholders"][1].as_str().unwrap());
        assert_eq!(
            _placeholders_of(&["a", "b"]),
            g["placeholders"][2].as_str().unwrap()
        );

        let empty: Vec<i64> = vec![];
        assert_eq!(id_chunks(empty), Vec::<Vec<i64>>::new());
        assert_eq!(
            serde_json::json!(id_chunks(0..5)),
            g["id_chunks"][1],
            "single chunk"
        );
        assert_eq!(
            serde_json::json!(id_chunks_sized(0..7, 3)),
            g["id_chunks"][2]
        );
        let lens: Vec<usize> = id_chunks(0..901).iter().map(|c| c.len()).collect();
        assert_eq!(serde_json::json!(lens), g["id_chunks"][3]);
    }

    #[test]
    fn ended_by_compression_matches_golden() {
        let g = golden();
        let cases: Vec<(Option<(Option<f64>, Option<String>)>, bool)> = vec![
            (None, g["ended_by_compression"][0].as_bool().unwrap()),
            (
                Some((None, Some("compression".into()))),
                g["ended_by_compression"][1].as_bool().unwrap(),
            ),
            (
                Some((Some(1.0), Some("compression".into()))),
                g["ended_by_compression"][2].as_bool().unwrap(),
            ),
            (
                Some((Some(1.0), Some("agent_close".into()))),
                g["ended_by_compression"][3].as_bool().unwrap(),
            ),
        ];
        for (i, (row, expect)) in cases.into_iter().enumerate() {
            assert_eq!(_ended_by_compression(row), expect, "case {i}");
        }
    }

    #[test]
    fn is_automatic_end_reason_matches_golden_table() {
        let g = golden();
        for case in g["is_automatic_end_reason"].as_array().unwrap() {
            let reason = case[0].as_str();
            let expect = case[1].as_bool().unwrap();
            assert_eq!(
                is_automatic_end_reason(reason),
                expect,
                "reason {:?}",
                case[0]
            );
        }
        // JSON null → None path.
        assert!(!is_automatic_end_reason(None));
    }

    #[test]
    fn routed_sessions_setting_env_branch() {
        // Unscoped: env bridge, as before (PARITY: env branch of 26–43).
        unsafe { std::env::set_var("HERMES_TEST_ROUTED_SETTING", "from-env") };
        assert_eq!(
            routed_sessions_setting("any", "HERMES_TEST_ROUTED_SETTING").as_deref(),
            Some("from-env")
        );
        unsafe { std::env::remove_var("HERMES_TEST_ROUTED_SETTING") };
        assert_eq!(routed_sessions_setting("missing", "HERMES_TEST_NOPE"), None);
    }

    #[test]
    fn routed_sessions_setting_override_reads_profile_config() {
        // Override set → this profile's config.yaml sessions.<key>.
        let td = tempfile::TempDir::new().unwrap();
        std::fs::write(
            td.path().join("config.yaml"),
            "sessions:\n  carrier_backend: cron\nother: 1\n",
        )
        .unwrap();
        let token = hermes_constants::set_hermes_home_override(Some(td.path()));
        let got = routed_sessions_setting("carrier_backend", "HERMES_TEST_ROUTED_SETTING");
        let missing = routed_sessions_setting("nope", "HERMES_TEST_ROUTED_SETTING");
        hermes_constants::reset_hermes_home_override(token);
        assert_eq!(got.as_deref(), Some("cron"));
        assert_eq!(missing, None);
    }

    #[test]
    fn stat_db_file_identity_of_real_file() {
        let td = tempfile::TempDir::new().unwrap();
        let p = td.path().join("x.db");
        std::fs::write(&p, b"x").unwrap();
        let (dev, ino) = stat_db_file_identity(&p).expect("identity of existing file");
        assert!(dev != 0 && ino != 0);
        assert_eq!(
            stat_db_file_identity(&td.path().join("missing")),
            None,
            "missing file → None"
        );
    }
}
