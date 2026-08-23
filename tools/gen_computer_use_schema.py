#!/usr/bin/env python3
"""Generate crates/hermes-tools/src/computer_use_schema.rs from upstream
tools/computer_use/schema.py COMPUTER_USE_SCHEMA (a JSON-compatible dict
literal). Emitted code is reviewed + committed (generator is a helper,
not a build step)."""
import ast, json, os, sys

upstream_roots = [
    os.environ.get("HERMES_UPSTREAM", ""),
    "/home/mustbearn/Projects/Research/hermes-agent-repo",
    "/run/media/mustbearnold/Projects/Research/hermes-agent-repo",
]
upstream_root = next((root for root in upstream_roots if root and os.path.isdir(root)), None)
if upstream_root is None:
    sys.exit("set HERMES_UPSTREAM to the upstream checkout")
UP = os.path.join(upstream_root, "tools/computer_use/schema.py")
tree = ast.parse(open(UP).read())
schema = None
for node in ast.walk(tree):
    if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name) and node.target.id == "COMPUTER_USE_SCHEMA":
        schema = ast.literal_eval(node.value)
        break
if schema is None:
    sys.exit("COMPUTER_USE_SCHEMA not found")
raw = json.dumps(schema, ensure_ascii=False, indent=2)
out = [
    "//! Generic `computer_use` tool schema (model-agnostic).",
    "// PARITY: tools/computer_use/schema.py COMPUTER_USE_SCHEMA +",
    "//         get_computer_use_schema() @ b9aa928 (extracted via",
    "//         tools/gen_computer_use_schema.py; byte-identical JSON).",
    "",
    "use once_cell::sync::Lazy;",
    "",
    "#[rustfmt::skip]",
    "const COMPUTER_USE_SCHEMA_JSON: &str = r####\"",
    raw,
    "\"####;",
    "",
    "/// Return the generic OpenAI function-calling schema.",
    "pub fn get_computer_use_schema() -> &'static serde_json::Value {",
    "    static SCHEMA: Lazy<serde_json::Value> = Lazy::new(|| {",
    "        serde_json::from_str(COMPUTER_USE_SCHEMA_JSON).expect(\"computer_use schema\")",
    "    });",
    "    &SCHEMA",
    "}",
]
path = "crates/hermes-tools/src/computer_use_schema.rs"
open(path, "w").write("\n".join(out))
print(f"wrote {path} ({len(raw)} JSON bytes)")
