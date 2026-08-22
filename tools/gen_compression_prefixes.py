#!/usr/bin/env python3
"""Generate compression-handoff prefix constants from upstream
agent/context_compressor.py (SUMMARY_PREFIX / LEGACY_SUMMARY_PREFIX /
_HISTORICAL_SUMMARY_PREFIXES), vendored into a Rust module + golden JSON.

The classifier these feed (`ContextCompressor.classify_summary_content`) lives
in the agent crate (P2); until then search.rs's `list_recent_user_messages`
needs the exact prefixes to skip persisted compaction-handoff rows. The
generator is a helper, not a build step — emitted code is reviewed + committed.
"""
import ast, json, sys, pathlib

UP = "/home/mustbearn/Projects/Research/hermes-agent-repo/agent/context_compressor.py"
OUT_RS = pathlib.Path(__file__).resolve().parent.parent / "crates/hermes-state/src/compression_prefix.rs"
OUT_JSON = pathlib.Path(__file__).resolve().parent.parent / "upstream/golden_compression_prefixes.json"

HEADING = "## Historical Task Snapshot"

def eval_str(node: ast.AST) -> str:
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return node.value
    if isinstance(node, ast.JoinedStr):
        out = []
        for v in node.values:
            if isinstance(v, ast.Constant):
                out.append(v.value)
            elif (isinstance(v, ast.FormattedValue)
                  and isinstance(v.value, ast.Name)
                  and v.value.id == "HISTORICAL_TASK_HEADING"):
                out.append(HEADING)
            else:
                raise ValueError(f"unhandled f-string part: {ast.dump(v)}")
        return "".join(out)
    raise ValueError(ast.dump(node))

src = open(UP).read()
mod = ast.parse(src)
found = {}
for node in ast.walk(mod):
    if not isinstance(node, ast.Assign):
        continue
    for t in node.targets:
        if isinstance(t, ast.Name) and t.id in (
            "SUMMARY_PREFIX", "_HISTORICAL_SUMMARY_PREFIXES", "LEGACY_SUMMARY_PREFIX"
        ):
            v = node.value
            if isinstance(v, ast.Tuple):
                found[t.id] = [eval_str(e) for e in v.elts]
            elif isinstance(v, (ast.Constant, ast.JoinedStr)):
                found[t.id] = eval_str(v)
            else:
                raise ValueError(f"unsupported {t.id}: {ast.dump(v)}")

_summary_found = found["SUMMARY_PREFIX"]
summary = _summary_found[0] if isinstance(_summary_found, list) else _summary_found
legacy = found["LEGACY_SUMMARY_PREFIX"]
historical = found["_HISTORICAL_SUMMARY_PREFIXES"]

def rs_str(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'

lines = [
    "//! Compression-handoff prefix constants vendored from upstream",
    "//! agent/context_compressor.py @ b9aa928 (regenerable via",
    "//! tools/gen_compression_prefixes.py). Used by search.rs to skip",
    "//! persisted compaction-handoff rows in list_recent_user_messages.",
    "// PARITY: agent/context_compressor.py SUMMARY_PREFIX /",
    "//         LEGACY_SUMMARY_PREFIX / _HISTORICAL_SUMMARY_PREFIXES.",
    "",
    'pub const SUMMARY_PREFIX: &str = ' + rs_str(summary) + ";",
    "",
    'pub const LEGACY_SUMMARY_PREFIX: &str = ' + rs_str(legacy) + ";",
    "",
    "pub const HISTORICAL_SUMMARY_PREFIXES: &[&str] = &[",
]
for h in historical:
    lines.append("    " + rs_str(h) + ",")
lines.append("];")
lines.append("")

MERGED = "[END OF PRIOR CONTEXT — COMPACTION SUMMARY BELOW]"
# Insert the delimiter right after the module doc header (index 7), keeping
# the `//!` docs as an outer doc comment on the first item.
merged_line = "pub const MERGED_SUMMARY_DELIMITER: &str = " + rs_str(MERGED) + ";"
lines = lines[:7] + [merged_line, ""] + lines[7:]
OUT_RS.write_text("\n".join(lines))
json.dump(
    {
        "summary_prefix": summary,
        "legacy_summary_prefix": legacy,
        "historical_summary_prefixes": historical,
        "merged_summary_delimiter": MERGED,
    },
    open(OUT_JSON, "w"),
    ensure_ascii=False,
    indent=2,
)
print(f"wrote {OUT_RS} ({len(summary)}-char current prefix, {len(historical)} historical)")
