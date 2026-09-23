//! YAML load/write helpers: atomic writes, the comment-preserving
//! single-key update, and the comment-preserving full-state save.
//!
//! PARITY: utils.py @ 5d59366 — `IndentDumper` (360–372, exercised through
//! `atomic_yaml_write`'s 2-space sequence indent), `atomic_yaml_write`
//! (375–388), `atomic_roundtrip_yaml_update` (410–448),
//! `_YAML11_AMBIGUOUS_WORDS` + `atomic_roundtrip_yaml_save` (455–492),
//! `fast_safe_load` (503–511). The ruamel round-trip loader/dumper pair
//! maps to line-targeted text editing (update) + line-tree merge (save) —
//! see PORT SEAMS in the save docs.

use crate::atomic::{atomic_write_with, preserve_file_mode, AtomicSpec};
use serde::Serialize;
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;

/// YAML 1.2 (ruamel) resolves `off`/`yes`… as plain strings while every
/// YAML 1.1 reader here (the safe-load family) parses them as booleans —
/// the save path forces quotes for the ambiguous set so a re-read cannot
/// flip `approvals.mode: off` to `False` (upstream
/// `_YAML11_AMBIGUOUS_WORDS`, line 455).
const YAML11_AMBIGUOUS_WORDS: [&str; 10] = [
    "y", "n", "yes", "no", "true", "false", "on", "off", "null", "~",
];

/// Tags the PyYAML/CSafeLoader family accepts (schema/core constructors).
const SAFE_YAML_TAGS: [&str; 13] = [
    "str",
    "int",
    "float",
    "bool",
    "null",
    "seq",
    "map",
    "set",
    "omap",
    "pairs",
    "binary",
    "timestamp",
    "value",
];

/// Reject tag tokens a safe loader would refuse — quote/comment aware so a
/// `!!` inside a scalar or comment is content, not a tag. `!!python/…`
/// (the `test_rejects_arbitrary_python_objects_like_safe_load` oracle) and
/// unknown local `!tags` error out before serde_yaml's permissive
/// Value-acceptance can launder them.
fn reject_unsafe_tags(text: &str) -> Result<(), serde_yaml::Error> {
    use serde::de::Error as _;
    let bytes = text.as_bytes();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut in_comment = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_comment {
            if c == '\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if in_double {
            if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        if in_single {
            if c == '\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        match c {
            '\'' => in_single = true,
            '"' => in_double = true,
            '#' => in_comment = true,
            '!' if i + 1 < bytes.len() && bytes[i + 1] == b'!' => {
                // `!!name` — must be a safe constructor.
                let start = i + 2;
                let mut j = start;
                while j < bytes.len()
                    && ((bytes[j] as char).is_ascii_alphanumeric()
                        || matches!(bytes[j], b'/' | b'-' | b'_' | b'.' | b':'))
                {
                    j += 1;
                }
                let name = &text[start..j];
                let at_delim = j >= bytes.len()
                    || matches!(bytes[j], b' ' | b'\t' | b'\n' | b'\r' | b',' | b']' | b'}');
                if at_delim && !name.is_empty() && !SAFE_YAML_TAGS.contains(&name) {
                    return Err(serde_yaml::Error::custom(format!(
                        "unsafe yaml tag !!{name}"
                    )));
                }
                i = j;
                continue;
            }
            '!' if i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_alphanumeric() => {
                // Unknown local tag `!Foo` — the safe loader rejects these.
                return Err(serde_yaml::Error::custom("unsupported yaml tag"));
            }
            _ => {}
        }
        i += 1;
    }
    Ok(())
}

/// `yaml.safe_load` equivalent — parse a YAML document into a Value.
///
/// Upstream prefers the libyaml C loader with pure-Python fallback; both
/// implement the same restricted safe tag set (rejected above), and empty
/// documents return None — serde_yaml yields `Value::Null` for those (the
/// None analog; callers treat both as "no document").
///
/// PARITY: `fast_safe_load` (509–511) + the safe-loader tag contract
/// (`test_fast_safe_load.py`).
pub fn fast_safe_load(text: &str) -> serde_yaml::Result<serde_yaml::Value> {
    reject_unsafe_tags(text)?;
    serde_yaml::from_str(text)
}

/// Write YAML data to a file atomically.
///
/// Mirrors upstream semantics: parent dirs created; existing mode preserved
/// (`_mode_for_write` with preserve=True); `create_mode` applied only when
/// the target does not exist; temp file mode applied before the replace
/// (no 0600 transit); `extra_content` appended after the YAML dump;
/// symlink targets swapped in place; owner carried across.
///
/// Known serializer divergence (documented in PLAN.md): PyYAML and this
/// emitter emit different whitespace/quoting for identical data. Byte
/// parity with PyYAML is not a goal; value/schema parity, the 2-space
/// IndentDuzzer sequence shape (#31999), and atomicity are.
///
/// PARITY: `atomic_yaml_write` (375–388).
pub fn atomic_yaml_write(
    path: &Path,
    data: &impl Serialize,
    sort_keys: bool,
    extra_content: Option<&str>,
    create_mode: Option<u32>,
) -> std::io::Result<()> {
    let mode = mode_for_write(path, create_mode);
    let prefix = format!(
        ".{}_",
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    );
    atomic_write_with(
        path,
        &AtomicSpec {
            prefix,
            mode,
            preserve_owner: true,
            fsync_dir: false,
        },
        |f| {
            let rendered = render_yaml(data, sort_keys);
            f.write_all(rendered.as_bytes())?;
            if let Some(extra) = extra_content {
                f.write_all(extra.as_bytes())?;
            }
            Ok(())
        },
    )?;
    Ok(())
}

/// `_mode_for_write` with preserve=True (the yaml writer's policy).
fn mode_for_write(path: &Path, create_mode: Option<u32>) -> Option<u32> {
    let mode = preserve_file_mode(path);
    if mode.is_some() || path.exists() {
        mode
    } else {
        create_mode
    }
}

/// Render YAML with insertion order preserved (Mapping) or keys sorted
/// when `sort_keys` is true.
pub fn render_yaml(data: &impl Serialize, sort_keys: bool) -> String {
    render_yaml_impl(data, sort_keys, false)
}

/// Full document render; `yaml11` forces quotes on the YAML-1.1 ambiguous
/// word set (the save path — see `YAML11_AMBIGUOUS_WORDS`).
fn render_yaml_impl(data: &impl Serialize, sort_keys: bool, yaml11: bool) -> String {
    // Serialize through serde_json::Value to preserve insertion order and
    // handle sorting deterministically (HashMap iteration is unordered).
    let value = serde_json::to_value(data).unwrap_or(serde_json::Value::Null);
    let yaml_value = json_to_yaml(value);
    let mut doc = yaml_mapping_to_string(&yaml_value, sort_keys, 0, yaml11);
    if doc.ends_with('\n') {
        doc.pop();
    }
    doc.push('\n');
    doc
}

/// Save-path render: value blocks with YAML-1.1 ambiguous words quoted.
fn render_yaml_quoted(data: &impl Serialize) -> String {
    render_yaml_impl(data, false, true)
}

fn json_to_yaml(v: serde_json::Value) -> serde_yaml::Value {
    match v {
        serde_json::Value::Null => serde_yaml::Value::Null,
        serde_json::Value::Bool(b) => serde_yaml::Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_yaml::Value::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                serde_yaml::Value::Number(u.into())
            } else {
                serde_yaml::Value::Number(n.as_f64().unwrap_or_default().into())
            }
        }
        serde_json::Value::String(s) => serde_yaml::Value::String(s),
        serde_json::Value::Array(items) => {
            serde_yaml::Value::Sequence(items.into_iter().map(json_to_yaml).collect())
        }
        serde_json::Value::Object(map) => {
            let mut ymap = serde_yaml::Mapping::new();
            for (k, v) in map {
                ymap.insert(serde_yaml::Value::String(k), json_to_yaml(v));
            }
            serde_yaml::Value::Mapping(ymap)
        }
    }
}

fn yaml_scalar_string(v: &serde_yaml::Value, yaml11: bool) -> String {
    // Render a scalar the way serde_yaml would (quoted when needed).
    match v {
        serde_yaml::Value::String(s) => {
            if s.is_empty() {
                return "''".to_string();
            }
            // YAML-1.1 ambiguous words (save path): a plain `off` re-reads
            // as False under the safe loader — force the double-quoted form
            // PyYAML's representer would pick for a bool-conflicting string.
            let ambiguous = yaml11 && YAML11_AMBIGUOUS_WORDS.contains(&s.to_lowercase().as_str());
            // Quote only when necessary for YAML plain-scalar safety.
            if !ambiguous
                && s.chars().all(|c| !c.is_control())
                && !s.starts_with(char::is_whitespace)
                && !s.ends_with(char::is_whitespace)
                && !s.contains(':')
                && !s.starts_with([
                    '-', '?', '!', '&', '*', '#', '{', '}', '[', ']', ',', ']', '>', '|', '@', '`',
                    '"', '\'', '%',
                ])
                && !s.contains(" #")
                && !s.contains('\n')
            {
                s.clone()
            } else {
                // serde_yaml-style double-quoted escaping.
                let escaped = s
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\t', "\\t")
                    .replace('\r', "\\r");
                format!("\"{}\"", escaped)
            }
        }
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => {
            serde_yaml::to_string(&serde_yaml::Value::Number(n.clone()))
                .map(|s| s.trim_end().to_string())
                .unwrap_or_default()
        }
        serde_yaml::Value::Null => "null".to_string(),
        // Empty containers reached through the scalar arm (the mapping
        // renderer only routes NON-empty containers elsewhere) must not
        // render as "" — `key: ` parses as null, not `{}`/`[]`.
        serde_yaml::Value::Mapping(m) if m.is_empty() => "{}".to_string(),
        serde_yaml::Value::Sequence(seq) if seq.is_empty() => "[]".to_string(),
        _ => String::new(),
    }
}

fn yaml_mapping_to_string(
    v: &serde_yaml::Value,
    sort_keys: bool,
    depth: usize,
    yaml11: bool,
) -> String {
    let indent = "  ".repeat(depth);
    match v {
        serde_yaml::Value::Mapping(m) => {
            // Collect keys; sort when requested.
            let keys: Vec<&serde_yaml::Value> = {
                let mut ks: Vec<&serde_yaml::Value> = m.keys().collect();
                if sort_keys {
                    ks.sort_by(|a, b| {
                        (a.as_str().unwrap_or("").to_string())
                            .cmp(&b.as_str().unwrap_or("").to_string())
                    });
                }
                ks
            };
            let mut out = String::new();
            for k in keys {
                let key_str = yaml_scalar_string(k, yaml11);
                let value = m.get(k).unwrap();
                match value {
                    serde_yaml::Value::Mapping(_) | serde_yaml::Value::Sequence(_)
                        if !is_flow_empty(value) =>
                    {
                        out.push_str(&format!("{}{}:\n", indent, key_str));
                        out.push_str(&yaml_value_to_string(value, sort_keys, depth + 1, yaml11));
                    }
                    serde_yaml::Value::Null => {
                        out.push_str(&format!("{}{}: null\n", indent, key_str));
                    }
                    other => {
                        out.push_str(&format!(
                            "{}{}: {}\n",
                            indent,
                            key_str,
                            yaml_scalar_string(other, yaml11)
                        ));
                    }
                }
            }
            out
        }
        other => yaml_value_to_string(other, sort_keys, depth, yaml11),
    }
}

fn is_flow_empty(v: &serde_yaml::Value) -> bool {
    match v {
        serde_yaml::Value::Mapping(m) => m.is_empty(),
        serde_yaml::Value::Sequence(s) => s.is_empty(),
        _ => false,
    }
}

fn yaml_value_to_string(
    v: &serde_yaml::Value,
    sort_keys: bool,
    depth: usize,
    yaml11: bool,
) -> String {
    let indent = "  ".repeat(depth);
    match v {
        serde_yaml::Value::Sequence(seq) => {
            let mut out = String::new();
            for item in seq {
                match item {
                    serde_yaml::Value::Mapping(_) | serde_yaml::Value::Sequence(_) => {
                        out.push_str(&format!("{}- ", indent));
                        let rendered = yaml_mapping_to_string(item, sort_keys, 0, yaml11);
                        // First line inline after "- ", rest indented.
                        out.push_str(&rendered.replace('\n', &format!("\n{}  ", indent)));
                        out.push('\n');
                    }
                    other => {
                        out.push_str(&format!(
                            "{}- {}\n",
                            indent,
                            yaml_scalar_string(other, yaml11)
                        ));
                    }
                }
            }
            out
        }
        serde_yaml::Value::Mapping(m) if m.is_empty() => "{}".to_string(),
        serde_yaml::Value::Mapping(_) => yaml_mapping_to_string(v, sort_keys, depth, yaml11),
        other => format!("{}\n", yaml_scalar_string(other, yaml11)),
    }
}

/// Update one dotted YAML key while preserving comments and readable text.
///
/// This is intentionally narrower than `atomic_yaml_write`: it is for
/// user-edited config files where comments, ordering, quoting, and Unicode
/// should survive a single setting mutation. Writes use the same
/// temp-file + fsync + atomic-replace pattern.
///
/// The Rust port preserves comments via **line-targeted editing** for scalar
/// leaf updates (the dominant `config set` use): it locates the leaf key's
/// line at the correct nesting depth and replaces only that line's value.
/// For complex (multi-line) value updates, or when the file cannot be parsed,
/// it falls back to a full parse→update→rewrite (documented divergence:
/// comments lost in the fallback path).
///
/// PARITY: utils.py `atomic_roundtrip_yaml_update` (416–480).
pub fn atomic_roundtrip_yaml_update(
    path: &Path,
    key_path: &str,
    value: &serde_yaml::Value,
) -> std::io::Result<()> {
    let text = if path.exists() {
        std::fs::read_to_string(path)?
    } else {
        String::new()
    };

    let keys: Vec<&str> = key_path.split('.').filter(|s| !s.is_empty()).collect();
    let new_text = crate::yaml::roundtrip_update_text(&text, &keys, value);

    let prefix = format!(
        ".{}_",
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    );
    atomic_write_with(
        path,
        &AtomicSpec {
            prefix,
            mode: preserve_file_mode(path),
            preserve_owner: true,
            fsync_dir: false,
        },
        |f| {
            f.write_all(new_text.as_bytes())?;
            Ok(())
        },
    )?;
    Ok(())
}

/// Line-targeted scalar updater (public for unit testing).
pub fn roundtrip_update_text(text: &str, keys: &[&str], value: &serde_yaml::Value) -> String {
    let value_line = render_leaf_value(value);

    // Empty doc → build a fresh minimal document.
    let trimmed = text.trim();
    if trimmed.is_empty() {
        let mut out = String::new();
        for (i, k) in keys.iter().enumerate() {
            let indent = "  ".repeat(i.min(keys.len() - 1));
            if i == keys.len() - 1 {
                out.push_str(&format!("{}{}: {}\n", indent, k, value_line));
            } else {
                out.push_str(&format!("{}{}:\n", indent, k));
            }
        }
        return out;
    }

    // Map from parsed doc: nested mapping for leading keys, plus a leaf
    // render. We need the leaf's expected indent. Approach: scan lines with
    // an indent stack to find the mapping context for the path, then set the
    // leaf line.
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    if text.ends_with('\n') {
        // keep trailing newline semantics; lines() lost it
        lines.push(String::new());
    }

    let result = update_lines(&mut lines, keys, &value_line);

    // If line-surgery failed, fall back to full parse → update → rewrite.
    if !result {
        return fallback_roundtrip(text, keys, value);
    }

    // Join lines preserving the trailing newline count.
    let mut out = lines.join("\n");
    if text.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    } else if !text.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

fn render_leaf_value(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(s) => {
            if s.is_empty() {
                "''".to_string()
            } else if s.chars().all(|c| !c.is_control())
                && !s.starts_with(char::is_whitespace)
                && !s.ends_with(char::is_whitespace)
                && !s.contains(':')
                && !s.contains(" #")
                && !s.contains('\n')
                && !s.starts_with([
                    '-', '?', '!', '&', '*', '#', '{', '}', '[', ']', ',', '>', '|', '@', '`', '"',
                    '\'', '%',
                ])
            {
                s.clone()
            } else {
                let escaped = s
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\t', "\\t")
                    .replace('\r', "\\r");
                format!("\"{}\"", escaped)
            }
        }
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => {
            serde_yaml::to_string(&serde_yaml::Value::Number(n.clone()))
                .map(|s| s.trim_end().to_string())
                .unwrap_or_else(|_| "null".into())
        }
        serde_yaml::Value::Null => "null".to_string(),
        serde_yaml::Value::Sequence(seq) if seq.is_empty() => "[]".to_string(),
        serde_yaml::Value::Mapping(m) if m.is_empty() => "{}".to_string(),
        // Complex values: render via serde_yaml (multi-line acceptable).
        other => serde_yaml::to_string(other)
            .map(|s| s.trim_end().to_string())
            .unwrap_or_else(|_| "null".into()),
    }
}

/// Attempt in-place line surgery. Returns false when unable to handle.
fn update_lines(lines: &mut Vec<String>, keys: &[&str], value_line: &str) -> bool {
    let Some(last) = keys.last() else {
        return false;
    };
    let leaf = *last;
    let parent_depth = keys.len().saturating_sub(1);

    // Track mapping indentation for each depth as we scan.
    let mut depth_indent: Vec<usize> = Vec::new(); // index = path depth
    let mut found: Option<usize> = None;
    let mut i = 0usize;

    while i < lines.len() {
        let line = lines[i].as_str();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }
        // Parse mapping entry "indent key:" or "indent key: value" or "- item".
        let indent = line.len() - line.trim_start().len();
        if trimmed.starts_with("- ") {
            // List item at this depth — only relevant if current depth is
            // inside the target map; conservative: treat as unknown context
            // and bail when the path needs to descend through it.
            i += 1;
            continue;
        }
        // Find "key:" (possibly "key: rest").
        let Some(colon_pos_rel) = find_mapping_colon(trimmed) else {
            i += 1;
            continue;
        };
        let key = trimmed[..colon_pos_rel].trim();
        if key.is_empty() {
            i += 1;
            continue;
        }
        // Determine depth: pop while indent <= previous depth indent.
        while depth_indent.last().map(|&d| indent <= d).unwrap_or(false) && !depth_indent.is_empty()
        {
            depth_indent.pop();
        }
        let depth = depth_indent.len(); // 0-based depth of this key
        if depth < parent_depth {
            if leaf_match(key, keys[depth]) {
                depth_indent.push(indent);
                i += 1;
                continue;
            }
            // Different key at this depth; descend context unchanged.
            depth_indent.push(indent);
            i += 1;
            continue;
        }
        if depth == parent_depth {
            if leaf_match(key, leaf) {
                // Found the leaf's line: replace the value part, preserving a
                // trailing comment when the new value fits on one line.
                let before = &line[..indent];
                let mut replacement = format!("{}{}: {}", before, key, value_line);
                if let Some(hash_idx) = trimmed.find('#') {
                    if !value_line.contains('\n') {
                        // Preserve the comment exactly, including the space
                        // run separating it from the value (ruamel behavior).
                        let mut comment_start = hash_idx;
                        while comment_start > 0 && trimmed.as_bytes()[comment_start - 1] == b' ' {
                            comment_start -= 1;
                        }
                        let comment = &trimmed[comment_start..];
                        replacement = format!("{}{}: {}{}", before, key, value_line, comment);
                    }
                }
                lines[i] = replacement;
                found = Some(i);
                break;
            }
            // Else: different key at target depth — keep scanning.
            depth_indent.push(indent);
            i += 1;
            continue;
        }
        // depth > parent_depth: deeper than target — skip its subtree until
        // indent returns to <= parent depth.
        i += 1;
    }

    // Leaf not found: append (insert) at the end of the parent map. For a
    // top-level leaf (parent_depth == 0) we append at end of document.
    if found.is_none() {
        if parent_depth == 0 {
            let indent = "".to_string();
            let mut line = format!("{}{}: {}", indent, leaf, value_line);
            if !lines.is_empty() && !lines[lines.len() - 1].is_empty() {
                line = format!("\n{}", line);
            }
            lines.push(line);
            found = Some(lines.len() - 1);
        } else {
            // Nested insert: locate the last block of depth parent_depth-1 note
            // we do not support auto-creating intermediate maps in circular
            // mode; report soft failure.
            return false;
        }
    }
    let _ = found;
    true
}

fn leaf_match(key: &str, want: &str) -> bool {
    key.trim_matches('"').trim_matches('\'') == want.trim_matches('"').trim_matches('\'')
}

fn find_mapping_colon(trimmed: &str) -> Option<usize> {
    for (i, c) in trimmed.char_indices() {
        if c == ':' {
            // ensure not part of a URL-ish value with no space after
            return Some(i);
        }
    }
    None
}

/// Full parse→update→rewrite fallback (loses comments — documented).
fn fallback_roundtrip(text: &str, keys: &[&str], value: &serde_yaml::Value) -> String {
    let mut doc: serde_yaml::Value = serde_yaml::from_str(text).unwrap_or(serde_yaml::Value::Null);
    // Recursive setter with auto-vivification of missing intermediate maps.
    fn set(node: &mut serde_yaml::Value, keys: &[&str], value: serde_yaml::Value) {
        if keys.is_empty() {
            *node = value;
            return;
        }
        if !node.is_mapping() {
            *node = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        }
        let m = node.as_mapping_mut().unwrap();
        let k = serde_yaml::Value::String(keys[0].to_string());
        if keys.len() == 1 {
            m.insert(k, value.clone());
        } else {
            let entry = m
                .entry(k)
                .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
            set(entry, &keys[1..], value.clone());
        }
    }
    set(&mut doc, keys, value.clone());
    let rendered = serde_yaml::to_string(&doc).unwrap_or_default();
    if rendered.ends_with('\n') {
        rendered
    } else {
        format!("{}\n", rendered)
    }
}

// ---------------------------------------------------------------------------
// Comment-preserving full-state save (upstream `atomic_roundtrip_yaml_save`,
// 458–492, backed by `hermes_cli.config.require_readable_config_before_write`
// 1976–2025 for the fail-closed read gate).
//
// PORT SEAMS: ruamel's CommentedMap round-trip maps to a line-tree merge —
// existing lines are kept verbatim (comments, key order, quoting, Unicode)
// and edited per node: scalars update in place (trailing comment kept),
// mappings recurse (interior comments survive), sequence/shape changes
// replace the node's block, missing new_state keys append at their level,
// keys absent from new_state delete their whole block ("explicit
// absence"). The `_FIX_YAML` remediation line omits the backups-dir
// pointer (needs `hermes_constants::display_hermes_home`, below this
// crate's dependency edge); the "this change was not saved" contract the
// oracle pins is preserved verbatim.
// ---------------------------------------------------------------------------

/// Fail-closed lead + `Details:` line — upstream `_refuse_overwrite`
/// (hermes_cli/config.py 1958–1966).
fn refuse_overwrite(
    path: &Path,
    reason: &str,
    fix: &str,
    detail: impl std::fmt::Display,
) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        format!(
            "Your settings file ({}) {}, so this change was not saved. {fix} Details: {}",
            path.display(),
            reason,
            flat_ws(&detail.to_string())
        ),
    )
}

fn flat_ws(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `require_readable_config_before_write` (upstream 1976–2025): an
/// existing-but-unreadable / unparseable / non-mapping config must raise
/// rather than be silently replaced with only `new_state`. Returns the
/// existing text (empty when the file is absent).
fn require_readable_existing(path: &Path) -> std::io::Result<String> {
    if !path.exists() {
        return Ok(String::new());
    }
    let text = std::fs::read_to_string(path).map_err(|e| {
        refuse_overwrite(
            path,
            "cannot be read",
            "Fix the file permissions or move it aside first.",
            e,
        )
    })?;
    match fast_safe_load(&text) {
        Ok(serde_yaml::Value::Null) => Ok(text), // empty document
        Ok(serde_yaml::Value::Mapping(_)) => Ok(text),
        Ok(_) => Err(refuse_overwrite(
            path,
            "must start with settings names, but its top level is not a mapping",
            "Fix it with `hermes config edit` and check with `hermes config check`.",
            "top-level YAML must be a mapping",
        )),
        Err(e) => Err(refuse_overwrite(
            path,
            "has a formatting error",
            "Fix it with `hermes config edit` and check with `hermes config check`.",
            e,
        )),
    }
}

/// Persist a full config-state dict while preserving comments and ordering.
///
/// Comment-safe replacement for a whole-file dump: merges `new_state` into
/// the existing text line-by-line so comments, key order, quoting, and
/// readable Unicode survive; keys missing from `new_state` are deleted
/// (the explicit-absence semantics of the old `cfg.pop(…)` + save
/// pattern). Writes via the shared temp+fsync+replace core; refuses to
/// replace an existing config that cannot be read or parsed (fail closed).
///
/// PARITY: `atomic_roundtrip_yaml_save` (458–492).
pub fn atomic_roundtrip_yaml_save(
    path: &Path,
    new_state: &serde_json::Value,
) -> std::io::Result<()> {
    let Some(new_map) = new_state.as_object() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "atomic_roundtrip_yaml_save: new_state must be a JSON object",
        ));
    };
    let existing = require_readable_existing(path)?;
    let merged = merge_save_text(&existing, new_map);
    let prefix = format!(
        ".{}_",
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    );
    atomic_write_with(
        path,
        &AtomicSpec {
            prefix,
            mode: preserve_file_mode(path),
            preserve_owner: true,
            fsync_dir: false,
        },
        |f| {
            f.write_all(merged.as_bytes())?;
            Ok(())
        },
    )?;
    Ok(())
}

/// One mapping entry located in the existing text: `[start, end)` covers
/// the key line and everything belonging to its block (deeper content and
/// indented comments); a same-or-less-indented comment before the NEXT key
/// stays outside every range (raw-emit between nodes).
struct SaveNode {
    key: String,
    start: usize,
    end: usize,
}

/// Strict mapping-colon finder (simple-key rule: `:` followed by
/// whitespace or EOL, outside quotes).
fn find_mapping_colon_strict(t: &str) -> Option<usize> {
    let b = t.as_bytes();
    let mut in_s = false;
    let mut in_d = false;
    let mut esc = false;
    for (i, &c) in b.iter().enumerate() {
        let ch = c as char;
        if esc {
            esc = false;
            continue;
        }
        if in_d {
            if ch == '\\' {
                esc = true;
            } else if ch == '"' {
                in_d = false;
            }
            continue;
        }
        if in_s {
            if ch == '\'' {
                in_s = false;
            }
            continue;
        }
        match ch {
            '\'' => in_s = true,
            '"' => in_d = true,
            ':' => {
                let next = b.get(i + 1).map(|x| *x as char);
                if matches!(
                    next,
                    None | Some(' ') | Some('\t') | Some('\n') | Some('\r')
                ) {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn unquote_key(raw: &str) -> String {
    let t = raw.trim();
    if t.len() >= 2
        && ((t.starts_with('\'') && t.ends_with('\'')) || (t.starts_with('"') && t.ends_with('"')))
    {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

/// Block extent of the entry starting at `from`: deeper content and
/// indented comments; stops before the next content OR comment at
/// `indent` or above (those belong between nodes).
fn find_block_end(lines: &[String], from: usize, hi: usize, indent: usize) -> usize {
    let mut j = from;
    while j < hi {
        let raw = &lines[j];
        let t = raw.trim();
        if t.is_empty() {
            j += 1;
            continue;
        }
        let ind = raw.len() - raw.trim_start().len();
        if t.starts_with('#') {
            if ind <= indent {
                break; // between-node comment — stays outside the range
            }
            j += 1;
            continue;
        }
        if ind <= indent {
            break;
        }
        j += 1;
    }
    j
}

/// Mapping entries of one level within `[lo, hi)`.
fn parse_level(lines: &[String], lo: usize, hi: usize, indent: usize) -> Vec<SaveNode> {
    let mut nodes = Vec::new();
    let mut i = lo;
    while i < hi {
        let raw = &lines[i];
        let t = raw.trim();
        if t.is_empty() || t.starts_with('#') {
            i += 1;
            continue;
        }
        let ind = raw.len() - raw.trim_start().len();
        if ind < indent {
            break;
        }
        if ind > indent {
            i += 1;
            continue;
        }
        let Some(colon) = find_mapping_colon_strict(t) else {
            i += 1;
            continue;
        };
        let key_raw = t[..colon].trim();
        if key_raw.is_empty() {
            i += 1;
            continue;
        }
        let key = unquote_key(key_raw);
        let end = find_block_end(lines, i + 1, hi, indent);
        nodes.push(SaveNode {
            key,
            start: i,
            end: end.max(i + 1),
        });
        i = end.max(i + 1);
    }
    nodes
}

/// Split a key line's value part into (value, trailing-comment-with-its-spacing).
fn split_trailing_comment(rest: &str) -> (String, String) {
    let b = rest.as_bytes();
    let mut in_s = false;
    let mut in_d = false;
    let mut esc = false;
    for (i, &c) in b.iter().enumerate() {
        let ch = c as char;
        if esc {
            esc = false;
            continue;
        }
        if in_d {
            if ch == '\\' {
                esc = true;
            } else if ch == '"' {
                in_d = false;
            }
            continue;
        }
        if in_s {
            if ch == '\'' {
                in_s = false;
            }
            continue;
        }
        match ch {
            '\'' => in_s = true,
            '"' => in_d = true,
            '#' if i == 0 || b[i - 1] == b' ' || b[i - 1] == b'\t' => {
                // Keep the whitespace run before '#' in the comment so
                // `value` + comment re-join with the original spacing.
                let mut start = i;
                while start > 0 && (b[start - 1] == b' ' || b[start - 1] == b'\t') {
                    start -= 1;
                }
                return (
                    rest[..start].trim_end().to_string(),
                    rest[start..].to_string(),
                );
            }
            _ => {}
        }
    }
    (rest.trim_end().to_string(), String::new())
}

/// Render `{key: value}` through the YAML-1.1-quoted renderer and shift
/// every line right by `indent` spaces (relative structure preserved).
fn render_block(indent: usize, key: &str, value: &serde_json::Value) -> Vec<String> {
    let synthetic = serde_json::json!({ key: value });
    let rendered = render_yaml_quoted(&synthetic);
    let pad = " ".repeat(indent);
    rendered.lines().map(|l| format!("{pad}{l}")).collect()
}

/// Scalar leaf for an in-place key-line update (YAML-1.1 quoted).
fn render_scalar_quoted(value: &serde_json::Value) -> String {
    let yaml_val = json_to_yaml(value.clone());
    yaml_scalar_string(&yaml_val, true)
}

#[derive(Clone, Copy, PartialEq)]
enum NewKind {
    Scalar,
    Seq,
    Map,
}

fn new_kind_of(v: &serde_json::Value) -> NewKind {
    if v.is_array() {
        NewKind::Seq
    } else if v.is_object() {
        NewKind::Map
    } else {
        NewKind::Scalar
    }
}

/// Transform one existing node against its new value; `None` deletes it.
fn transform_node(
    lines: &[String],
    node: &SaveNode,
    new_val: &serde_json::Value,
) -> Option<Vec<String>> {
    let own = &lines[node.start];
    let ind = own.len() - own.trim_start().len();
    let prefix = &own[..ind];
    let trimmed = own.trim();
    let colon = find_mapping_colon_strict(trimmed).unwrap_or(0);
    let key_repr = &trimmed[..colon];
    let rest = &trimmed[colon + 1..];
    let (value_part, comment) = split_trailing_comment(rest);
    let value_trim = value_part.trim();

    let interior_lo = node.start + 1;
    let interior_hi = node.end;
    let has_interior_content = (interior_lo..interior_hi).any(|i| {
        let t = lines[i].trim();
        !t.is_empty() && !t.starts_with('#')
    });
    let first_content_is_seq = (interior_lo..interior_hi)
        .map(|i| lines[i].trim())
        .find(|t| !t.is_empty() && !t.starts_with('#'))
        .map(|t| t.starts_with("- "))
        .unwrap_or(false);
    let old_is_map = (value_trim.is_empty() && has_interior_content && !first_content_is_seq)
        || value_trim == "{}"
        || (value_trim.is_empty() && !has_interior_content);

    match new_kind_of(new_val) {
        NewKind::Scalar => {
            let leaf = render_scalar_quoted(new_val);
            Some(vec![format!("{prefix}{key_repr}: {leaf}{comment}")])
        }
        NewKind::Seq => {
            // Whole-block replace: key line + rendered items (interior of
            // the old value — comments inside a replaced block — drops with
            // the shape change; upstream ruamel behaves the same when the
            // node's value is overwritten wholesale).
            let mut block = vec![format!("{prefix}{key_repr}:{comment}")];
            let synthetic = serde_json::json!({ "k": new_val });
            let rendered = render_yaml_quoted(&synthetic);
            let mut lines_r = rendered.lines();
            lines_r.next(); // drop the synthetic key line
            for l in lines_r {
                block.push(format!("{prefix}{l}"));
            }
            Some(block)
        }
        NewKind::Map => {
            let sub = new_val.as_object().expect("map");
            if old_is_map {
                if sub.is_empty() {
                    return Some(vec![format!("{prefix}{key_repr}: {{}}{comment}")]);
                }
                // Recurse: preserve interior comments/order, upsert/delete
                // children, append new ones.
                let child_indent = (interior_lo..interior_hi)
                    .map(|i| &lines[i])
                    .filter(|l| {
                        let t = l.trim();
                        !t.is_empty() && !t.starts_with('#')
                    })
                    .map(|l| l.len() - l.trim_start().len())
                    .next()
                    .unwrap_or(ind + 2);
                let mut block = vec![format!("{prefix}{key_repr}:{comment}")];
                block.extend(build_level(
                    lines,
                    interior_lo,
                    interior_hi,
                    child_indent,
                    sub,
                ));
                if block.len() == 1 {
                    // Empty interior and… children were all we needed;
                    // build_level already appended them. (len==1 means no
                    // children rendered — sub non-empty guarantees append;
                    // defensive: render directly.)
                    block = vec![format!("{prefix}{key_repr}:{comment}")];
                    block.extend(
                        render_block(child_indent, "", &serde_json::Value::Object(sub.clone()))
                            .into_iter()
                            .skip(1),
                    );
                }
                Some(block)
            } else {
                // Scalar/seq → mapping: replace the whole block.
                let mut block = vec![format!("{prefix}{key_repr}:{comment}")];
                let synthetic = serde_json::json!({ "k": new_val });
                let rendered = render_yaml_quoted(&synthetic);
                let mut lines_r = rendered.lines();
                lines_r.next();
                for l in lines_r {
                    block.push(format!("{prefix}{l}"));
                }
                Some(block)
            }
        }
    }
}

/// Emit the `[lo, hi)` line range merged against `new_map` at `indent`.
fn build_level(
    lines: &[String],
    lo: usize,
    hi: usize,
    indent: usize,
    new_map: &serde_json::Map<String, serde_json::Value>,
) -> Vec<String> {
    let nodes = parse_level(lines, lo, hi, indent);
    let node_keys: HashSet<&str> = nodes.iter().map(|n| n.key.as_str()).collect();
    let mut out = Vec::new();
    let mut i = lo;
    while i < hi {
        if let Some(n) = nodes.iter().find(|n| n.start == i) {
            match new_map.get(&n.key) {
                None => {
                    i = n.end; // explicit absence: drop the block
                    continue;
                }
                Some(v) => {
                    if let Some(block) = transform_node(lines, n, v) {
                        out.extend(block);
                    }
                    i = n.end;
                    continue;
                }
            }
        }
        out.push(lines[i].clone());
        i += 1;
    }
    // Keys present in new_state but missing here: append at this level
    // (upstream `_merge` adds them; the dumper places them last).
    for (k, v) in new_map {
        if !node_keys.contains(k.as_str()) {
            out.extend(render_block(indent, k, v));
        }
    }
    out
}

/// Merge `new_map` into the existing document text.
fn merge_save_text(existing: &str, new_map: &serde_json::Map<String, serde_json::Value>) -> String {
    if existing.trim().is_empty() {
        let synthetic = serde_json::Value::Object(new_map.clone());
        return render_yaml_quoted(&synthetic);
    }
    let had_trailing = existing.ends_with('\n');
    let lines: Vec<String> = existing.lines().map(str::to_string).collect();
    let out = build_level(&lines, 0, lines.len(), 0, new_map);
    let mut text = out.join("\n");
    if had_trailing && !text.ends_with('\n') {
        text.push('\n');
    }
    if !had_trailing && text.ends_with('\n') {
        text.pop();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;
    use tempfile::TempDir;

    #[test]
    fn fast_safe_load_basic() {
        let v = fast_safe_load("a: 1\nb:\n  - x\n  - y\n").unwrap();
        assert_eq!(v["a"].as_i64(), Some(1));
        assert_eq!(v["b"][0].as_str(), Some("x"));
    }

    #[test]
    fn fast_safe_load_fails_on_duplicate() {
        assert!(
            fast_safe_load("a: 1\na: 2\n").is_err(),
            "serde_yaml rejects duplicate keys"
        );
    }

    #[test]
    fn render_yaml_roundtrips() {
        let v = serde_json::json!({"name": "hermes", "nested": {"a": true, "list": [1, 2]}});
        let s = render_yaml(&v, false);
        let parsed: serde_yaml::Value = serde_yaml::from_str(&s).unwrap();
        assert_eq!(parsed["name"].as_str(), Some("hermes"));
        assert_eq!(parsed["nested"]["a"].as_bool(), Some(true));
        assert_eq!(parsed["nested"]["list"][1].as_i64(), Some(2));
    }

    #[test]
    fn render_yaml_sort_keys() {
        let v = serde_json::json!({"b": 1, "a": 2});
        let s = render_yaml(&v, true);
        assert!(s.starts_with("a: 2\nb: 1"), "got:\n{}", s);
        let s2 = render_yaml(&v, false);
        assert!(
            s2.starts_with("b: 1\na: 2") || s2.starts_with("a: 2\nb: 1"),
            "insertion order kept: {}",
            s2
        );
    }

    #[test]
    fn roundtrip_preserves_comments_scalar() {
        let text = "# top comment\n# another\nkey: old  # trailing\n# other section\nnext: 1\n";
        let out = roundtrip_update_text(text, &["key"], &Value::String("new".into()));
        assert!(out.contains("# top comment"), "comments preserved: {}", out);
        assert!(out.contains("# another"), "comments preserved: {}", out);
        assert!(
            out.contains("key: new  # trailing"),
            "scalar + trailing comment updated: {}",
            out
        );
        assert!(out.contains("next: 1"));
    }

    #[test]
    fn roundtrip_updates_nested_dotted_key() {
        let text = "agent:\n  reasoning_effort: high\n  other: 1\n";
        let out = roundtrip_update_text(
            text,
            &["agent", "reasoning_effort"],
            &Value::String("low".into()),
        );
        assert!(out.contains("  reasoning_effort: low"), "got:\n{}", out);
        assert!(out.contains("  other: 1"));
    }

    #[test]
    fn roundtrip_inserts_new_top_level_key() {
        let text = "existing: 1\n";
        let out = roundtrip_update_text(text, &["newkey"], &Value::Bool(true));
        assert!(out.contains("existing: 1"));
        assert!(out.contains("newkey: true"), "got:\n{}", out);
    }

    #[test]
    fn roundtrip_empty_doc_builds_minimal() {
        let out = roundtrip_update_text("", &["a", "b"], &Value::Number(5.into()));
        assert_eq!(out.trim(), "a:\n  b: 5");
    }

    #[test]
    fn atomic_yaml_write_end_to_end() {
        let td = TempDir::new().unwrap();
        let p = td.path().join("cfg.yaml");
        atomic_yaml_write(&p, &serde_json::json!({"a": 1}), false, None, None).unwrap();
        let parsed: serde_yaml::Value =
            serde_yaml::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(parsed["a"].as_i64(), Some(1));
    }

    #[cfg(unix)]
    #[test]
    fn atomic_yaml_write_preserves_mode() {
        use std::os::unix::fs::PermissionsExt;
        let td = TempDir::new().unwrap();
        let p = td.path().join("cfg.yaml");
        std::fs::write(&p, "old: 1\n").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o640)).unwrap();
        atomic_yaml_write(&p, &serde_json::json!({"a": 1}), false, None, None).unwrap();
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
    }
}
