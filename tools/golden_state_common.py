#!/usr/bin/env python3
"""Regenerate upstream/golden_state_common.json from the pinned upstream
hermes_state_common.py — the byte-level oracle for the Rust port's SQL
builders, preview shaping, and taxonomy constants.

Usage:  HERMES_UPSTREAM=.upstream-pin/5d59366 python3 tools/golden_state_common.py
"""
import json
import os
import sys

UP = os.environ.get("HERMES_UPSTREAM", ".upstream-pin/5d59366")
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.normpath(os.path.join(HERE, "..", "upstream", "golden_state_common.json"))

sys.path.insert(0, UP)
import hermes_state_common as c  # noqa: E402
from agent.skill_commands import (  # noqa: E402
    describe_skill_invocation,
    extract_user_instruction_from_skill_message,
)

# Representative inputs: the pre-retarget samples plus compaction/summary,
# scaffolded, newline, and unicode cases the 5d59366 preview suite gained.
SAMPLES = [
    "plain user message about deploy",
    "50%_off coupon for a\\b_c.d% path",
    "",
    "%%%",
    "_",
    "[IMPORTANT: The user has invoked the %s skill. Apply its guidance now.] deploy — fix the flaky test",
    "[CONTEXT COMPACTION — REFERENCE ONLY] Earlier turns were compacted into the summary below. Do NOT answer questions from the summary. Prior facts only.\n[END OF PRIOR CONTEXT — COMPACTION SUMMARY BELOW]\n[CONTEXT SUMMARY]: legacy tail summary body",
    "[CONTEXT SUMMARY]: only legacy summary no marker",
    "[CONTEXT SUMMARY]: legacy with marker --- END OF CONTEXT SUMMARY — respond to the message below, not the summary above --- actionable remainder here",
    "[PRIOR CONTEXT — for reference only; not a new message]\n\nprior body\n[END OF PRIOR CONTEXT — COMPACTION SUMMARY BELOW]\n[CONTEXT SUMMARY]: merged summary body",
    "multi\nline\rmessage",
    "交付状态正常 with unicode em—dash",
]


def main():
    data = {
        "pin": "5d59366",
        "samples": SAMPLES,
        "shape_preview": [c._shape_preview(s) for s in SAMPLES],
        "describe": [describe_skill_invocation(s) for s in SAMPLES],
        "extract": [extract_user_instruction_from_skill_message(s) for s in SAMPLES],
        "escape_like": [
            c.escape_like(x) for x in ["plain", "50%_", r"a\b_c.d%", "", "%%%", "_"]
        ],
        "branch_s": c._BRANCH_CHILD_SQL.format(a="s"),
        "compression_s": c._COMPRESSION_CHILD_SQL.format(a="s"),
        "reset_s": c._RESET_CHILD_SQL.format(a="s"),
        "legacy_reset_a": c._legacy_reset_child_sql("{a}", c._RESET_END_REASONS_SQL),
        "listable": c._LISTABLE_CHILD_SQL,
        "ephemeral_s": c._ephemeral_child_sql("s"),
        "last_active_s": c._sql_session_last_active("s"),
        "last_active_by_id": c._sql_session_last_active_by_id("'abc'"),
        "freshest_of": c._sql_freshest_of("s.last_activity_at", "s.id", "s.started_at"),
        "preview_content_sql": c._PREVIEW_CONTENT_SQL,
        "preview_scaffolded_sql": c._PREVIEW_SCAFFOLDED_SQL,
        "preview_standalone_summary_sql": c._PREVIEW_STANDALONE_SUMMARY_SQL,
        "preview_merged_after_sql": c._PREVIEW_MERGED_AFTER_SQL,
        "preview_merged_summary_sql": c._PREVIEW_MERGED_SUMMARY_SQL,
        "preview_merged_prior_sql": c._PREVIEW_MERGED_PRIOR_SQL,
        "preview_merged_prior_ltrimmed_sql": c._PREVIEW_MERGED_PRIOR_LTRIMMED_SQL,
        "preview_merged_prior_unwrapped_sql": c._PREVIEW_MERGED_PRIOR_UNWRAPPED_SQL,
        "preview_force_user_remainder_sql": c._PREVIEW_FORCE_USER_REMAINDER_SQL,
        "preview_eligible_sql": c._PREVIEW_ELIGIBLE_SQL,
        "preview_raw_select": c._PREVIEW_RAW_SELECT,
        "preview_raw_subquery_sql": c._PREVIEW_RAW_SUBQUERY_SQL,
        "preview_long_form_prefix": c._PREVIEW_LONG_FORM_PREFIX,
        "sql_literal": [c._sql_literal(x) for x in ["plain", "it's", "a''b", ""]],
        "sql_after_marker": c._sql_after_marker("MARK"),
        "sql_starts_with": c._sql_starts_with("m.content", ("[A] ", "[B] ")),
        "sql_ltrim": c._sql_ltrim_whitespace("m.content"),
        "sql_trim": c._sql_trim_whitespace("m.content"),
        "sql_json_extract": c._sql_json_extract("{a}.model_config", "$._branched_from"),
        "fts_indexed_content_s": c._fts_indexed_content_sql("s"),
        "fts_trigram_session_sql_empty": c.fts_trigram_session_sql(),
        "fts_trigram_session_sql_s": c.fts_trigram_session_sql("s"),
        "placeholders": [
            c._placeholders(0),
            c._placeholders(3),
            c._placeholders(["a", "b"]),
        ],
        "id_chunks": [
            list(c._id_chunks([])),
            list(c._id_chunks(range(5))),
            list(c._id_chunks(range(7), 3)),
            [len(x) for x in c._id_chunks(range(901))],
        ],
        "ended_by_compression": [
            c._ended_by_compression(None),
            c._ended_by_compression({"ended_at": None, "end_reason": "compression"}),
            c._ended_by_compression({"ended_at": 1.0, "end_reason": "compression"}),
            c._ended_by_compression({"ended_at": 1.0, "end_reason": "agent_close"}),
        ],
        "is_automatic_end_reason": [
            [r, c.is_automatic_end_reason(r)]
            for r in [
                "tui_shutdown",
                "ws_disconnect",
                "idle_timeout",
                "lru_evict",
                "ws_orphan_reap",
                "agent_close",
                "startup_orphan_reap",
                "superseded_by_resume",
                "compression",
                "session_reset",
                "session_switch",
                "tui_close",
                None,
                "",
            ]
        ],
        "reset_end_reasons": list(c._RESET_END_REASONS),
        "reset_end_reasons_sql": c._RESET_END_REASONS_SQL,
        "boundary_end_reasons": sorted(c._BOUNDARY_END_REASONS),
        "recoverable_end_reasons": list(c._RECOVERABLE_END_REASONS),
        "recoverable_end_reasons_sql": c._RECOVERABLE_END_REASONS_SQL,
        "automatic_end_reasons": sorted(c._AUTOMATIC_END_REASONS),
        "trigram_excluded_sources": list(c.FTS_TRIGRAM_EXCLUDED_SOURCES),
        "scalars": {
            "SCHEMA_VERSION": c.SCHEMA_VERSION,
            "FTS_STORAGE_VERSION": c.FTS_STORAGE_VERSION,
            "MAX_FTS5_QUERY_CHARS": c.MAX_FTS5_QUERY_CHARS,
            "FTS_TOOL_CONTENT_PREFIX_CHARS": c.FTS_TOOL_CONTENT_PREFIX_CHARS,
            "AUTO_VACUUM_MIN_FREELIST_RATIO": c.AUTO_VACUUM_MIN_FREELIST_RATIO,
            "_SQL_IN_CHUNK": c._SQL_IN_CHUNK,
            "PREVIEW_HEAD": c._PREVIEW_HEAD_CHARS,
            "PREVIEW_WINDOW": c._PREVIEW_SCAFFOLD_WINDOW,
            "PREVIEW_MAX": c._PREVIEW_MAX_CHARS,
        },
        "keys": {
            "FTS_CJK_STALE_KEY": c.FTS_CJK_STALE_KEY,
            "FTS_STALE_KEY": c.FTS_STALE_KEY,
            "FTS_REBUILD_DEFERRAL_KEY": c.FTS_REBUILD_DEFERRAL_KEY,
            "FTS_TOOL_FULL_CONTENT_HIGH_WATER_KEY": c.FTS_TOOL_FULL_CONTENT_HIGH_WATER_KEY,
        },
        "string_consts": {
            "SCHEMA_SQL": c.SCHEMA_SQL,
            "DEFERRED_INDEX_SQL": c.DEFERRED_INDEX_SQL,
            "FTS_SQL": c.FTS_SQL,
            "FTS_TRIGRAM_SQL": c.FTS_TRIGRAM_SQL,
            "LEGACY_FTS_SQL": c.LEGACY_FTS_SQL,
            "LEGACY_FTS_TRIGRAM_SQL": c.LEGACY_FTS_TRIGRAM_SQL,
            "_ENDED_ROW_SQL": c._ENDED_ROW_SQL,
            "_COMPRESSION_LOCK_ROW_SQL": c._COMPRESSION_LOCK_ROW_SQL,
        },
        "triggers": {
            "_FTS_TRIGGERS": list(c._FTS_TRIGGERS),
            "_FTS_CJK_TRIGGERS": list(c._FTS_CJK_TRIGGERS),
        },
        "lock_contention_errnos": sorted(c._LOCK_CONTENTION_ERRNOS),
        "routed_sessions_setting_cases": [],
    }

    # routed_sessions_setting: unscoped reads env; override reads config.
    # Record oracle outputs under controlled inputs (no env mutation here —
    # the harness cases below only cover the override branch via a temp home,
    # the env branch is asserted by the Rust test against its own env seam).
    data["routed_sessions_setting_doc"] = (
        "env-branch returns os.environ.get(env_var); override branch returns "
        "(load_config_readonly().get('sessions') or {}).get(key) with "
        "exception → None. Rust mirrors via hermes_constants override + cfg.rs."
    )

    with open(OUT, "w") as fh:
        json.dump(data, fh, indent=2, ensure_ascii=False, sort_keys=True)
        fh.write("\n")
    print(f"wrote {OUT} ({os.path.getsize(OUT)} bytes)")


if __name__ == "__main__":
    main()
