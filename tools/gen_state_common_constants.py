#!/usr/bin/env python3
"""Generate the SQL-constant section of crates/hermes-state/src/common.rs from
upstream hermes_state_common.py. Byte-identical extraction: constants are copied
verbatim into Rust raw string literals; the emitted code is reviewed + committed
(the generator is a helper, not a build step)."""
import re, sys, textwrap

UP = "/home/mustbearn/Projects/Research/hermes-agent-repo/hermes_state_common.py"
src = open(UP).read()

# Constant names assigned a triple-quoted string at module scope.
const_re = re.compile(
    r'^([A-Z_][A-Z0-9_]*)\s*=\s*"""(.*?)"""\s*$',
    re.MULTILINE | re.DOTALL,
)

out = []
out.append("//! Shared SQL constants and query builders for the SessionDB family.")
out.append("// PARITY: hermes_state_common.py @ b9aa928 (extracted verbatim;")
out.append("//         regenerable via tools/gen_state_common_constants.py).")
out.append("")
out.append("pub const SCHEMA_VERSION: i64 = 25;")
out.append("pub const FTS_STORAGE_VERSION: i64 = 1;")
out.append("pub const MAX_FTS5_QUERY_CHARS: usize = 2_048;")
out.append("pub const FTS_CJK_STALE_KEY: &str = \"fts_cjk_stale\";")
out.append("")

for m in const_re.finditer(src):
    name = m.group(1)
    body = m.group(2)
    # Rust raw string delimiters: use r#"..."# unless body contains "#.
    delim = '#'
    while ('"' + delim) in body or (delim + '"') in body:
        delim += '#'
    out.append(f"pub const {name}: &str = r{delim}\"\"\"{body}\"\"\"{delim};")
    out.append("")

# The FTS trigger tuples are Python string tuples, not triple-quoted; emit
# explicitly from the source tuple literal.
triggers = re.search(r"^_FTS_TRIGGERS = \((.*?)\)\s*$", src, re.M | re.S)
if triggers:
    names = re.findall(r'"([^"]+)"', triggers.group(1))
    out.append("pub const _FTS_TRIGGERS: [&str; %d] = [%s];" % (
        len(names), ", ".join(f'"{n}"' for n in names)))
    out.append("")
cjk_triggers = re.search(r"^_FTS_CJK_TRIGGERS = \((.*?)\)\s*$", src, re.M | re.S)
if cjk_triggers:
    names = re.findall(r'"([^"]+)"', cjk_triggers.group(1))
    out.append("pub const _FTS_CJK_TRIGGERS: [&str; %d] = [%s];" % (
        len(names), ", ".join(f'"{n}"' for n in names)))
    out.append("")

open("crates/hermes-state/src/common_constants.rs", "w").write("\n".join(out))
print("wrote crates/hermes-state/src/common_constants.rs (%d lines)" % len(out))
