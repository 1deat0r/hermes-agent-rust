#!/usr/bin/env python3
"""Regenerate crates/hermes-state/src/common_constants.rs from the pinned
upstream hermes_state_common.py (+ CJK SQL from hermes_state_fts.py).

Evaluates the pinned module (system python3 imports it cleanly) and emits
Rust constants 1:1: f-string SQL bodies come out fully interpolated, exactly
byte-equal to what Python would hand to executescript. The emitted file is
reviewed + committed (the generator is a helper, not a build step).

Usage:  HERMES_UPSTREAM=.upstream-pin/5d59366 python3 tools/gen_state_common_constants.py
"""
import os
import sys

UP = os.environ.get("HERMES_UPSTREAM", ".upstream-pin/5d59366")
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.normpath(
    os.path.join(HERE, "..", "crates", "hermes-state", "src", "common_constants.rs")
)

sys.path.insert(0, UP)
import hermes_state_common as c  # noqa: E402
import hermes_state_fts as f  # noqa: E402


def rust_str(name: str, body: str) -> str:
    delim = "#"
    while ('"' + delim) in body or (delim + '"') in body:
        delim += "#"
    return f'pub const {name}: &str = r{delim}"{body}"{delim};'


def rust_f64(name: str, value: float) -> str:
    return f"pub const {name}: f64 = {value};"


def rust_int(name: str, value: int) -> str:
    return f"pub const {name}: i64 = {value};"


def rust_usize(name: str, value: int) -> str:
    return f"pub const {name}: usize = {value};"


def rust_str_lit(name: str, value: str) -> str:
    return f'pub const {name}: &str = "{value}";'


def rust_str_tuple(name: str, values) -> str:
    items = ", ".join(f'"{v}"' for v in values)
    return f"pub const {name}: [&str; {len(values)}] = [{items}];"


def rust_i32_set(name: str, values) -> str:
    items = ", ".join(str(int(v)) for v in sorted(values))
    return f"pub const {name}: [i32; {len(values)}] = [{items}];"


out = []
out.append("//! Shared SQL constants and query builders for the SessionDB family.")
out.append("// PARITY: hermes_state_common.py @ 5d59366 (extracted verbatim via")
out.append("//         tools/gen_state_common_constants.py — live module eval so")
out.append("//         f-string SQL bodies are byte-equal to executescript).")
out.append("//         CJK SQL tail from hermes_state_fts.py @ 5d59366.")
out.append("")

# Scalars evaluated from the live module (never hand-maintained again).
out.append(rust_int("SCHEMA_VERSION", c.SCHEMA_VERSION))
out.append(rust_int("FTS_STORAGE_VERSION", c.FTS_STORAGE_VERSION))
out.append(rust_usize("MAX_FTS5_QUERY_CHARS", c.MAX_FTS5_QUERY_CHARS))
out.append(rust_usize("_PREVIEW_HEAD_CHARS", c._PREVIEW_HEAD_CHARS))
out.append(rust_usize("_PREVIEW_SCAFFOLD_WINDOW", c._PREVIEW_SCAFFOLD_WINDOW))
out.append(rust_usize("_PREVIEW_MAX_CHARS", c._PREVIEW_MAX_CHARS))
out.append(rust_usize("_SQL_IN_CHUNK", c._SQL_IN_CHUNK))
out.append(rust_usize("FTS_TOOL_CONTENT_PREFIX_CHARS", c.FTS_TOOL_CONTENT_PREFIX_CHARS))
out.append(rust_f64("AUTO_VACUUM_MIN_FREELIST_RATIO", c.AUTO_VACUUM_MIN_FREELIST_RATIO))
out.append(rust_f64("_FTS_REBUILD_LOCK_TIMEOUT_SECONDS", c._FTS_REBUILD_LOCK_TIMEOUT_SECONDS))
out.append(rust_f64("_FTS_REBUILD_LOCK_POLL_SECONDS", c._FTS_REBUILD_LOCK_POLL_SECONDS))
out.append(rust_f64("_LOCK_BREAK_REACQUIRE_SECONDS", c._LOCK_BREAK_REACQUIRE_SECONDS))
out.append("")
for name in (
    "FTS_CJK_STALE_KEY",
    "FTS_STALE_KEY",
    "FTS_REBUILD_DEFERRAL_KEY",
    "FTS_TOOL_FULL_CONTENT_HIGH_WATER_KEY",
    "_ENDED_ROW_SQL",
    "_COMPRESSION_LOCK_ROW_SQL",
    "_PREVIEW_CONTENT_SQL",
    "SCHEMA_SQL",
    "DEFERRED_INDEX_SQL",
    "FTS_SQL",
    "FTS_TRIGRAM_SQL",
    "LEGACY_FTS_SQL",
    "LEGACY_FTS_TRIGRAM_SQL",
    "FTS_CJK_TABLE_SQL",
    "FTS_CJK_TRIGGER_SQL",
):
    src_obj = f if name.startswith("FTS_CJK_") and hasattr(f, name) else c
    out.append(rust_str(name, getattr(src_obj, name)))
    out.append("")
out.append(rust_str_tuple("_FTS_TRIGGERS", c._FTS_TRIGGERS))
out.append(rust_str_tuple("_FTS_CJK_TRIGGERS", c._FTS_CJK_TRIGGERS))
out.append(rust_str_tuple("FTS_TRIGRAM_EXCLUDED_SOURCES", c.FTS_TRIGRAM_EXCLUDED_SOURCES))
out.append(rust_str_tuple("_RESET_END_REASONS", c._RESET_END_REASONS))
out.append(rust_str_tuple("_RECOVERABLE_END_REASONS", c._RECOVERABLE_END_REASONS))
out.append(rust_str_lit("_RESET_END_REASONS_SQL", c._RESET_END_REASONS_SQL))
out.append(rust_str_lit("_RECOVERABLE_END_REASONS_SQL", c._RECOVERABLE_END_REASONS_SQL))
# frozensets have unstable order — sort for a deterministic Rust array;
# membership semantics are order-independent (golden sorts them too).
out.append(rust_str_tuple("_BOUNDARY_END_REASONS", sorted(c._BOUNDARY_END_REASONS)))
out.append(rust_str_tuple("_AUTOMATIC_END_REASONS", sorted(c._AUTOMATIC_END_REASONS)))
out.append("")
out.append(rust_str_tuple("_LOCK_CONTENTION_ERRNOS", [str(e) for e in sorted(c._LOCK_CONTENTION_ERRNOS)]).replace(
    'pub const _LOCK_CONTENTION_ERRNOS: [&str;',
    'pub const _LOCK_CONTENTION_ERRNOS: [i32;',
).replace(", ".join(f'"{e}"' for e in sorted(c._LOCK_CONTENTION_ERRNOS)),
          ", ".join(str(e) for e in sorted(c._LOCK_CONTENTION_ERRNOS))))
out.append("")

text = "\n".join(out) + "\n"
with open(OUT, "w") as fh:
    fh.write(text)
print(f"wrote {OUT} ({len(text.splitlines())} lines)")
print(
    f"SCHEMA_VERSION={c.SCHEMA_VERSION} FTS_STORAGE_VERSION={c.FTS_STORAGE_VERSION} "
    f"SQL bytes: SCHEMA={len(c.SCHEMA_SQL)} FTS={len(c.FTS_SQL)} TRIGRAM={len(c.FTS_TRIGRAM_SQL)}"
)
