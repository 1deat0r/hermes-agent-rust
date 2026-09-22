//! Parity oracles for structured-document extraction, mirroring upstream
//! tests/tools/test_read_extract.py @ 5d59366 (TestIsExtractable,
//! TestAnydocSizeCap, TestAnydocAbsent, TestAnydocInitLifecycle,
//! TestNotebookExtraction, TestDocxExtraction, TestXlsxExtraction,
//! TestPdfCoverageNote + the read_extract-owned halves of the bytes path).
//!
//! The upstream real-binding class (TestAnydocExtraction) requires the
//! optional firecrawl-anydoc package and skips without it — upstream's
//! skip condition; the production loader here is the same "unavailable"
//! path (no Python import machinery). The read_file_tool integration
//! class belongs to the tools.file_tools row.
//!
//! Upstream monkeypatches module globals (`_anydoc_module`,
//! `MAX_ANYDOC_BYTES`, `_pdf_page_texts`, importlib); the Rust analogs are
//! the provider/cap/pdftotext-override seams. ALL tests in this binary
//! serialize on one lock — those globals are process-wide.
//! Tier: `unit`.

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use hermes_tools::read_extract::{
    anydoc_cache_is_unset, coverage_note_from_texts, extract_anydoc, extract_anydoc_bytes,
    extract_document_bytes, extract_document_text, hosted_ocr_available, is_extractable_document,
    needs_ocr_warning, pdf_page_texts, reset_anydoc_cache, reset_hosted_ocr_for_test,
    set_anydoc_provider, set_anydoc_retry_seconds, set_max_anydoc_bytes,
    set_pdftotext_override_for_test, wire_hosted_ocr, AnydocConverter, AnydocError,
    ExtractionError, PDF_GAP_MAP_MAX_ENTRIES,
};

/// Serializes every test in this binary: the anydoc cache, retry cooldown,
/// size caps, hosted-OCR wiring and pdftotext lookup are process-global.
static LOCK: Mutex<()> = Mutex::new(());

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
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
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
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
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

fn write_notebook(path: &PathBuf, cells: serde_json::Value) {
    let nb = serde_json::json!({
        "cells": cells, "metadata": {}, "nbformat": 4, "nbformat_minor": 5
    });
    std::fs::write(path, nb.to_string()).expect("write nb");
}

/// A fake anydoc converter counting conversions, driven by tests.
#[derive(Debug, Default)]
struct FakeAnydoc {
    md_calls: AtomicUsize,
    bytes_calls: AtomicUsize,
    /// When set, to_markdown returns this instead of "converted\n".
    text: Mutex<Option<String>>,
}

impl FakeAnydoc {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    fn with_text(text: &str) -> Arc<Self> {
        Arc::new(Self {
            md_calls: AtomicUsize::new(0),
            bytes_calls: AtomicUsize::new(0),
            text: Mutex::new(Some(text.to_string())),
        })
    }
    fn output(&self) -> String {
        self.text
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| "converted\n".to_string())
    }
}

impl AnydocConverter for FakeAnydoc {
    fn to_markdown(&self, _path: &std::path::Path) -> Result<String, AnydocError> {
        self.md_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.output())
    }
    fn to_markdown_bytes(&self, _data: &[u8]) -> Result<String, AnydocError> {
        self.bytes_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.output())
    }
}

fn set_fake_anydoc() -> Arc<FakeAnydoc> {
    reset_anydoc_cache();
    let fake = FakeAnydoc::new();
    let slot = fake.clone();
    set_anydoc_provider(Some(Box::new(move || Some(slot.clone() as _))));
    fake
}

fn default_anydoc_state() {
    set_anydoc_provider(None);
    reset_anydoc_cache();
    set_anydoc_retry_seconds(300.0);
    set_max_anydoc_bytes(50 * 1024 * 1024);
    set_pdftotext_override_for_test(None);
}

// ---------------------------------------------------------------------------
// TestIsExtractable
// ---------------------------------------------------------------------------

#[test]
fn recognized_extensions() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    assert!(is_extractable_document("a.ipynb"));
    assert!(is_extractable_document("/x/B.DOCX"));
    assert!(is_extractable_document("report.xlsx"));
}

#[test]
fn unrecognized_extensions() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    assert!(!is_extractable_document("a.py"));
    assert!(!is_extractable_document("a.txt"));
    assert!(!is_extractable_document("a.mp4"));
}

#[test]
fn anydoc_extensions_track_availability() {
    // ORACLE: `test_anydoc_extensions_track_availability` — pdf/odt/epub
    // are extractable exactly when the optional converter loads.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    assert!(!is_extractable_document("a.pdf"));
    assert!(!is_extractable_document("a.odt"));
    assert!(!is_extractable_document("a.epub"));
    let _ = set_fake_anydoc();
    assert!(is_extractable_document("a.pdf"));
    assert!(is_extractable_document("a.odt"));
    assert!(is_extractable_document("a.epub"));
    // Reset returns to unavailable (default loader has no package).
    default_anydoc_state();
    assert!(!is_extractable_document("a.pdf"));
}

// ---------------------------------------------------------------------------
// TestAnydocAbsent
// ---------------------------------------------------------------------------

#[test]
fn anydoc_absent_contract() {
    // ORACLE: TestAnydocAbsent — pdf/rtf not extractable; direct
    // `_extract_anydoc` raises the teaching error; stdlib unaffected.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    assert!(!is_extractable_document("a.pdf"));
    assert!(!is_extractable_document("a.rtf"));
    let err = extract_anydoc("/tmp/whatever.pdf").expect_err("must raise");
    assert!(
        err.0.contains("optional anydoc") && err.0.contains("pip install firecrawl-anydoc"),
        "{}",
        err.0
    );
    assert!(is_extractable_document("a.ipynb"));
    assert!(is_extractable_document("a.docx"));
    assert!(is_extractable_document("a.xlsx"));
}

// ---------------------------------------------------------------------------
// TestAnydocSizeCap
// ---------------------------------------------------------------------------

#[test]
fn oversized_file_rejected_before_conversion() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fake = set_fake_anydoc();
    set_max_anydoc_bytes(10);
    let p = tmp("big.pdf");
    std::fs::write(&p, b"x".repeat(11)).expect("write");
    let err = extract_anydoc(&p.to_string_lossy()).expect_err("too large");
    assert!(err.0.contains("too large"), "{}", err.0);
    // Oracle `_check_size` message: Python `f"{size:,}"` grouping.
    assert!(err.0.contains("(11 bytes, limit is 10)"), "{}", err.0);
    assert_eq!(fake.md_calls.load(Ordering::SeqCst), 0, "no conversion");
    let _ = std::fs::remove_file(&p);
    default_anydoc_state();
}

#[test]
fn file_at_limit_converts() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fake = set_fake_anydoc();
    set_max_anydoc_bytes(10);
    let p = tmp("ok.pdf");
    std::fs::write(&p, b"x".repeat(10)).expect("write");
    let text = extract_anydoc(&p.to_string_lossy()).expect("convert");
    assert_eq!(text, "converted\n");
    assert_eq!(fake.md_calls.load(Ordering::SeqCst), 1);
    let _ = std::fs::remove_file(&p);
    default_anydoc_state();
}

#[test]
fn missing_file_raises_extraction_error() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fake = set_fake_anydoc();
    let p = tmp("gone.pdf");
    assert!(extract_anydoc(&p.to_string_lossy()).is_err());
    assert_eq!(fake.md_calls.load(Ordering::SeqCst), 0, "size check first");
    default_anydoc_state();
}

// ---------------------------------------------------------------------------
// TestAnydocInitLifecycle
// ---------------------------------------------------------------------------

#[test]
fn successful_load_is_cached() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let loads = Arc::new(AtomicUsize::new(0));
    let counter = loads.clone();
    let fake = FakeAnydoc::new();
    let slot = fake.clone();
    set_anydoc_provider(Some(Box::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        Some(slot.clone() as _)
    })));
    let first = hermes_tools::read_extract::anydoc().expect("load");
    let second = hermes_tools::read_extract::anydoc().expect("cached");
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(loads.load(Ordering::SeqCst), 1, "second is a cache hit");
    default_anydoc_state();
}

#[test]
fn failed_load_retried_after_cooldown() {
    // ORACLE: `test_failed_load_is_retried_after_cooldown` (retry = 0).
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    set_anydoc_retry_seconds(0.0);
    let loads = Arc::new(AtomicUsize::new(0));
    let counter = loads.clone();
    let fake = FakeAnydoc::new();
    let slot = fake.clone();
    set_anydoc_provider(Some(Box::new(move || {
        if counter.fetch_add(1, Ordering::SeqCst) == 0 {
            None // first attempt fails (ImportError analog)
        } else {
            Some(slot.clone() as _)
        }
    })));
    assert!(hermes_tools::read_extract::anydoc().is_none());
    assert!(hermes_tools::read_extract::anydoc().is_some(), "retried");
    assert_eq!(loads.load(Ordering::SeqCst), 2);
    default_anydoc_state();
}

#[test]
fn failed_load_not_retried_within_cooldown() {
    // ORACLE: `test_failed_load_not_retried_within_cooldown` — one attempt
    // total; the handle stays Unset so a later retry remains possible.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    set_anydoc_retry_seconds(3600.0);
    let loads = Arc::new(AtomicUsize::new(0));
    let counter = loads.clone();
    set_anydoc_provider(Some(Box::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        None
    })));
    assert!(hermes_tools::read_extract::anydoc().is_none());
    assert!(hermes_tools::read_extract::anydoc().is_none());
    assert_eq!(loads.load(Ordering::SeqCst), 1, "no retry in cooldown");
    assert!(
        anydoc_cache_is_unset(),
        "failed handle stays UNSET (retry remains possible)"
    );
    default_anydoc_state();
}

#[test]
fn concurrent_first_load_imports_once() {
    // ORACLE: `test_concurrent_first_load_imports_once` — 4 racing first
    // users, one loader call, everyone gets the same handle.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let loads = Arc::new(AtomicUsize::new(0));
    let counter = loads.clone();
    let fake = FakeAnydoc::new();
    let slot = fake.clone();
    set_anydoc_provider(Some(Box::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        Some(slot.clone() as _)
    })));
    let barrier = Arc::new(std::sync::Barrier::new(4));
    let mut handles = Vec::new();
    for _ in 0..3 {
        let barrier = barrier.clone();
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            hermes_tools::read_extract::anydoc()
        }));
    }
    barrier.wait();
    let main = hermes_tools::read_extract::anydoc();
    let mut results = vec![main];
    for h in handles {
        results.push(h.join().unwrap());
    }
    assert_eq!(loads.load(Ordering::SeqCst), 1, "loader runs once");
    let first = results[0].as_ref().expect("loaded");
    for r in &results {
        assert!(Arc::ptr_eq(first, r.as_ref().unwrap()));
    }
    default_anydoc_state();
}

// ---------------------------------------------------------------------------
// TestNotebookExtraction
// ---------------------------------------------------------------------------

#[test]
fn markdown_and_code_in_order() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nb.ipynb");
    write_notebook(
        &p,
        serde_json::json!([
            {"cell_type": "markdown", "source": ["# Title\n", "para"]},
            {"cell_type": "code", "source": "x = 1\nprint(x)",
             "outputs": [{"output_type": "stream", "text": ["1\n"]}],
             "execution_count": 1}
        ]),
    );
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("# Title"));
    assert!(text.contains("print(x)"));
    assert!(!text.contains("output_type"));
    assert!(!text.contains("execution_count"));
    assert!(text.find("Title").unwrap() < text.find("print(x)").unwrap());
    let _ = std::fs::remove_file(&p);
}

#[test]
fn empty_cells_raises() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("empty.ipynb");
    std::fs::write(&p, serde_json::json!({"cells": []}).to_string()).expect("write");
    assert!(extract_document_text(&p.to_string_lossy()).is_err());
    let _ = std::fs::remove_file(&p);
}

#[test]
fn stream_output_rendered() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nb_out.ipynb");
    write_notebook(
        &p,
        serde_json::json!([{"cell_type": "code", "source": "print('epoch done')",
            "outputs": [{"output_type": "stream", "name": "stdout",
                         "text": ["epoch done\n", "loss=0.42\n"]}]}]),
    );
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("Output (cell 1)"));
    assert!(text.contains("loss=0.42"));
    let _ = std::fs::remove_file(&p);
}

#[test]
fn error_output_keeps_traceback_strips_ansi() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nb_err.ipynb");
    write_notebook(
        &p,
        serde_json::json!([{"cell_type": "code", "source": "1/0",
            "outputs": [{"output_type": "error", "ename": "ZeroDivisionError",
                         "evalue": "division by zero",
                         "traceback": ["\u{1b}[31mZeroDivisionError\u{1b}[0m: division by zero"]}]}]),
    );
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("Error: ZeroDivisionError: division by zero"));
    assert!(!text.contains('\u{1b}'), "ANSI stripped: {text:?}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn image_output_replaced_with_placeholder() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let payload = "A".repeat(4096); // ~3 KB decoded
    let p = tmp("nb_img.ipynb");
    write_notebook(
        &p,
        serde_json::json!([{"cell_type": "code", "source": "plot()",
            "outputs": [{"output_type": "display_data",
                         "data": {"image/png": payload.clone()}}]}]),
    );
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(
        text.contains("[image/png output — 3 KB, omitted]"),
        "{text}"
    );
    assert!(!text.contains(&payload), "payload must not leak");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn execute_result_prefers_text_plain_over_html() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nb_df.ipynb");
    write_notebook(
        &p,
        serde_json::json!([{"cell_type": "code", "source": "df.head()",
            "outputs": [{"output_type": "execute_result",
                         "data": {"text/html": "<table><tr><td>1</td></tr></table>",
                                  "text/plain": "   col\n0    1"}}]}]),
    );
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("   col"));
    assert!(!text.contains("<table>"));
    let _ = std::fs::remove_file(&p);
}

#[test]
fn carriage_return_progress_collapsed() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nb_tqdm.ipynb");
    write_notebook(
        &p,
        serde_json::json!([{"cell_type": "code", "source": "train()",
            "outputs": [{"output_type": "stream",
                         "text": [" 10%|\u{2588}\r 50%|\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\r100%|\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\n"]}]}]),
    );
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains(
        "100%|\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}"
    ));
    assert!(
        !text.contains("50%"),
        "only the final \\r frame survives: {text:?}"
    );
    let _ = std::fs::remove_file(&p);
}

#[test]
fn widget_output_placeholder() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nb_widget.ipynb");
    write_notebook(
        &p,
        serde_json::json!([{"cell_type": "code", "source": "slider",
            "outputs": [{"output_type": "display_data",
                         "data": {"application/vnd.jupyter.widget-view+json": {"model_id": "abc"},
                                  "text/plain": "IntSlider(value=0)"}}]}]),
    );
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("[interactive widget — omitted]"), "{text}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn oversized_outputs_truncated_with_jq_hint() {
    // ORACLE: `test_oversized_outputs_truncated` — 20k cap + jq hint naming
    // the v4 cell pointer.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nb_big.ipynb");
    write_notebook(
        &p,
        serde_json::json!([
            {"cell_type": "markdown", "source": "# intro"},
            {"cell_type": "code", "source": "spam()",
             "outputs": [{"output_type": "stream", "text": "x".repeat(25_000)}]}
        ]),
    );
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("output chars truncated"), "{text}");
    let name = p.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        text.contains(&format!("— full output: jq -r '.cells[1].outputs' {name}]")),
        "{text}"
    );
    assert!(text.chars().count() < 20_000 + 2_000, "bounded output");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn oversized_outputs_truncated_v3_jq_hint() {
    // ORACLE: `test_oversized_outputs_truncated_v3_jq_hint` — worksheets
    // pointer: `.worksheets[0].cells[1].outputs`.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nb_v3_big.ipynb");
    let nb = serde_json::json!({"worksheets": [{"cells": [
        {"cell_type": "markdown", "source": "# intro"},
        {"cell_type": "code", "source": "spam()",
         "outputs": [{"output_type": "stream", "text": "x".repeat(25_000)}]}
    ]}], "nbformat": 3});
    std::fs::write(&p, nb.to_string()).expect("write");
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("output chars truncated"));
    let name = p.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        text.contains(&format!(
            "— full output: jq -r '.worksheets[0].cells[1].outputs' {name}]"
        )),
        "{text}"
    );
    let _ = std::fs::remove_file(&p);
}

#[test]
fn legacy_v3_pyout_flat_fields() {
    // ORACLE: `test_legacy_v3_pyout_flat_fields` — v3 pyout with flat
    // `text` (no data dict) renders as a textual result.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nb_v3.ipynb");
    let nb = serde_json::json!({"worksheets": [{"cells": [
        {"cell_type": "code", "source": "1+1",
         "outputs": [{"output_type": "pyout", "text": ["2"]}]}
    ]}], "nbformat": 3});
    std::fs::write(&p, nb.to_string()).expect("write");
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("Output (cell 1)"), "{text}");
    assert!(text.contains("2"), "{text}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn malformed_outputs_ignored() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nb_bad_out.ipynb");
    write_notebook(
        &p,
        serde_json::json!([
            {"cell_type": "code", "source": "ok()",
             "outputs": ["not-a-dict", {"output_type": "bogus"}, null]},
            {"cell_type": "code", "source": "also_ok()", "outputs": "not-a-list"}
        ]),
    );
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("ok()"));
    assert!(text.contains("also_ok()"));
    assert!(!text.contains("Output (cell"), "{text}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn notebook_root_must_be_object() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nb_arr.ipynb");
    std::fs::write(&p, "[1,2,3]").expect("write");
    let err = extract_document_text(&p.to_string_lossy()).expect_err("err");
    assert!(err.0.contains("Notebook root is not an object"));
    let _ = std::fs::remove_file(&p);
}

#[test]
fn error_output_header_falls_back_gracefully() {
    // Oracle-probe: empty ename/evalue + non-list traceback — header
    // rstrip leaves "Error"; non-list traceback contributes no lines.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nb_err2.ipynb");
    write_notebook(
        &p,
        serde_json::json!([{"cell_type": "code", "source": "x",
            "outputs": [{"output_type": "error", "ename": "", "evalue": "",
                         "traceback": "not-a-list"}]}]),
    );
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    // header = "Error: : ".rstrip(": ") == "Error"; tb_text = "" → "Error"
    assert!(
        text.contains("Error\n") || text.trim_end().ends_with("Error"),
        "{text:?}"
    );
    let _ = std::fs::remove_file(&p);
}

// ---------------------------------------------------------------------------
// TestDocxExtraction / TestXlsxExtraction
// ---------------------------------------------------------------------------

fn doc(body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?><w:document xmlns:w="{NS_W}"><w:body>{body}</w:body></w:document>"#
    )
}

#[test]
fn paragraphs_and_runs() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
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
    let _ = std::fs::remove_file(&p);
}

#[test]
fn missing_document_xml_raises() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("nodoc.docx");
    let file = std::fs::File::create(&p).expect("create");
    let mut zf = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    zf.start_file("other.xml", opts).unwrap();
    zf.write_all(b"<x/>").unwrap();
    zf.finish().unwrap();
    assert!(extract_document_text(&p.to_string_lossy()).is_err());
    let _ = std::fs::remove_file(&p);
}

#[test]
fn stdlib_docx_path_still_authoritative() {
    // ORACLE (anydoc class, runnable without the package): .docx keeps the
    // stdlib extractor — same content either way.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = set_fake_anydoc(); // anydoc present must NOT take over .docx
    let p = tmp("auth.docx");
    write_docx(&p, &doc("<w:p><w:r><w:t>hello</w:t></w:r></w:p>"));
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert_eq!(text, "hello\n");
    let _ = std::fs::remove_file(&p);
    default_anydoc_state();
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
        &[
            ("xl/worksheets/sheet1.xml", &sheet1),
            ("xl/worksheets/sheet2.xml", &sheet2),
        ],
    );
}

#[test]
fn visible_sheet_content() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("wb.xlsx");
    build_xlsx(&p, true);
    let text = extract_document_text(&p.to_string_lossy()).expect("extract");
    assert!(text.contains("Data"), "sheet label");
    assert!(text.contains("Name\tScore"), "shared-string header row");
    assert!(text.contains("Alice\t95"), "string + numeric cells");
    assert!(!text.contains("SECRETDATA"), "hidden sheet omitted");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn not_a_zip_raises() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    let p = tmp("bad.xlsx");
    std::fs::write(&p, b"nope").expect("write");
    let err = extract_document_text(&p.to_string_lossy()).expect_err("err");
    assert!(err.0.contains("Not a valid XLSX"), "{}", err.0);
    let _ = std::fs::remove_file(&p);
}

// ---------------------------------------------------------------------------
// extract_document_bytes (upstream lines 117–126)
// ---------------------------------------------------------------------------

#[test]
fn extract_document_bytes_stdlib_and_dispatch() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    default_anydoc_state();
    // Unsupported extension raises before any temp copy.
    let err = extract_document_bytes(b"print(1)", "/x/script.py").expect_err("err");
    assert!(err.0.contains("Unsupported document type"), "{}", err.0);
    // Anydoc extension routes to the converter with the transferred bytes.
    let fake = set_fake_anydoc();
    let text = extract_document_bytes(b"{\\rtf1 fake}", "/workspace/remote.rtf").expect("bytes");
    assert_eq!(text, "converted\n");
    assert_eq!(fake.bytes_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        fake.md_calls.load(Ordering::SeqCst),
        0,
        "bytes path not path path"
    );
    default_anydoc_state();
}

// ---------------------------------------------------------------------------
// TestPdfCoverageNote — driven through the pure texts seam (upstream mocks
// _pdf_page_texts; the Rust analog feeds the note builder directly).
// ---------------------------------------------------------------------------

fn note_with_counts(counts: Option<&[usize]>) -> String {
    let owned: Option<Vec<String>> = counts.map(|c| c.iter().map(|n| "x".repeat(*n)).collect());
    let borrowed: Option<Vec<&str>> = owned
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect());
    coverage_note_from_texts(borrowed.as_deref(), None)
}

#[test]
fn mostly_scanned_pdf_warns_with_page_ranges() {
    // ORACLE: 3 text pages then 6 empty — well past the ratio.
    let note = note_with_counts(Some(&[900, 800, 700, 0, 0, 3, 0, 0, 0]));
    assert!(note.contains("EXTRACTION COVERAGE WARNING"), "{note}");
    assert!(note.contains("6 of 9 pages"), "{note}");
    assert!(note.contains("pages 4-9"), "{note}");
    assert!(note.contains("(6 pages)"), "{note}");
    assert!(note.contains("vision_analyze"), "{note}");
    assert!(note.contains("ocr-and-documents"), "{note}");
    assert!(note.contains("do NOT OCR or render everything"), "{note}");
}

#[test]
fn gap_labels_carry_preceding_section_text() {
    // ORACLE: `test_gap_labels_carry_preceding_section_text`.
    let texts = [
        "Section One: Bylaws of the Corporation",
        "",
        "",
        "",
        "",
        "",
        "Section Two: Budget details here",
        "",
        "",
        "",
        "",
    ];
    let note = coverage_note_from_texts(Some(&texts), None);
    assert!(
        note.contains(
            "pages 2-6 (5 pages) — after \"Section One: Bylaws of the Corporation\" (p1)"
        ),
        "{note}"
    );
    assert!(
        note.contains("pages 8-11 (4 pages) — after \"Section Two: Budget details here\" (p7)"),
        "{note}"
    );
}

#[test]
fn gap_map_caps_pathological_alternation() {
    // ORACLE: `test_gap_map_caps_pathological_alternation` — 60 single-page
    // gaps → 20 lines + one summary of the remaining 40 pages.
    let mut texts: Vec<String> = Vec::new();
    for i in 0..60 {
        texts.push(format!("Divider page number {i} with enough text"));
        texts.push(String::new());
    }
    let borrowed: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let note = coverage_note_from_texts(Some(&borrowed), None);
    let gap_lines: Vec<&str> = note.lines().filter(|ln| ln.starts_with("  ")).collect();
    assert_eq!(
        gap_lines.len(),
        PDF_GAP_MAP_MAX_ENTRIES + 1,
        "20 gaps + summary: {gap_lines:?}"
    );
    assert!(gap_lines.last().unwrap().contains("more gaps"));
    assert!(gap_lines.last().unwrap().contains("(40 pages)"));
}

#[test]
fn full_text_pdf_is_silent() {
    let counts = vec![500; 20];
    assert_eq!(note_with_counts(Some(&counts)), "");
}

#[test]
fn one_blank_page_is_tolerated() {
    assert_eq!(note_with_counts(Some(&[500, 0, 500, 500])), "");
}

#[test]
fn small_share_below_ratio_and_absolute_is_silent() {
    // 3 empty of 40 (7.5% < 20%, and < absolute threshold of 10).
    let mut counts = vec![400; 37];
    counts.extend([0, 0, 0]);
    assert_eq!(note_with_counts(Some(&counts)), "");
}

#[test]
fn large_absolute_count_warns_even_below_ratio() {
    // 12 empty of 100 (12% < 20% ratio) still warns.
    let mut counts = vec![400; 88];
    counts.extend([0; 12]);
    let note = note_with_counts(Some(&counts));
    assert!(note.contains("12 of 100 pages"), "{note}");
}

#[test]
fn undeterminable_counts_are_silent() {
    assert_eq!(note_with_counts(None), "");
    assert_eq!(note_with_counts(Some(&[0])), "");
}

// ---------------------------------------------------------------------------
// _pdf_page_texts — pdftotext lookup + form-feed parsing (upstream mocks
// shutil.which + subprocess.run; here a lookup override drives both paths).
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn fake_pdftotext(script_body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let p = tmp("fake_pdftotext");
    std::fs::write(&p, format!("#!/bin/sh\n{script_body}\n")).expect("write");
    let mut perm = std::fs::metadata(&p).unwrap().permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(&p, perm).unwrap();
    p
}

#[test]
fn page_texts_missing_pdftotext() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    set_pdftotext_override_for_test(Some(None));
    assert!(pdf_page_texts("/x/doc.pdf").is_none());
    set_pdftotext_override_for_test(None);
}

#[cfg(unix)]
#[test]
fn page_texts_parses_formfeeds() {
    // ORACLE: `test_page_texts_parses_formfeeds` — trailing empty segment
    // after the final \f drops; the real empty page between two \f's stays.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let script = fake_pdftotext("printf 'alpha beta\\014gamma\\014\\014'");
    set_pdftotext_override_for_test(Some(Some(script)));
    let pages = pdf_page_texts("/x/doc.pdf").expect("pages");
    assert_eq!(pages, ["alpha beta", "gamma", ""]);
    set_pdftotext_override_for_test(None);
}

// ---------------------------------------------------------------------------
// _extract_anydoc note-prepend + bytes display-path plumbing
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn extract_anydoc_prepends_note_for_pdf() {
    // ORACLE: `test_extract_anydoc_prepends_note_for_pdf` — the warning
    // leads the extracted text (a trailing footer may never be fetched).
    // The oracle mocks `_pdf_coverage_note`; here a fake pdftotext drives
    // the REAL note computation (1 text page + 4 empty → warn).
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let fake = set_fake_anydoc();
    let _ = fake;
    set_anydoc_provider(Some(Box::new({
        let fake = FakeAnydoc::with_text("# Title\n\nBody");
        move || Some(fake.clone() as _)
    })));
    let script = fake_pdftotext("printf 'text\\014\\014\\014\\014\\014'");
    set_pdftotext_override_for_test(Some(Some(script)));
    let p = tmp("note.pdf");
    std::fs::write(&p, b"%PDF-1.4 fake").expect("write");
    let text = extract_anydoc(&p.to_string_lossy()).expect("extract");
    assert!(text.starts_with("[EXTRACTION COVERAGE WARNING"), "{text}");
    assert!(text.contains("# Title"), "{text}");
    let _ = std::fs::remove_file(&p);
    set_pdftotext_override_for_test(None);
    default_anydoc_state();
}

#[cfg(unix)]
#[test]
fn extract_anydoc_no_note_for_non_pdf() {
    // ORACLE: `test_extract_anydoc_no_note_for_non_pdf` — behavior half of
    // the upstream `note.assert_not_called()` spy: non-pdf output is exactly
    // the converted text, no coverage prefix (the call itself is not
    // observable through the public surface — see PORT SEAMS).
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    set_anydoc_provider(None);
    reset_anydoc_cache();
    let fake = FakeAnydoc::with_text("converted");
    let slot = fake.clone();
    set_anydoc_provider(Some(Box::new(move || Some(slot.clone() as _))));
    let script = fake_pdftotext("printf 'text\\014\\014\\014\\014\\014'");
    set_pdftotext_override_for_test(Some(Some(script)));
    let p = tmp("note.rtf");
    std::fs::write(&p, b"{\\rtf1 fake}").expect("write");
    let text = extract_anydoc(&p.to_string_lossy()).expect("extract");
    assert_eq!(text, "converted\n", "no coverage prefix on non-pdf");
    let _ = std::fs::remove_file(&p);
    set_pdftotext_override_for_test(None);
    default_anydoc_state();
}

#[cfg(unix)]
#[test]
fn bytes_path_prepends_note_with_display_path() {
    // ORACLE: `test_bytes_path_prepends_note_with_display_path` — the
    // recovery command names the backend-visible path, not the host temp
    // file the scan ran against.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    set_anydoc_provider(None);
    reset_anydoc_cache();
    let fake = FakeAnydoc::with_text("# Title\n\nBody");
    let slot = fake.clone();
    set_anydoc_provider(Some(Box::new(move || Some(slot.clone() as _))));
    let script = fake_pdftotext("printf 'text\\014\\014\\014\\014\\014'");
    set_pdftotext_override_for_test(Some(Some(script)));
    let text = extract_anydoc_bytes(b"%PDF-1.4 fake", "/workspace/remote.pdf").expect("bytes");
    assert!(text.starts_with("[EXTRACTION COVERAGE WARNING"), "{text}");
    assert!(
        text.contains("/workspace/remote.pdf"),
        "display path (not the temp scan file): {text}"
    );
    assert!(text.contains("# Title"), "{text}");
    set_pdftotext_override_for_test(None);
    default_anydoc_state();
}

#[test]
fn bytes_path_no_note_for_non_pdf() {
    // ORACLE: `test_bytes_path_no_note_for_non_pdf`.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    set_anydoc_provider(None);
    reset_anydoc_cache();
    let fake = FakeAnydoc::with_text("converted");
    let slot = fake.clone();
    set_anydoc_provider(Some(Box::new(move || Some(slot.clone() as _))));
    let text = extract_anydoc_bytes(b"{\\rtf1 fake}", "/workspace/remote.rtf").expect("bytes");
    assert_eq!(text, "converted\n");
    default_anydoc_state();
}

#[test]
fn anydoc_empty_output_raises_no_extractable_text() {
    // ORACLE: `_finalize_anydoc_text` — blank/non-text conversion output.
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    set_anydoc_provider(None);
    reset_anydoc_cache();
    let fake = FakeAnydoc::with_text("   \n ");
    let slot = fake.clone();
    set_anydoc_provider(Some(Box::new(move || Some(slot.clone() as _))));
    let p = tmp("blank.rtf");
    std::fs::write(&p, b"x").expect("write");
    let err = extract_anydoc(&p.to_string_lossy()).expect_err("empty");
    assert!(
        err.0.contains("Document contains no extractable text"),
        "{}",
        err.0
    );
    let _ = std::fs::remove_file(&p);
    default_anydoc_state();
}

// ---------------------------------------------------------------------------
// Hosted-OCR config seam + needs-ocr warning (upstream lines 145–178)
// ---------------------------------------------------------------------------

#[test]
fn hosted_ocr_default_unavailable() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_hosted_ocr_for_test();
    assert!(!hosted_ocr_available(), "no key wired → disabled");
}

#[test]
fn hosted_ocr_tracks_key_and_config_gate() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset_hosted_ocr_for_test();
    wire_hosted_ocr(Box::new(|| Some("fc-key".to_string())), Box::new(|| false));
    assert!(hosted_ocr_available(), "key present + not disabled");
    wire_hosted_ocr(Box::new(|| Some("fc-key".to_string())), Box::new(|| true));
    assert!(
        !hosted_ocr_available(),
        "file_tools.hosted_ocr: false disables even with a key"
    );
    reset_hosted_ocr_for_test();
    assert!(!hosted_ocr_available());
}

#[test]
fn needs_ocr_warning_message_shape() {
    let msg = needs_ocr_warning("/x/f.pdf", &[1, 3], "KeyError: boom");
    assert!(msg.starts_with("[NEEDS OCR: pages 1, 3"), "{msg}");
    assert!(
        msg.contains("Hosted OCR was attempted and failed (KeyError: boom)"),
        "{msg}"
    );
    assert!(msg.contains("pdftoppm -jpeg -r 150"), "{msg}");
    assert!(msg.contains("vision_analyze"), "{msg}");
    assert!(msg.contains("skills_list"), "{msg}");
    assert!(msg.ends_with("]\n"), "{msg}");
    // No hosted error → that clause is omitted; empty pages → "unknown".
    let bare = needs_ocr_warning("/x/f.pdf", &[], "");
    assert!(bare.contains("pages unknown"), "{bare}");
    assert!(!bare.contains("Hosted OCR was attempted"), "{bare}");
}

// ---------------------------------------------------------------------------
// ExtractionError Display shape (message carries through read_file)
// ---------------------------------------------------------------------------

#[test]
fn extraction_error_display_roundtrip() {
    let e = ExtractionError("boom".to_string());
    assert_eq!(e.to_string(), "boom");
}
