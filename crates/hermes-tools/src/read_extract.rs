//! Stdlib document-to-text extraction for `read_file`.
//!
//! PARITY: tools/read_extract.py @ b9aa928 (346 LOC, ported 1:1 for the
//! three stdlib formats). The optional `anydoc` converter (legacy Office,
//! OpenDocument, RTF, EPUB, PDF) is a deferred seam: without the converter
//! those extensions report unsupported, exactly like an absent package.

use std::io::Read;
use std::path::Path;

use serde_json::Value;

pub const EXTRACTABLE_EXTENSIONS: [&str; 3] = [".ipynb", ".docx", ".xlsx"];
pub const ANYDOC_EXTENSIONS: [&str; 0] = [];
// anydoc converter seam: MAX_ANYDOC_BYTES (50MB) applies only when the converter lands.
const MAX_XLSX_ROWS_PER_SHEET: usize = 5000;
const MAX_XLSX_COLS: usize = 256;

const NS_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

#[derive(Debug)]
pub struct ExtractionError(pub String);

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ExtractionError {}

fn extension(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let ext = format!(".{ext}");
    if EXTRACTABLE_EXTENSIONS.contains(&ext.as_str()) {
        return match ext.as_str() {
            ".ipynb" => ".ipynb",
            ".docx" => ".docx",
            ".xlsx" => ".xlsx",
            _ => unreachable!(),
        };
    }
    // anydoc converter seam: unavailable -> no extension.
    if ANYDOC_EXTENSIONS.contains(&ext.as_str()) {
        return "";
    }
    ""
}

pub fn is_extractable_document(path: &str) -> bool {
    !extension(path).is_empty()
}

pub fn extract_document_text(path: &str) -> Result<String, ExtractionError> {
    match extension(path) {
        ".ipynb" => extract_notebook(path),
        ".docx" => extract_docx(path),
        ".xlsx" => extract_xlsx(path),
        _ => Err(ExtractionError(format!("Unsupported document type: {path:?}"))),
    }
}

fn source_text(source: &Value) -> String {
    match source {
        Value::String(s) => s.to_string(),
        Value::Array(items) => items
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

fn extract_notebook(path: &str) -> Result<String, ExtractionError> {
    let content = std::fs::read(path).map_err(|e| ExtractionError(e.to_string()))?;
    let nb: Value = serde_json::from_slice(&content)
        .map_err(|e| ExtractionError(format!("Not a valid notebook: {e}")))?;
    if !nb.is_object() {
        return Err(ExtractionError("Notebook root is not an object".to_string()));
    }
    let nb_obj = nb.as_object().unwrap();
    let cells: Vec<Value> = match nb_obj.get("cells").and_then(Value::as_array) {
        Some(c) => c.clone(),
        None => nb_obj
            .get("worksheets")
            .and_then(Value::as_array)
            .map(|ws| {
                ws.iter()
                    .filter_map(|w| w.as_object())
                    .flat_map(|w| w.get("cells").and_then(Value::as_array).cloned().unwrap_or_default())
                    .collect()
            })
            .unwrap_or_default(),
    };
    if cells.is_empty() {
        return Err(ExtractionError("Notebook contains no cells".to_string()));
    }
    let mut counts = std::collections::HashMap::new();
    let labels = [("markdown", "Markdown"), ("code", "Code"), ("raw", "Raw")];
    let mut out: Vec<String> = Vec::new();
    for cell in &cells {
        let Some(obj) = cell.as_object() else { continue };
        let Some(typ) = obj.get("cell_type").and_then(Value::as_str) else { continue };
        let Some((_, label)) = labels.iter().find(|(t, _)| *t == typ) else { continue };
        let n = *counts.entry(typ.to_string()).and_modify(|c| *c += 1).or_insert(1usize);
        let suffix = if typ == "raw" { String::new() } else { format!(" {n}") };
        let src = source_text(obj.get("source").unwrap_or(&Value::Null));
        out.push(format!("# ── {label} cell{suffix} ──"));
        out.push(src.trim_end_matches('\n').to_string());
        out.push(String::new());
    }
    if out.is_empty() {
        return Err(ExtractionError("Notebook contains no readable cells".to_string()));
    }
    Ok(format!("{}\n", out.join("\n").trim_end_matches('\n')))
}

fn zip_xml(zf: &mut zip::ZipArchive<std::io::BufReader<std::fs::File>>, name: &str) -> Result<String, ExtractionError> {
    let mut entry = zf
        .by_name(name)
        .map_err(|e| ExtractionError(format!("Missing {name}: {e}")))?;
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|e| ExtractionError(e.to_string()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn extract_docx(path: &str) -> Result<String, ExtractionError> {
    let file = std::fs::File::open(path).map_err(|e| ExtractionError(e.to_string()))?;
    let mut zf = zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|e| ExtractionError(format!("Not a valid DOCX: {e}")))?;
    let xml = zip_xml(&mut zf, "word/document.xml")?;
    let doc = roxmltree::Document::parse(&xml)
        .map_err(|e| ExtractionError(format!("Malformed XML in word/document.xml: {e}")))?;

    let mut lines: Vec<String> = Vec::new();
    for para in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "p") {
        let mut buf: Vec<String> = Vec::new();
        for node in para.descendants() {
            if !node.is_element() {
                continue;
            }
            match node.tag_name().name() {
                "t" => buf.push(node.text().unwrap_or("").to_string()),
                "tab" => buf.push("\t".to_string()),
                "br" | "cr" => buf.push("\n".to_string()),
                _ => {}
            }
        }
        lines.extend(buf.join("").split('\n').map(|s| s.to_string()));
    }
    if !lines.iter().any(|l| !l.trim().is_empty()) {
        return Err(ExtractionError("DOCX contains no extractable text".to_string()));
    }
    Ok(format!("{}\n", lines.join("\n").trim_end_matches('\n')))
}

fn extract_xlsx(path: &str) -> Result<String, ExtractionError> {
    let file = std::fs::File::open(path).map_err(|e| ExtractionError(e.to_string()))?;
    let mut zf = zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|e| ExtractionError(format!("Not a valid XLSX: {e}")))?;
    let names: std::collections::HashSet<String> = zf.file_names().map(|s| s.to_string()).collect();
    let shared = shared_strings(&mut zf, &names);
    let sheets = workbook_sheets(&mut zf)?;
    let rels = workbook_rels(&mut zf, &names);
    let mut out: Vec<String> = Vec::new();
    for (name, state, rid) in &sheets {
        if state == "hidden" || state == "veryHidden" {
            continue;
        }
        let part = sheet_part(rels.get(rid).map(|s| s.as_str()).unwrap_or(""));
        if !names.contains(&part) {
            continue;
        }
        let Ok(xml) = zip_xml(&mut zf, &part) else { continue };
        let Ok(rows) = sheet_rows(&xml, &shared) else { continue };
        out.push(format!("# ── Sheet: {name} ──"));
        for row in &rows {
            out.push(row.join("\t"));
        }
        if rows.is_empty() {
            out.push("(empty)".to_string());
        }
        out.push(String::new());
    }
    if out.is_empty() {
        return Err(ExtractionError("XLSX has no visible sheets with content".to_string()));
    }
    Ok(format!("{}\n", out.join("\n").trim_end_matches('\n')))
}

fn shared_strings(zf: &mut zip::ZipArchive<std::io::BufReader<std::fs::File>>, names: &std::collections::HashSet<String>) -> Vec<String> {
    if !names.contains("xl/sharedStrings.xml") {
        return Vec::new();
    }
    let Ok(xml) = zip_xml(zf, "xl/sharedStrings.xml") else { return Vec::new() };
    let Ok(doc) = roxmltree::Document::parse(&xml) else { return Vec::new() };
    let mut out = Vec::new();
    for si in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "si") {
        let text: String = si
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "t")
            .filter_map(|t| t.text())
            .collect();
        out.push(text);
    }
    out
}

fn workbook_sheets(zf: &mut zip::ZipArchive<std::io::BufReader<std::fs::File>>) -> Result<Vec<(String, String, String)>, ExtractionError> {
    let xml = zip_xml(zf, "xl/workbook.xml")?;
    let doc = roxmltree::Document::parse(&xml).map_err(|e| ExtractionError(format!("Malformed XML in xl/workbook.xml: {e}")))?;
    let mut out = Vec::new();
    for sheet in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "sheet") {
        let name = sheet.attribute("name").unwrap_or("Sheet").to_string();
        let state = sheet.attribute("state").unwrap_or("visible").to_string();
        let rid = sheet
            .attribute((NS_REL, "id"))
            .unwrap_or("")
            .to_string();
        out.push((name, state, rid));
    }
    Ok(out)
}

fn workbook_rels(zf: &mut zip::ZipArchive<std::io::BufReader<std::fs::File>>, names: &std::collections::HashSet<String>) -> std::collections::HashMap<String, String> {
    let rels_path = "xl/_rels/workbook.xml.rels";
    if !names.contains(rels_path) {
        return std::collections::HashMap::new();
    }
    let Ok(xml) = zip_xml(zf, rels_path) else { return std::collections::HashMap::new() };
    let Ok(doc) = roxmltree::Document::parse(&xml) else { return std::collections::HashMap::new() };
    let mut out = std::collections::HashMap::new();
    for rel in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "Relationship") {
        if let Some(id) = rel.attribute("Id") {
            out.insert(id.to_string(), rel.attribute("Target").unwrap_or("").to_string());
        }
    }
    out
}

fn sheet_part(target: &str) -> String {
    let target = target.trim_start_matches('/');
    let full = if target.starts_with("xl/") {
        target.to_string()
    } else if target.is_empty() {
        String::new()
    } else {
        format!("xl/{target}")
    };
    posix_normpath(&full)
}

fn posix_normpath(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for comp in path.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    parts.join("/")
}

fn col_index(ref_: &str) -> usize {
    let mut idx = 0usize;
    for ch in ref_.chars() {
        if !ch.is_ascii_alphabetic() {
            break;
        }
        idx = idx * 26 + (ch.to_ascii_uppercase() as usize - 'A' as usize) + 1;
    }
    idx.saturating_sub(1)
}

fn sheet_rows(xml: &str, shared: &[String]) -> Result<Vec<Vec<String>>, ExtractionError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| ExtractionError(format!("Malformed XML in sheet: {e}")))?;
    let mut rows: Vec<Vec<String>> = Vec::new();
    for row in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "row") {
        if rows.len() >= MAX_XLSX_ROWS_PER_SHEET {
            break;
        }
        let mut cells: std::collections::BTreeMap<usize, String> = std::collections::BTreeMap::new();
        let mut max_col: isize = -1;
        let mut cell_count = 0usize;
        for cell in row.descendants().filter(|n| n.is_element() && n.tag_name().name() == "c") {
            cell_count += 1;
            let col = match cell.attribute("r") {
                Some(r) => col_index(r),
                None => (max_col + 1).max(0) as usize,
            };
            if col >= MAX_XLSX_COLS {
                continue;
            }
            let value = cell_value(cell, shared);
            cells.insert(col, value);
            max_col = max_col.max(col as isize);
        }
        let _ = cell_count;
        if max_col >= 0 {
            let row_vals: Vec<String> = (0..=max_col as usize).map(|i| cells.get(&i).cloned().unwrap_or_default()).collect();
            rows.push(row_vals);
        } else {
            rows.push(Vec::new());
        }
    }
    while rows.last().map(|r| r.iter().all(|v| v.trim().is_empty())).unwrap_or(false) {
        rows.pop();
    }
    Ok(rows)
}

fn cell_value(cell: roxmltree::Node, shared: &[String]) -> String {
    let value = cell
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "v")
        .and_then(|v| v.text())
        .unwrap_or("")
        .to_string();
    let typ = cell.attribute("t").unwrap_or("").to_string();
    match typ.as_str() {
        "s" => value.parse::<usize>().ok().and_then(|i| shared.get(i).cloned()).unwrap_or_default(),
        "inlineStr" => {
            let mut text = String::new();
            if let Some(is_node) = cell.children().find(|n| n.is_element() && n.tag_name().name() == "is") {
                for t in is_node.descendants().filter(|n| n.is_element() && n.tag_name().name() == "t") {
                    text.push_str(t.text().unwrap_or(""));
                }
            }
            text
        }
        "b" => {
            if matches!(value.trim(), "1" | "true" | "TRUE") {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        "e" => {
            if value.is_empty() {
                "#ERROR".to_string()
            } else {
                value
            }
        }
        _ => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_detection() {
        assert!(is_extractable_document("a.ipynb"));
        assert!(is_extractable_document("b.DOCX"));
        assert!(!is_extractable_document("c.txt"));
        assert!(!is_extractable_document("d.pdf"));
    }

    #[test]
    fn col_index_maps_letters() {
        assert_eq!(col_index("A1"), 0);
        assert_eq!(col_index("C5"), 2);
        assert_eq!(col_index("AA10"), 26);
    }

    #[test]
    fn posix_normpath_normalizes() {
        assert_eq!(posix_normpath("xl/worksheets/sheet1.xml"), "xl/worksheets/sheet1.xml");
        assert_eq!(posix_normpath("xl/../workbook.xml"), "workbook.xml");
        assert_eq!(posix_normpath("../share"), "share");
    }

    #[test]
    fn notebook_extraction() {
        let dir = std::env::temp_dir();
        let path = dir.join("hfs_test_nb.ipynb");
        std::fs::write(
            &path,
            serde_json::json!({
                "cells": [
                    {"cell_type": "markdown", "source": ["# Title\n", "Some text"]},
                    {"cell_type": "code", "source": ["print('hi')"]}
                ]
            })
            .to_string(),
        )
        .expect("write");
        let out = extract_document_text(&path.to_string_lossy()).expect("extract");
        assert!(out.contains("# Title"));
        assert!(out.contains("print('hi')"));
        assert!(out.contains("Markdown cell 1"));
        assert!(out.contains("Code cell 1"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn invalid_notebook_errors() {
        let dir = std::env::temp_dir();
        let path = dir.join("hfs_test_bad.ipynb");
        std::fs::write(&path, "not json").expect("write");
        let err = extract_document_text(&path.to_string_lossy()).expect_err("must err");
        assert!(err.0.contains("Not a valid notebook"));
        std::fs::remove_file(&path).ok();
    }
}
