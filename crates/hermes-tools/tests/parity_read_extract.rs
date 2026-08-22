//! Parity oracles for stdlib document extraction, mirroring upstream
//! tests/tools/test_read_extract.py @ b9aa928 (is_extractable_document,
//! notebook/docx/xlsx extraction, hidden-sheet omission). The anydoc
//! converter is absent in this port, so PDF etc. report unsupported.

use std::io::Write;
use std::path::PathBuf;

use hermes_tools::read_extract::{extract_document_text, is_extractable_document};

const NS_W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const NS_S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir();
    static CTR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = CTR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    dir.join(format!("rex_{n}_{name}"))
}

fn write_docx(path: &PathBuf, document_xml: &str) {
    let file = std::fs::File::create(path).expect("create");
    let mut zf = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zf.start_file("[Content_Types].xml", opts).unwrap();
    zf.write_all(b"<Types/>").unwrap();
    zf.start_file("word/document.xml", opts).unwrap();
    zf.write_all(document_xml.as_bytes()).unwrap();
    zf.finish().unwrap();
}

fn write_xlsx(
    path: &PathBuf,
    workbook: &str,
    rels: &str,
    shared: Option<&str>,
    sheets: &[(&str, &str)],
) {
    let file = std::fs::File::create(path).expect("create");
    let mut zf = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zf.start_file("xl/workbook.xml", opts).unwrap();
    zf.write_all(workbook.as_bytes()).unwrap();
    zf.start_file("xl/_rels/workbook.xml.rels", opts).unwrap();
    zf.write_all(rels.as_bytes()).unwrap();
    if let Some(s) = shared {
        zf.start_file("xl/sharedStrings.xml", opts).unwrap();
        zf.write_all(s.as_bytes()).unwrap();
    }
    for (part, xml) in sheets {
        zf.start_file(part, opts).unwrap();
        zf.write_all(xml.as_bytes()).unwrap();
    }
    zf.finish().unwrap();
}

#[test]
fn recognized_extensions() {
    assert!(is_extractable_document("a.ipynb"));
    assert!(is_extractable_document("/x/B.DOCX"));
    assert!(is_extractable_document("report.xlsx"));
}

#[test]
fn unrecognized_extensions() {
    assert!(!is_extractable_document("a.py"));
    assert!(!is_extractable_document("a.txt"));
    assert!(!is_extractable_document("a.mp4"));
    // anydoc formats unavailable without the converter.
    assert!(!is_extractable_document("a.pdf"));
}

#[test]
fn unsupported_type_raises() {
    let p = tmp("x.pdf");
    std::fs::write(&p, "%PDF-nope").expect("write");
    let err = extract_document_text(&p.to_string_lossy()).expect_err("err");
    assert!(err.0.contains("Unsupported document type"));
    std::fs::remove_file(&p).ok();
}

#[test]
fn markdown_and_code_in_order() {
    let p = tmp("nb.ipynb");
    let nb = serde_json::json!({
        "cells": [
            {"cell_type": "markdown", "source": ["# Title\n", "para"]},
            {"cell_type": "code", "source": "x = 1\nprint(x)",
             "outputs": [{"output_type": "stream", "text": ["1\n"]}],
             "execution_count": 1}
        ],
        "metadata": {}, "nbformat": 4, "nbformat_minor": 5
    });
    std::fs::write(&p, nb.to_string()).expect("write");
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("# Title"));
    assert!(text.contains("print(x)"));
    assert!(!text.contains("output_type"));
    assert!(!text.contains("execution_count"));
    assert!(text.find("Title").unwrap() < text.find("print(x)").unwrap());
    std::fs::remove_file(&p).ok();
}

#[test]
fn empty_cells_raises() {
    let p = tmp("empty.ipynb");
    std::fs::write(&p, serde_json::json!({"cells": []}).to_string()).expect("write");
    assert!(extract_document_text(&p.to_string_lossy()).is_err());
    std::fs::remove_file(&p).ok();
}

fn doc(body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?><w:document xmlns:w="{NS_W}"><w:body>{body}</w:body></w:document>"#
    )
}

#[test]
fn paragraphs_and_runs() {
    let p = tmp("d.docx");
    write_docx(
        &p,
        &doc(
            "<w:p><w:r><w:t>Hello </w:t></w:r><w:r><w:t>World</w:t></w:r></w:p>\
             <w:p><w:r><w:t>Second</w:t></w:r></w:p>",
        ),
    );
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("Hello World"));
    assert!(text.contains("Second"));
    std::fs::remove_file(&p).ok();
}

#[test]
fn missing_document_xml_raises() {
    let p = tmp("nodoc.docx");
    let file = std::fs::File::create(&p).expect("create");
    let mut zf = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    zf.start_file("other.xml", opts).unwrap();
    zf.write_all(b"<x/>").unwrap();
    zf.finish().unwrap();
    assert!(extract_document_text(&p.to_string_lossy()).is_err());
    std::fs::remove_file(&p).ok();
}

fn build_xlsx(path: &PathBuf, include_hidden: bool) {
    let hidden_sheet = if include_hidden {
        format!(r#"<sheet name="Hidden" sheetId="2" state="hidden" xmlns:r="{NS_R}" r:id="rId2"/>"#)
    } else {
        String::new()
    };
    let workbook = format!(
        r#"<workbook xmlns="{NS_S}" xmlns:r="{NS_R}"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/>{hidden_sheet}</sheets></workbook>"#
    );
    let rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Target="worksheets/sheet1.xml" Type="x"/><Relationship Id="rId2" Target="worksheets/sheet2.xml" Type="x"/></Relationships>"#;
    let shared = format!(
        r#"<sst xmlns="{NS_S}"><si><t>Name</t></si><si><t>Score</t></si><si><t>Alice</t></si></sst>"#
    );
    let sheet1 = format!(
        r#"<worksheet xmlns="{NS_S}"><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row><row r="2"><c r="A2" t="s"><v>2</v></c><c r="B2"><v>95</v></c></row></sheetData></worksheet>"#
    );
    let sheet2 = format!(
        r#"<worksheet xmlns="{NS_S}"><sheetData><row r="1"><c r="A1" t="str"><v>SECRETDATA</v></c></row></sheetData></worksheet>"#
    );
    write_xlsx(
        path,
        &workbook,
        rels,
        Some(&shared),
        &[("xl/worksheets/sheet1.xml", &sheet1), ("xl/worksheets/sheet2.xml", &sheet2)],
    );
}

#[test]
fn visible_sheet_content() {
    let p = tmp("wb.xlsx");
    build_xlsx(&p, true);
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("Data"), "sheet label");
    assert!(text.contains("Name\tScore"), "shared-string header row");
    assert!(text.contains("Alice\t95"), "string + numeric cells");
    // Hidden sheet content omitted.
    assert!(!text.contains("SECRETDATA"));
    std::fs::remove_file(&p).ok();
}

#[test]
fn not_a_zip_raises() {
    let p = tmp("bad.xlsx");
    std::fs::write(&p, b"nope").expect("write");
    assert!(extract_document_text(&p.to_string_lossy()).is_err());
    std::fs::remove_file(&p).ok();
}
