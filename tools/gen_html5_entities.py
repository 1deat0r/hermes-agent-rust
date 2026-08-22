#!/usr/bin/env python3
"""Generate crates/hermes-tools/src/html5_entities.rs from CPython's
html.entities.html5 + html._invalid_charrefs/_invalid_codepoints.

The emitted table is the exact HTML5 named-character-reference map including
legacy semicolon-less forms; the Rust decoder in tts_text_normalize.rs mirrors
CPython Lib/html/__init__.py `html.unescape` (the call site in
tools/tts_text_normalize.py `html.unescape(str(text))`). Emitted code is
reviewed + committed (generator is a helper, not a build step)."""
import html.entities as E
import html

def esc(s: str) -> str:
    """Rust string literal escaping for a &str."""
    out = []
    for ch in s:
        c = ord(ch)
        if ch == '"':
            out.append('\\"')
        elif ch == '\\':
            out.append('\\\\')
        elif c < 0x20 or c == 0x7f:
            out.append(f"\\u{{{c:x}}}")
        elif c < 0x80:
            out.append(ch)
        else:
            out.append(f"\\u{{{c:x}}}")
    return "".join(out)

out = []
out.append("//! HTML5 named character references + CPython invalid-ref tables.")
out.append("// PARITY: html.entities.html5 + html._invalid_charrefs /")
out.append("//         html._invalid_codepoints @ b9aa928 (regenerable via")
out.append("//         tools/gen_html5_entities.py). Decoder mirrors CPython")
out.append("//         Lib/html/__init__.py `html.unescape`.")
out.append("")

# HTML5 named entity table (name -> replacement); keys may or may not end in ';'.
items = sorted(E.html5.items(), key=lambda kv: kv[0])
out.append("/// Sorted (name, replacement) pairs; binary-search by name.")
out.append("pub(crate) static HTML5_ENTITIES: &[(&str, &str)] = &[")
for name, value in items:
    out.append(f'    ("{esc(name)}", "{esc(value)}"),')
out.append("];")
out.append("")

# CPython html._invalid_charrefs: numeric codepoint -> replacement char.
inv = sorted(html._invalid_charrefs.items())
out.append("/// _invalid_charrefs: numeric refs remapped per the HTML5 spec.")
out.append("pub(crate) static INVALID_CHARREFS: &[(u32, &str)] = &[")
for num, repl in inv:
    out.append(f'    ({num}, "{esc(repl)}"),')
out.append("];")
out.append("")

# CPython html._invalid_codepoints: disallowed codepoints decoded to ''.
cp = sorted(html._invalid_codepoints)
out.append("/// _invalid_codepoints: codepoints silently dropped by unescape.")
out.append("pub(crate) static INVALID_CODEPOINTS: &[u32] = &[")
for num in cp:
    out.append(f"    {num},")
out.append("];")
out.append("")

path = "crates/hermes-tools/src/html5_entities.rs"
with open(path, "w") as f:
    f.write("\n".join(out))
print(f"wrote {path}: {len(items)} entities, {len(inv)} invalid charrefs, {len(cp)} codepoints")
