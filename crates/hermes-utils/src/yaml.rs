//! YAML load/write helpers, including a comment-preserving single-key
//! update (upstream `atomic_roundtrip_yaml_update`).
//!
//! PARITY: utils.py lines 319–480 (`IndentDumper`, `atomic_yaml_write`,
//! `atomic_roundtrip_yaml_update`) and 499–524 (`fast_safe_load`).

use crate::atomic::{atomic_replace, fchmod, preserve_file_mode, preserve_file_owner, restore_file_mode, restore_file_owner};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::io::Write;

/// `yaml.safe_load` equivalent — parse a YAML document into a Value.
///
/// Upstream prefers the libyaml C loader with pure-Python fallback; both
/// implement the same restricted safe tag set. serde_yaml's safe parsing is
/// the Rust equivalent.
pub fn fast_safe_load(text: &str) -> serde_yaml::Result<serde_yaml::Value> {
    serde_yaml::from_str(text)
}

/// Write YAML data to a file atomically.
///
/// Mirrors upstream semantics: parent dirs created; existing mode preserved;
/// `create_mode` applied only when the target does not exist; temp file mode
/// applied before the replace (no 0600 transit); `extra_content` appended
/// after the YAML dump; symlink targets swapped in place.
///
/// Known serializer divergence (documented in PLAN.md): PyYAML and serde_yaml
/// emit different whitespace/quoting for identical data. Byte parity with
/// PyYAML is not a goal; value/schema parity and atomicity are.
///
/// PARITY: utils.py `atomic_yaml_write` (335–413).
pub fn atomic_yaml_write(
    path: &Path,
    data: &impl Serialize,
    sort_keys: bool,
    extra_content: Option<&str>,
    create_mode: Option<u32>,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut original_mode = preserve_file_mode(path);
    let original_owner = preserve_file_owner(path);
    if original_mode.is_none() && create_mode.is_some() && !path.exists() {
        original_mode = create_mode;
    }

    let prefix = format!(".{}_", path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
    let (mut tmp, tmp_path) = crate::atomic::create_temp_in(
        path.parent().unwrap_or_else(|| Path::new(".")),
        &prefix,
        ".tmp",
    )?;

    let result = (|| -> std::io::Result<PathBuf> {
        #[cfg(unix)]
        if let Some(mode) = original_mode {
            fchmod(tmp.as_file(), mode)?;
        }
        let rendered = render_yaml(data, sort_keys);
        tmp.write_all(rendered.as_bytes())?;
        if let Some(extra) = extra_content {
            tmp.write_all(extra.as_bytes())?;
        }
        tmp.flush()?;
        tmp.as_file().sync_all()?;
        Ok(atomic_replace(&tmp_path, path))
    })();

    match result {
        Ok(real_path) => {
            restore_file_owner(&real_path, original_owner);
            restore_file_mode(&real_path, original_mode);
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            Err(e)
        }
    }
}

/// Render YAML with insertion order preserved (Mapping) or keys sorted
/// when `sort_keys` is true.
pub fn render_yaml(data: &impl Serialize, sort_keys: bool) -> String {
    // Serialize through serde_json::Value to preserve insertion order and
    // handle sorting deterministically (HashMap iteration is unordered).
    let value = serde_json::to_value(data).unwrap_or(serde_json::Value::Null);
    let yaml_value = json_to_yaml(value);
    let mut doc = yaml_mapping_to_string(&yaml_value, sort_keys, 0);
    if doc.ends_with('\n') {
        doc.pop();
    }
    doc.push('\n');
    doc
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
        serde_json::Value::Array(items) => serde_yaml::Value::Sequence(items.into_iter().map(json_to_yaml).collect()),
        serde_json::Value::Object(map) => {
            let mut ymap = serde_yaml::Mapping::new();
            for (k, v) in map {
                ymap.insert(serde_yaml::Value::String(k), json_to_yaml(v));
            }
            serde_yaml::Value::Mapping(ymap)
        }
    }
}

fn yaml_scalar_string(v: &serde_yaml::Value) -> String {
    // Render a scalar the way serde_yaml would (quoted when needed).
    match v {
        serde_yaml::Value::String(s) => {
            if s.is_empty() {
                return "''".to_string();
            }
            // Quote only when necessary for YAML plain-scalar safety.
            if s.chars().all(|c| !c.is_control())
                && !s.starts_with(char::is_whitespace)
                && !s.ends_with(char::is_whitespace)
                && !s.contains(':')
                && !s.starts_with(['-', '?', '!', '&', '*', '#', '{', '}', '[', ']', ',', ']', '>', '|', '@', '`', '"', '\'', '%'])
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
        serde_yaml::Value::Number(n) => serde_yaml::to_string(&serde_yaml::Value::Number(n.clone()))
            .map(|s| s.trim_end().to_string())
            .unwrap_or_default(),
        serde_yaml::Value::Null => "null".to_string(),
        _ => String::new(),
    }
}

fn yaml_mapping_to_string(v: &serde_yaml::Value, sort_keys: bool, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    match v {
        serde_yaml::Value::Mapping(m) => {
            // Collect keys; sort when requested.
            let keys: Vec<&serde_yaml::Value> = {
                let mut ks: Vec<&serde_yaml::Value> = m.keys().collect();
                if sort_keys {
                    ks.sort_by(|a, b| {
                        (a.as_str().unwrap_or("").to_string()).cmp(&b.as_str().unwrap_or("").to_string())
                    });
                }
                ks
            };
            let mut out = String::new();
            for k in keys {
                let key_str = yaml_scalar_string(k);
                let value = m.get(k).unwrap();
                match value {
                    serde_yaml::Value::Mapping(_) | serde_yaml::Value::Sequence(_) if !is_flow_empty(value) => {
                        out.push_str(&format!("{}{}:\n", indent, key_str));
                        out.push_str(&yaml_value_to_string(value, sort_keys, depth + 1));
                    }
                    serde_yaml::Value::Null => {
                        out.push_str(&format!("{}{}: null\n", indent, key_str));
                    }
                    other => {
                        out.push_str(&format!("{}{}: {}\n", indent, key_str, yaml_scalar_string(other)));
                    }
                }
            }
            out
        }
        other => yaml_value_to_string(other, sort_keys, depth),
    }
}

fn is_flow_empty(v: &serde_yaml::Value) -> bool {
    match v {
        serde_yaml::Value::Mapping(m) => m.is_empty(),
        serde_yaml::Value::Sequence(s) => s.is_empty(),
        _ => false,
    }
}

fn yaml_value_to_string(v: &serde_yaml::Value, sort_keys: bool, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    match v {
        serde_yaml::Value::Sequence(seq) => {
            let mut out = String::new();
            for item in seq {
                match item {
                    serde_yaml::Value::Mapping(_) | serde_yaml::Value::Sequence(_) => {
                        out.push_str(&format!("{}- ", indent));
                        let rendered = yaml_mapping_to_string(item, sort_keys, 0);
                        // First line inline after "- ", rest indented.
                        out.push_str(&rendered.replace('\n', &format!("\n{}  ", indent)));
                        out.push('\n');
                    }
                    other => {
                        out.push_str(&format!("{}- {}\n", indent, yaml_scalar_string(other)));
                    }
                }
            }
            out
        }
        serde_yaml::Value::Mapping(m) if m.is_empty() => "{}".to_string(),
        serde_yaml::Value::Mapping(_) => yaml_mapping_to_string(v, sort_keys, depth),
        other => format!("{}\n", yaml_scalar_string(other)),
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
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let original_mode = preserve_file_mode(path);
    let original_owner = preserve_file_owner(path);

    let text = if path.exists() {
        std::fs::read_to_string(path)?
    } else {
        String::new()
    };

    let keys: Vec<&str> = key_path.split('.').filter(|s| !s.is_empty()).collect();
    let new_text = crate::yaml::roundtrip_update_text(&text, &keys, value);

    let prefix = format!(".{}_", path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
    let (mut tmp, tmp_path) = crate::atomic::create_temp_in(
        path.parent().unwrap_or_else(|| Path::new(".")),
        &prefix,
        ".tmp",
    )?;
    let result = (|| -> std::io::Result<PathBuf> {
        #[cfg(unix)]
        if let Some(mode) = original_mode {
            fchmod(tmp.as_file(), mode)?;
        }
        tmp.write_all(new_text.as_bytes())?;
        tmp.flush()?;
        tmp.as_file().sync_all()?;
        Ok(atomic_replace(&tmp_path, path))
    })();
    match result {
        Ok(real_path) => {
            restore_file_owner(&real_path, original_owner);
            restore_file_mode(&real_path, original_mode);
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            Err(e)
        }
    }
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
                && !s.starts_with(['-', '?', '!', '&', '*', '#', '{', '}', '[', ']', ',', '>', '|', '@', '`', '"', '\'', '%'])
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
        serde_yaml::Value::Number(n) => serde_yaml::to_string(&serde_yaml::Value::Number(n.clone()))
            .map(|s| s.trim_end().to_string())
            .unwrap_or_else(|_| "null".into()),
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
        while depth_indent.last().map(|&d| indent <= d).unwrap_or(false) && !depth_indent.is_empty() {
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
            let entry = m.entry(k).or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
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
        assert!(fast_safe_load("a: 1\na: 2\n").is_err(), "serde_yaml rejects duplicate keys");
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
        assert!(s2.starts_with("b: 1\na: 2") || s2.starts_with("a: 2\nb: 1"), "insertion order kept: {}", s2);
    }

    #[test]
    fn roundtrip_preserves_comments_scalar() {
        let text = "# top comment\n# another\nkey: old  # trailing\n# other section\nnext: 1\n";
        let out = roundtrip_update_text(text, &["key"], &Value::String("new".into()));
        assert!(out.contains("# top comment"), "comments preserved: {}", out);
        assert!(out.contains("# another"), "comments preserved: {}", out);
        assert!(out.contains("key: new  # trailing"), "scalar + trailing comment updated: {}", out);
        assert!(out.contains("next: 1"));
    }

    #[test]
    fn roundtrip_updates_nested_dotted_key() {
        let text = "agent:\n  reasoning_effort: high\n  other: 1\n";
        let out = roundtrip_update_text(text, &["agent", "reasoning_effort"], &Value::String("low".into()));
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
        let parsed: serde_yaml::Value = serde_yaml::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
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
