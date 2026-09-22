//! Document-to-text extraction for `read_file`: stdlib Jupyter/DOCX/XLSX
//! (always authoritative for those three), plus legacy Office/OpenDocument/
//! RTF/EPUB/PDF when the optional converter backend is available.
//! Malformed documents return [`ExtractionError`]; callers fall back to
//! text/binary handling.
//!
//! PARITY: tools/read_extract.py @ 5d59366 (whole non-compat module).
//!
//! PORT SEAMS (documented divergences):
//! - The optional `anydoc` package is loaded through a provider slot
//!   ([`set_anydoc_provider`]) — the upstream `tools.lazy_deps.ensure` +
//!   `importlib.import_module` machinery has no Rust analog (see
//!   `fal_common::set_ensure_hook` for the crate's house pattern). The
//!   DEFAULT loader returns `None` (package unavailable — a real upstream
//!   state the oracles exercise), so the cooldown/cache state machine is
//!   live: failures stamp the cooldown, successes cache the handle, first
//!   load serializes. A future native/HTTP converter plugs into the slot.
//!   The upstream two-stage "ensure failed → import not called" split is
//!   absorbed into the single provider call (same net observable: one
//!   attempt, cooldown, no handle).
//! - `_pdf_page_texts` mocks `shutil.which` + `subprocess.run`; here a
//!   pdftotext lookup override ([`set_pdftotext_override_for_test`]) drives
//!   the missing-binary path, and a fake executable drives the form-feed
//!   parse through the REAL spawn+run path. The 20s timeout kills the
//!   child and returns None (upstream `SubprocessError` arm).
//! - Upstream `_pdf_coverage_note` is mocked wholesale by the oracle; the
//!   Rust note builder is split so tests feed synthetic page texts
//!   directly ([`coverage_note_from_texts`]) — same pure computation the
//!   wrapper drives from pdftotext. The upstream spy assertion
//!   ("note not called for non-pdf") is only observable as output equality
//!   here (a call counter would test implementation, not behavior).
//! - `hosted_ocr`'s `agent.secret_scope.get_secret` +
//!   `hermes_cli.config` upward imports cannot cross the crate layering
//!   (tools sits below agent/cli) — the key/config readers are wired via
//!   [`wire_hosted_ocr`] (agent/CLI layer at startup); unwired = no key =
//!   disabled, matching the upstream "no key" default.
//! - `_finalize_anydoc_text` checks `isinstance(text, str)` before
//!   strip; [`AnydocConverter`] methods return `String`, so the non-str arm
//!   is unreachable (blank/non-text output still raises the same error).
//! - Notebook invalid-UTF-8 bytes: upstream opens with
//!   `errors="replace"`; the port does a lossy string conversion before
//!   JSON parse (same tolerance).
//! - The v3 worksheet `cells` non-list shape makes upstream raise an
//!   uncaught `TypeError`; the port skips the worksheet and surfaces the
//!   normal "Notebook contains no cells"/"no readable cells" error
//!   (friendlier fail, untested upstream).
//! - Path reprs in error text use simple `'{}'` quoting (upstream `!r`
//!   switches to double quotes only when the path itself contains a
//!   single quote — not a tested shape).
//! - `_temp_copy` creation failures surface as [`ExtractionError`]; upstream
//!   lets the raw `OSError` escape (callers surface the message either way).
//! - The PLUGIN-COMPAT constant `MAX_XLSX_BYTES` (upstream lines 545–551)
//!   is not ported (in-tree compat pointers are off limits).
//! - TestReadFileToolIntegration / TestAnydocExtraction real-binding rows
//!   belong to other ledger rows or skip upstream without the package.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use crate::ansi_strip::strip_ansi;

pub const EXTRACTABLE_EXTENSIONS: [&str; 3] = [".ipynb", ".docx", ".xlsx"];
/// Upstream `ANYDOC_EXTENSIONS` (lines 30–32): 18 legacy Office / ODF /
/// RTF / EPUB / PDF formats gated on the optional converter.
pub const ANYDOC_EXTENSIONS: [&str; 18] = [
    ".doc", ".docm", ".ppt", ".pps", ".pot", ".pptx", ".pptm", ".ppsx", ".ppsm", ".xls", ".xlsm",
    ".xlsb", ".odt", ".ods", ".odp", ".rtf", ".epub", ".pdf",
];
/// Upstream `MAX_ANYDOC_BYTES` (line 34) — test-mutable like the oracle.
pub const MAX_ANYDOC_BYTES: u64 = 50 * 1024 * 1024;
/// Upstream `MAX_DOCUMENT_BYTES` (line 35).
pub const MAX_DOCUMENT_BYTES: u64 = 50 * 1024 * 1024;
/// Upstream `ANYDOC_RETRY_SECONDS` (line 60) — test-mutable.
pub const ANYDOC_RETRY_SECONDS: f64 = 300.0;
const MAX_XLSX_ROWS_PER_SHEET: usize = 5000;
const MAX_XLSX_COLS: usize = 256;

/// Upstream `PDF_EMPTY_PAGE_CHARS` (line 237).
pub const PDF_EMPTY_PAGE_CHARS: usize = 20;
/// Upstream `PDF_COVERAGE_MIN_EMPTY, PDF_COVERAGE_MIN_RATIO,
/// PDF_COVERAGE_ABSOLUTE_EMPTY` (line 239).
pub const PDF_COVERAGE_MIN_EMPTY: usize = 2;
pub const PDF_COVERAGE_MIN_RATIO: f64 = 0.2;
pub const PDF_COVERAGE_ABSOLUTE_EMPTY: usize = 10;
/// Upstream `PDF_PAGE_SCAN_TIMEOUT` (line 240).
pub const PDF_PAGE_SCAN_TIMEOUT: f64 = 20.0;
/// Upstream `PDF_GAP_MAP_MAX_ENTRIES` (line 241).
pub const PDF_GAP_MAP_MAX_ENTRIES: usize = 20;
const GAP_CONTEXT_CHARS: usize = 60;

/// Upstream `_MAX_OUTPUT_CHARS` (line 351) — per code cell.
pub const MAX_OUTPUT_CHARS: usize = 20_000;
/// Upstream `_V3_MIME_KEYS` (line 353).
const V3_MIME_KEYS: [(&str, &str); 4] = [
    ("png", "image/png"),
    ("jpeg", "image/jpeg"),
    ("svg", "image/svg+xml"),
    ("html", "text/html"),
];

const NS_W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const NS_S: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const NS_PKG_REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// Raised when a supported-looking document cannot be rendered as text.
#[derive(Debug)]
pub struct ExtractionError(pub String);

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ExtractionError {}

// ---------------------------------------------------------------------------
// Anydoc backend seam (upstream `_anydoc` lazy loader, lines 55–84)
// ---------------------------------------------------------------------------

/// Failure modes a converter can raise (mirrors upstream exception classes).
#[derive(Debug)]
pub enum AnydocError {
    /// `OSError` from the host (missing file, permission) — message verbatim.
    Io(String),
    /// `NeedsOcrError` (anydoc ≥ 0.2 scanned-pages signal) — carries the
    /// empty-page list plus the exception text for error formatting.
    NeedsOcr { pages: Vec<u64>, message: String },
    /// Any `ConvertError` subclass (Unsupported/Malformed/Encrypted/...) —
    /// `kind` is the Python exception type name.
    Convert { kind: String, message: String },
}

impl AnydocError {
    /// Python `type(exc).__name__`.
    pub fn type_name(&self) -> &str {
        match self {
            AnydocError::Io(_) => "OSError",
            AnydocError::NeedsOcr { .. } => "NeedsOcrError",
            AnydocError::Convert { kind, .. } => kind,
        }
    }
    /// Python `str(exc)` (message body after the type name).
    pub fn message(&self) -> &str {
        match self {
            AnydocError::Io(m) | AnydocError::NeedsOcr { message: m, .. } => m,
            AnydocError::Convert { message, .. } => message,
        }
    }
}

/// The optional converter backend (upstream `anydoc` module handle).
/// [`to_markdown_ocr_hosted`] mirrors `mod.to_markdown(path, ocr="hosted",
/// api_key=...)`; the default implementation reports no hosted route (the
/// production default never enables hosted OCR anyway — no key).
pub trait AnydocConverter: Send + Sync {
    fn to_markdown(&self, path: &Path) -> Result<String, AnydocError>;
    fn to_markdown_bytes(&self, data: &[u8]) -> Result<String, AnydocError>;
    fn to_markdown_ocr_hosted(&self, path: &Path, api_key: &str) -> Result<String, AnydocError> {
        let _ = (path, api_key);
        Err(AnydocError::Convert {
            kind: "HostedOcrUnsupported".to_string(),
            message: "converter has no hosted OCR route".to_string(),
        })
    }
}

type AnydocProvider = Box<dyn Fn() -> Option<Arc<dyn AnydocConverter>> + Send + Sync>;

/// Upstream's `_anydoc_module` global: slot for the load hook (None slot =
/// default loader = package unavailable, i.e. the ImportError path).
static ANYDOC_PROVIDER: Mutex<Option<AnydocProvider>> = Mutex::new(None);
/// Cached handle + `_anydoc_failed_at` cooldown stamp (`None` handle =
/// `_ANYDOC_UNSET`).
type AnydocState = (Option<Arc<dyn AnydocConverter>>, Option<Instant>);
static ANYDOC_STATE: Mutex<AnydocState> = Mutex::new((None, None));
static ANYDOC_RETRY_BITS: AtomicU64 = AtomicU64::new(ANYDOC_RETRY_SECONDS.to_bits());
static MAX_ANYDOC_BITS: AtomicU64 = AtomicU64::new(MAX_ANYDOC_BYTES);

/// Lazily import the optional converter (upstream `_anydoc`, lines 64–84):
/// cached handle short-circuits; failures stamp the retry cooldown without
/// caching a negative handle (so a later retry stays possible); successes
/// clear the stamp. The whole attempt serializes under one lock (upstream's
/// double-checked import lock).
pub fn anydoc() -> Option<Arc<dyn AnydocConverter>> {
    let mut state = ANYDOC_STATE.lock().expect("anydoc state");
    if let Some(handle) = &state.0 {
        return Some(handle.clone());
    }
    let retry = anydoc_retry_seconds().max(0.0);
    if let Some(failed_at) = state.1 {
        if failed_at.elapsed() < Duration::from_secs_f64(retry) {
            return None;
        }
    }
    let provider = ANYDOC_PROVIDER.lock().expect("anydoc provider");
    let loaded = match provider.as_ref() {
        Some(p) => p(),
        None => None, // default: no Rust import machinery → package unavailable
    };
    match loaded {
        Some(handle) => {
            state.0 = Some(handle.clone());
            state.1 = None;
            Some(handle)
        }
        None => {
            state.1 = Some(Instant::now());
            None
        }
    }
}

/// Install (or clear, with `None`) the converter load hook — test seam for
/// the oracle's `_anydoc_module` / `importlib.import_module` patches, and
/// the future production wiring point.
pub fn set_anydoc_provider(provider: Option<AnydocProvider>) {
    *ANYDOC_PROVIDER.lock().expect("anydoc provider") = provider;
}

/// Drop the cached handle + cooldown stamp (upstream test setUp resetting
/// `_anydoc_module = _ANYDOC_UNSET` / `_anydoc_failed_at = None`).
pub fn reset_anydoc_cache() {
    let mut state = ANYDOC_STATE.lock().expect("anydoc state");
    state.0 = None;
    state.1 = None;
}

/// Whether the cache sits in the never-successfully-loaded state (upstream
/// `_anydoc_module is _ANYDOC_UNSET` — asserted after a failed load).
pub fn anydoc_cache_is_unset() -> bool {
    let state = ANYDOC_STATE.lock().expect("anydoc state");
    state.0.is_none()
}

/// Upstream `ANYDOC_RETRY_SECONDS` (seconds).
pub fn anydoc_retry_seconds() -> f64 {
    f64::from_bits(ANYDOC_RETRY_BITS.load(Ordering::SeqCst))
}

/// Test/wiring setter for the retry cooldown (oracle mutates the module
/// global).
pub fn set_anydoc_retry_seconds(seconds: f64) {
    ANYDOC_RETRY_BITS.store(seconds.to_bits(), Ordering::SeqCst);
}

/// Upstream `MAX_ANYDOC_BYTES`.
pub fn max_anydoc_bytes() -> u64 {
    MAX_ANYDOC_BITS.load(Ordering::SeqCst)
}

/// Test setter for the size cap (oracle mutates the module global).
pub fn set_max_anydoc_bytes(bytes: u64) {
    MAX_ANYDOC_BITS.store(bytes, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
// Hosted-OCR config seam (upstream lines 145–164)
// ---------------------------------------------------------------------------

type HostedKeyProvider = Box<dyn Fn() -> Option<String> + Send + Sync>;
type HostedDisabledProvider = Box<dyn Fn() -> bool + Send + Sync>;
static HOSTED_OCR: Mutex<Option<(HostedKeyProvider, HostedDisabledProvider)>> = Mutex::new(None);

/// Wire the `FIRECRAWL_API_KEY` reader + `file_tools.hosted_ocr` gate —
/// called by the agent/CLI layer (upstream reads these through
/// `agent.secret_scope` + `hermes_cli.config`; see PORT SEAMS).
pub fn wire_hosted_ocr(
    api_key: Box<dyn Fn() -> Option<String> + Send + Sync>,
    hosted_ocr_disabled: Box<dyn Fn() -> bool + Send + Sync>,
) {
    *HOSTED_OCR.lock().expect("hosted ocr") = Some((api_key, hosted_ocr_disabled));
}

/// Drop any wired seams (tests; production never unwires).
pub fn reset_hosted_ocr_for_test() {
    *HOSTED_OCR.lock().expect("hosted ocr") = None;
}

/// `(enabled, api_key)` — upstream `_hosted_ocr_config` (lines 145–159):
/// enabled iff a key exists and the config gate does not disable it; the
/// config read is exception-suppressed (a panicking gate leaves `enabled`
/// as the key computed it).
fn hosted_ocr_config() -> (bool, Option<String>) {
    let guard = HOSTED_OCR.lock().expect("hosted ocr");
    let Some((key_provider, disabled_provider)) = guard.as_ref() else {
        return (false, None);
    };
    let api_key = key_provider();
    let mut enabled = api_key.is_some();
    let gate = std::panic::catch_unwind(std::panic::AssertUnwindSafe(disabled_provider));
    if let Ok(true) = gate {
        enabled = false;
    }
    (enabled, api_key)
}

/// Probe for read_file's schema line (upstream lines 162–164).
pub fn hosted_ocr_available() -> bool {
    hosted_ocr_config().0
}

// ---------------------------------------------------------------------------
// Dispatch + size checks (upstream lines 49–127)
// ---------------------------------------------------------------------------

/// Upstream `_extension` (lines 49–52): known stdlib extension, or a
/// legacy format exactly when the converter loads (availability tracking).
fn extension(path: &str) -> &'static str {
    let raw = Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let dotted = format!(".{raw}");
    if let Some(found) = EXTRACTABLE_EXTENSIONS.iter().find(|e| **e == dotted) {
        return found;
    }
    if ANYDOC_EXTENSIONS.contains(&dotted.as_str()) && anydoc().is_some() {
        if let Some(found) = ANYDOC_EXTENSIONS.iter().find(|e| **e == dotted) {
            return found;
        }
    }
    ""
}

/// Upstream `is_extractable_document` (lines 87–88).
pub fn is_extractable_document(path: &str) -> bool {
    !extension(path).is_empty()
}

/// Python `f"{n:,}"` thousands grouping.
fn commas(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let bytes = digits.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

/// Upstream `_check_size` (lines 91–94).
fn check_size(size: u64, limit: u64) -> Result<(), ExtractionError> {
    if size > limit {
        return Err(ExtractionError(format!(
            "Document too large to convert ({} bytes, limit is {})",
            commas(size),
            commas(limit)
        )));
    }
    Ok(())
}

fn path_is_pdf(path: &str) -> bool {
    Path::new(path)
        .extension()
        .map(|e| e.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

/// Upstream `extract_document_text` (lines 108–114): stdlib table wins,
/// then converter formats, else unsupported.
pub fn extract_document_text(path: &str) -> Result<String, ExtractionError> {
    let ext = extension(path);
    if EXTRACTABLE_EXTENSIONS.contains(&ext) {
        return match ext {
            ".ipynb" => extract_notebook(path),
            ".docx" => extract_docx(path),
            ".xlsx" => extract_xlsx(path),
            _ => unreachable!("EXTRACTABLE table"),
        };
    }
    if ANYDOC_EXTENSIONS.contains(&ext) {
        return extract_anydoc(path);
    }
    Err(ExtractionError(format!(
        "Unsupported document type: '{path}'"
    )))
}

/// Upstream `extract_document_bytes` (lines 117–126): size gate first,
/// converter formats take the transferred bytes directly, stdlib formats
/// run through a private temp copy (the stdlib extractors are
/// path-oriented).
pub fn extract_document_bytes(data: &[u8], path: &str) -> Result<String, ExtractionError> {
    check_size(data.len() as u64, MAX_DOCUMENT_BYTES)?;
    let ext = extension(path);
    if ANYDOC_EXTENSIONS.contains(&ext) {
        return extract_anydoc_bytes(data, path);
    }
    if !EXTRACTABLE_EXTENSIONS.contains(&ext) {
        return Err(ExtractionError(format!(
            "Unsupported document type: '{path}'"
        )));
    }
    let extractor: fn(&str) -> Result<String, ExtractionError> = match ext {
        ".ipynb" => extract_notebook,
        ".docx" => extract_docx,
        ".xlsx" => extract_xlsx,
        _ => unreachable!("EXTRACTABLE table"),
    };
    extract_via_temp_copy(data, ext, extractor)
}

/// `_temp_copy` (lines 96–105): materialize backend bytes in a private
/// host temp file; removed even when parsing fails (TempPath drop).
fn extract_via_temp_copy(
    data: &[u8],
    suffix: &str,
    extractor: fn(&str) -> Result<String, ExtractionError>,
) -> Result<String, ExtractionError> {
    let tmp = tempfile::Builder::new()
        .suffix(suffix)
        .tempfile()
        .map_err(|e| ExtractionError(e.to_string()))?;
    let mut tmp = tmp;
    tmp.write_all(data)
        .map_err(|e| ExtractionError(e.to_string()))?;
    tmp.flush().map_err(|e| ExtractionError(e.to_string()))?;
    let path = tmp.into_temp_path();
    let result = extractor(&path.to_string_lossy());
    drop(path); // unlink (upstream `finally: os.unlink`)
    result
}

// ---------------------------------------------------------------------------
// Anydoc-backed extraction (upstream lines 129–232)
// ---------------------------------------------------------------------------

fn anydoc_missing_error(path: &str) -> String {
    format!(
        "Cannot convert '{path}': this format needs the optional anydoc \
converter, which is not installed (install blocked or first \
attempt failed; retried every 5 minutes). Fix: `pip install \
firecrawl-anydoc` in Hermes's environment, or convert the file \
yourself via terminal (e.g. libreoffice --headless --convert-to \
txt)."
    )
}

/// Upstream `_needs_ocr_warning` (lines 167–178): NeedsOcrError result
/// when hosted OCR is off/failed; hints at CHECKING for an OCR skill
/// (never names one) and never advertises the hosted_ocr knob.
pub fn needs_ocr_warning(path: &str, pages: &[u64], hosted_error: &str) -> String {
    let page_list = if pages.is_empty() {
        "unknown".to_string()
    } else {
        pages
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let hosted = if hosted_error.is_empty() {
        String::new()
    } else {
        format!("Hosted OCR was attempted and failed ({hosted_error}). ")
    };
    format!(
        "[NEEDS OCR: pages {page_list} of this PDF are scanned images \
with no text layer — their content is MISSING below. {hosted}\
If the missing pages matter: render just those pages with \
`pdftoppm -jpeg -r 150 -f <first> -l <last> '{path}' /tmp/page` \
and inspect via vision_analyze, or check whether an OCR skill is \
available (skills_list).]\n"
    )
}

/// `_ocr_scanned_pdf` (lines 189–200): hosted OCR when a route exists,
/// else teach recovery.
fn ocr_scanned_pdf(conv: &dyn AnydocConverter, path: &str, pages: Vec<u64>) -> String {
    let (enabled, api_key) = hosted_ocr_config();
    let mut hosted_error = String::new();
    if enabled {
        match conv.to_markdown_ocr_hosted(Path::new(path), api_key.as_deref().unwrap_or("")) {
            Ok(text) => return format!("{}\n", text.trim_end_matches('\n')),
            Err(e) => hosted_error = format!("{}: {}", e.type_name(), e.message()),
        }
    }
    needs_ocr_warning(path, &pages, &hosted_error)
}

/// `_finalize_anydoc_text` (lines 181–186): blank output raises; PDFs get
/// the coverage note PREPENDED (read_file paginates, so a footer may never
/// be fetched); single trailing newline.
fn finalize_anydoc_text(
    text: String,
    path: &str,
    pdf_note: &str,
) -> Result<String, ExtractionError> {
    if text.trim().is_empty() {
        return Err(ExtractionError(
            "Document contains no extractable text".to_string(),
        ));
    }
    let body = format!("{}\n", text.trim_end_matches('\n'));
    let mut out = if path_is_pdf(path) {
        pdf_note.to_string()
    } else {
        String::new()
    };
    out.push_str(&body);
    Ok(out)
}

/// Upstream `_extract_anydoc` (lines 203–220): converter required; size
/// check then conversion inside the try; `OSError` arm; `NeedsOcrError`
/// arm; any `ConvertError` subclass collapses to `Type: message`.
pub fn extract_anydoc(path: &str) -> Result<String, ExtractionError> {
    let conv = anydoc().ok_or_else(|| ExtractionError(anydoc_missing_error(path)))?;
    let size = std::fs::metadata(path).map_err(|e| ExtractionError(e.to_string()))?;
    check_size(size.len(), max_anydoc_bytes())?;
    let text = match conv.to_markdown(Path::new(path)) {
        Ok(text) => text,
        Err(AnydocError::Io(msg)) => return Err(ExtractionError(msg)),
        Err(e @ AnydocError::NeedsOcr { .. }) => {
            let pages = match &e {
                AnydocError::NeedsOcr { pages, .. } => pages.clone(),
                _ => unreachable!("matched NeedsOcr"),
            };
            return Ok(ocr_scanned_pdf(&*conv, path, pages));
        }
        Err(AnydocError::Convert { kind, message }) => {
            return Err(ExtractionError(format!("{kind}: {message}")));
        }
    };
    let note = if path_is_pdf(path) {
        pdf_coverage_note(path, None)
    } else {
        String::new()
    };
    finalize_anydoc_text(text, path, &note)
}

/// Upstream `_extract_anydoc_bytes` (lines 223–232): NO OSError/NeedsOcr
/// special-casing — every converter failure collapses to `Type: message`.
pub fn extract_anydoc_bytes(data: &[u8], path: &str) -> Result<String, ExtractionError> {
    let conv = anydoc().ok_or_else(|| ExtractionError(anydoc_missing_error(path)))?;
    check_size(data.len() as u64, max_anydoc_bytes())?;
    let text = conv
        .to_markdown_bytes(data)
        .map_err(|e| ExtractionError(format!("{}: {}", e.type_name(), e.message())))?;
    let note = if path_is_pdf(path) {
        pdf_coverage_note_from_bytes(data, path)
    } else {
        String::new()
    };
    finalize_anydoc_text(text, path, &note)
}

// ---------------------------------------------------------------------------
// Scanned-PDF coverage (upstream lines 235–317)
// ---------------------------------------------------------------------------

/// Test override for the pdftotext lookup: `None` = normal PATH search,
/// `Some(None)` = force "binary missing", `Some(Some(p))` = force a
/// specific executable (upstream mocks `shutil.which` / `subprocess.run`).
static PDFTOTEXT_OVERRIDE: Mutex<Option<Option<PathBuf>>> = Mutex::new(None);

/// Test seam driving [`pdf_page_texts`] (see PORT SEAMS).
pub fn set_pdftotext_override_for_test(lookup: Option<Option<PathBuf>>) {
    *PDFTOTEXT_OVERRIDE.lock().expect("pdftotext override") = lookup;
}

/// `shutil.which("pdftotext")` analog (existence-only; a non-executable
/// hit fails at spawn → None, same net result).
fn pdftotext_lookup() -> Option<PathBuf> {
    if let Some(overridden) = PDFTOTEXT_OVERRIDE
        .lock()
        .expect("pdftotext override")
        .clone()
    {
        return overridden;
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("pdftotext"))
        .find(|cand| cand.is_file())
}

/// Parse `pdftotext <path> -` output: form-feed page split, trailing
/// form-feed artifact dropped, empty output → None (upstream lines 250–258).
pub fn parse_pdftotext_output(out: &[u8], success: bool) -> Option<Vec<String>> {
    let text = if success {
        String::from_utf8_lossy(out).into_owned()
    } else {
        String::new()
    };
    if text.is_empty() {
        return None;
    }
    let mut pages: Vec<String> = text.split('\u{c}').map(str::to_string).collect();
    if pages.last().map(|p| p.trim().is_empty()).unwrap_or(false) {
        pages.pop(); // trailing form-feed artifact
    }
    if pages.is_empty() {
        None
    } else {
        Some(pages)
    }
}

/// Per-page extracted text, or None when undeterminable (upstream lines
/// 245–258): missing binary → None; spawn failure or non-zero exit → None
/// (20s timeout kills the child).
pub fn pdf_page_texts(path: &str) -> Option<Vec<String>> {
    let cmd = pdftotext_lookup()?;
    let mut child = std::process::Command::new(&cmd)
        .arg(path)
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let deadline = Instant::now() + Duration::from_secs_f64(PDF_PAGE_SCAN_TIMEOUT);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = reader.join();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => {
                let _ = reader.join();
                return None;
            }
        }
    };
    let out = reader.join().ok()?;
    parse_pdftotext_output(&out, status.success())
}

/// Python `" ".join(text.split())` — whitespace-normalize a context line.
fn normalize_ws(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `_gap_map` (lines 261–280): per-gap breakdown labeled with the text
/// before each gap; caps at [`PDF_GAP_MAP_MAX_ENTRIES`] ranges.
fn gap_map(counts: &[usize], texts: &[&str], empty: &[usize]) -> String {
    // Sorted 1-based page numbers → (start, end) runs of consecutive pages.
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for &page in empty {
        if let Some(last) = ranges.last_mut() {
            if last.1 + 1 == page {
                last.1 = page;
                continue;
            }
        }
        ranges.push((page, page));
    }
    let mut lines: Vec<String> = Vec::new();
    for &(a, b) in ranges.iter().take(PDF_GAP_MAP_MAX_ENTRIES) {
        let mut label = String::new();
        // Nearest preceding page with text (`range(a - 2, -1, -1)`).
        for prev in (0..a.saturating_sub(1)).rev() {
            if counts[prev] >= PDF_EMPTY_PAGE_CHARS {
                let snippet: String = normalize_ws(texts[prev])
                    .chars()
                    .take(GAP_CONTEXT_CHARS)
                    .collect();
                label = format!(" — after \"{snippet}\" (p{})", prev + 1);
                break;
            }
        }
        let span = if a == b {
            format!("page {a}")
        } else {
            format!("pages {a}-{b}")
        };
        let n = b - a + 1;
        let plural = if n != 1 { "s" } else { "" };
        lines.push(format!("  {span} ({n} page{plural}){label}"));
    }
    if ranges.len() > PDF_GAP_MAP_MAX_ENTRIES {
        let rest = &ranges[PDF_GAP_MAP_MAX_ENTRIES..];
        let rest_pages: usize = rest.iter().map(|(a, b)| b - a + 1).sum();
        lines.push(format!(
            "  … {} more gaps ({} pages)",
            rest.len(),
            rest_pages
        ));
    }
    lines.join("\n")
}

/// Pure note computation over synthetic page texts (upstream mocks
/// `_pdf_page_texts` and drives `_pdf_coverage_note`; see PORT SEAMS).
/// `display_path` names the recovery command (the wrapper passes the scan
/// path, the bytes path passes the backend-visible path).
pub fn coverage_note_from_texts(texts: Option<&[&str]>, display_path: Option<&str>) -> String {
    let Some(texts) = texts else {
        return String::new();
    };
    if texts.len() < 2 {
        return String::new();
    }
    let counts: Vec<usize> = texts.iter().map(|p| p.trim().chars().count()).collect();
    let empty: Vec<usize> = counts
        .iter()
        .enumerate()
        .filter(|(_, n)| **n < PDF_EMPTY_PAGE_CHARS)
        .map(|(i, _)| i + 1)
        .collect();
    let total = counts.len();
    let n_empty = empty.len();
    let enough = (n_empty as f64 / total as f64) >= PDF_COVERAGE_MIN_RATIO
        || n_empty >= PDF_COVERAGE_ABSOLUTE_EMPTY;
    if n_empty < PDF_COVERAGE_MIN_EMPTY || !enough {
        return String::new();
    }
    let shown = display_path.unwrap_or("");
    format!(
        "[EXTRACTION COVERAGE WARNING: {n_empty} of {total} pages in this PDF yielded no text. \
Those pages are likely scanned images (or blank) — their content \
is MISSING from the extracted text below, even where section \
headers appear with empty bodies. Unreadable gaps, each labeled \
with the last text extracted before it:\n{}\n\
Decide which gaps you actually need — do NOT OCR or render \
everything. For the gaps that matter, render just that range with \
`pdftoppm -jpeg -r 150 -f <first> -l <last> '{shown}' /tmp/page` \
and inspect each image with the vision_analyze tool, or use the \
ocr-and-documents skill (marker-pdf) for bulk OCR of large \
ranges.]\n",
        gap_map(&counts, texts, &empty)
    )
}

/// `_pdf_coverage_note` (lines 283–310): warning header when many pages
/// yielded no text, else ''.
pub fn pdf_coverage_note(path: &str, display_path: Option<&str>) -> String {
    let texts = pdf_page_texts(path);
    let borrowed: Option<Vec<&str>> = texts
        .as_ref()
        .map(|v| v.iter().map(String::as_str).collect());
    coverage_note_from_texts(borrowed.as_deref(), Some(display_path.unwrap_or(path)))
}

/// `_pdf_coverage_note_from_bytes` (lines 313–317): coverage note for
/// backend PDF bytes via a host temp copy (pdftotext needs a path);
/// creation failures suppress to '' (upstream `suppress(OSError)`).
pub fn pdf_coverage_note_from_bytes(data: &[u8], display_path: &str) -> String {
    match tempfile::Builder::new()
        .suffix(".pdf")
        .tempfile()
        .map_err(|e| e.to_string())
    {
        Ok(mut tmp) => {
            let ok = tmp.write_all(data).is_ok() && tmp.flush().is_ok();
            if ok {
                let path = tmp.into_temp_path();
                let note = pdf_coverage_note(&path.to_string_lossy(), Some(display_path));
                drop(path); // unlink (suppressible in upstream)
                note
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Stdlib extractors (upstream lines 320–542)
// ---------------------------------------------------------------------------

/// `_joined` (lines 320–324): join extracted lines with a single trailing
/// newline; raise when nothing non-blank.
fn joined(lines: &[String], empty_error: &str) -> Result<String, ExtractionError> {
    if !lines.iter().any(|l| !l.trim().is_empty()) {
        return Err(ExtractionError(empty_error.to_string()));
    }
    Ok(format!("{}\n", lines.join("\n").trim_end_matches('\n')))
}

/// `_source_text` (lines 327–331): notebook source/text fields are a str
/// or a list of str fragments.
fn source_text(source: &Value) -> String {
    match source {
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

/// Python `round()` — half-to-even for the `.5` ties (upstream
/// `_human_size`'s `round(n / 1024)`).
fn py_round_half_even(v: f64) -> i64 {
    let floor = v.floor();
    let frac = v - floor;
    let f = floor as i64;
    if frac > 0.5 {
        f + 1
    } else if frac < 0.5 || (frac == 0.5 && f % 2 == 0) {
        f
    } else {
        // Exact .5 tie on an odd floor — round up to even.
        f + 1
    }
}

/// `_human_size` (lines 334–335).
fn human_size(n_bytes: u64) -> String {
    if n_bytes >= 1024 {
        format!("{} KB", py_round_half_even(n_bytes as f64 / 1024.0))
    } else {
        format!("{n_bytes} B")
    }
}

/// `_base64_bytes` (lines 338–341): approximate decoded size of a base64
/// payload (whitespace ignored; ≤2 padding chars subtracted).
fn base64_bytes(payload: &str) -> u64 {
    let clean: String = payload
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '='))
        .collect();
    let pad = clean.chars().rev().take_while(|c| *c == '=').count().min(2);
    let len = clean.len() as u64;
    ((len * 3) / 4).saturating_sub(pad as u64)
}

/// `_clean_stream_text` (lines 344–348): strip ANSI escapes; keep only the
/// final `\r` frame of each line (tqdm redraws).
fn clean_stream_text(text: &str) -> String {
    let stripped = strip_ansi(text);
    let normalized = stripped.replace("\r\n", "\n");
    normalized
        .split('\n')
        .map(|line| {
            let frames: Vec<&str> = line.split('\r').filter(|f| !f.is_empty()).collect();
            frames.last().copied().unwrap_or("")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `_notebook_output_text` (lines 356–388): one notebook output as
/// compact text — stream/traceback/textual results kept; token-heavy
/// payloads (images, HTML, widgets) become sized placeholders. Handles v4
/// and legacy v3 shapes.
fn notebook_output_text(output: &Value) -> String {
    let Some(o) = output.as_object() else {
        return String::new();
    };
    let Some(otype) = o.get("output_type").and_then(Value::as_str) else {
        return String::new();
    };
    match otype {
        "stream" => {
            let body = clean_stream_text(&source_text(o.get("text").unwrap_or(&Value::Null)));
            if body.trim().is_empty() {
                String::new()
            } else {
                body
            }
        }
        "error" | "pyerr" => {
            let tb_text = match o.get("traceback") {
                Some(Value::Array(items)) => {
                    let joined = items
                        .iter()
                        .filter_map(|l| l.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    clean_stream_text(&joined)
                }
                _ => clean_stream_text(""),
            };
            let ename = o.get("ename").and_then(Value::as_str).unwrap_or("");
            let evalue = o.get("evalue").and_then(Value::as_str).unwrap_or("");
            let header = format!("Error: {ename}: {evalue}");
            // Python `.rstrip(": ")` — strip trailing ':' and ' ' repeatedly.
            let header = header.trim_end_matches([':', ' ']);
            format!("{header}\n{tb_text}").trim_end().to_string()
        }
        "execute_result" | "display_data" | "pyout" => {
            let data: Map<String, Value> = match o.get("data") {
                Some(Value::Object(m)) => m.clone(),
                _ => {
                    // Legacy v3: mime payloads sit flat on the output dict.
                    let mut m = Map::new();
                    if let Some(t) = o.get("text") {
                        if t.is_string() || t.is_array() {
                            m.insert("text/plain".to_string(), t.clone());
                        }
                    }
                    for (k, mime) in V3_MIME_KEYS {
                        if let Some(v) = o.get(k) {
                            m.insert(mime.to_string(), v.clone());
                        }
                    }
                    m
                }
            };
            if data.contains_key("application/vnd.jupyter.widget-view+json") {
                return "[interactive widget — omitted]".to_string();
            }
            for mime in ["text/plain", "text/markdown"] {
                if let Some(value) = data.get(mime) {
                    let body = clean_stream_text(&source_text(value));
                    if !body.trim().is_empty() {
                        return body;
                    }
                }
            }
            for (mime, value) in &data {
                if mime.starts_with("image/") {
                    return format!(
                        "[{mime} output — {}, omitted]",
                        human_size(base64_bytes(&source_text(value)))
                    );
                }
            }
            if let Some(html) = data.get("text/html") {
                return format!(
                    "[text/html output — {} chars, omitted]",
                    commas(source_text(html).chars().count() as u64)
                );
            }
            let keys: Vec<String> = data.keys().cloned().collect();
            let joined_keys = if keys.is_empty() {
                "unknown".to_string()
            } else {
                keys.join(", ")
            };
            format!("[{joined_keys} output — omitted]")
        }
        _ => String::new(),
    }
}

/// `_notebook_outputs` (lines 391–400): joined non-empty outputs; past the
/// per-cell char budget, truncate with the jq-pointer hint.
fn notebook_outputs(cell: &Value, jq_pointer: &str, filename: &str) -> String {
    let Some(outputs) = cell.get("outputs").and_then(Value::as_array) else {
        return String::new();
    };
    let texts: Vec<String> = outputs
        .iter()
        .map(notebook_output_text)
        .filter(|t| !t.is_empty())
        .collect();
    let joined = texts.join("\n");
    let len = joined.chars().count();
    if len <= MAX_OUTPUT_CHARS {
        return joined;
    }
    let hint = if !jq_pointer.is_empty() && !filename.is_empty() {
        format!(" — full output: jq -r '{jq_pointer}' {filename}")
    } else {
        String::new()
    };
    let omitted = len - MAX_OUTPUT_CHARS;
    let head: String = joined.chars().take(MAX_OUTPUT_CHARS).collect();
    format!(
        "{head}\n… [{} output chars truncated{hint}]",
        commas(omitted as u64)
    )
}

const CELL_LABELS: [(&str, &str); 3] = [("markdown", "Markdown"), ("code", "Code"), ("raw", "Raw")];

/// `_extract_notebook` (lines 406–438): v4 cells list or v3 worksheets,
/// each cell labeled + sources + code outputs (jq pointers preserved for
/// the truncation hint).
fn extract_notebook(path: &str) -> Result<String, ExtractionError> {
    let bytes =
        std::fs::read(path).map_err(|e| ExtractionError(format!("Not a valid notebook: {e}")))?;
    // Upstream opens with `errors="replace"` — parse a lossy view.
    let lossy = String::from_utf8_lossy(&bytes);
    let nb: Value = serde_json::from_str(&lossy)
        .map_err(|e| ExtractionError(format!("Not a valid notebook: {e}")))?;
    let Some(nb_obj) = nb.as_object() else {
        return Err(ExtractionError(
            "Notebook root is not an object".to_string(),
        ));
    };
    let mut cells: Vec<(String, Value)> = Vec::new();
    match nb_obj.get("cells").and_then(Value::as_array) {
        Some(raw_cells) => {
            for (i, cell) in raw_cells.iter().enumerate() {
                cells.push((format!(".cells[{i}].outputs"), cell.clone()));
            }
        }
        None => {
            // nbformat v3: cells live under worksheets (dict worksheets only).
            if let Some(worksheets) = nb_obj.get("worksheets").and_then(Value::as_array) {
                for (wi, ws) in worksheets.iter().enumerate() {
                    let Some(ws_obj) = ws.as_object() else {
                        continue;
                    };
                    // Non-list `cells` (e.g. null): upstream raises an
                    // uncaught TypeError; the port skips (PORT SEAMS).
                    if let Some(raw) = ws_obj.get("cells").and_then(Value::as_array) {
                        for (ci, cell) in raw.iter().enumerate() {
                            cells.push((
                                format!(".worksheets[{wi}].cells[{ci}].outputs"),
                                cell.clone(),
                            ));
                        }
                    }
                }
            }
        }
    }
    if cells.is_empty() {
        return Err(ExtractionError("Notebook contains no cells".to_string()));
    }
    let nb_name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut out: Vec<String> = Vec::new();
    for (jq_pointer, cell) in &cells {
        let Some(obj) = cell.as_object() else {
            continue;
        };
        let Some(typ) = obj.get("cell_type").and_then(Value::as_str) else {
            continue;
        };
        let Some((_, label)) = CELL_LABELS.iter().find(|(t, _)| *t == typ) else {
            continue;
        };
        let n = {
            let counter = counts.entry(typ).or_insert(0);
            *counter += 1;
            *counter
        };
        let suffix = if typ == "raw" {
            String::new()
        } else {
            format!(" {n}")
        };
        let src = source_text(obj.get("source").unwrap_or(&Value::Null));
        out.push(format!("# ── {label} cell{suffix} ──"));
        out.push(src.trim_end_matches('\n').to_string());
        out.push(String::new());
        let rendered = if typ == "code" {
            notebook_outputs(cell, jq_pointer, &nb_name)
        } else {
            String::new()
        };
        if !rendered.is_empty() {
            out.push(format!("# ── Output (cell {n}) ──"));
            out.push(rendered.trim_end_matches('\n').to_string());
            out.push(String::new());
        }
    }
    joined(&out, "Notebook contains no readable cells")
}

// ── OOXML plumbing (upstream `_open_zip` / `_zip_xml`, lines 441–461) ──

type Zip = zip::ZipArchive<std::io::BufReader<std::fs::File>>;

/// `_open_zip`: bad-zip / OS failures become `ExtractionError` with the
/// package kind (upstream BadZipFile → "Not a valid {kind}").
fn open_zip(path: &str, kind: &str) -> Result<Zip, ExtractionError> {
    let file = std::fs::File::open(path).map_err(|e| ExtractionError(e.to_string()))?;
    zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|e| ExtractionError(format!("Not a valid {kind}: {e}")))
}

/// Required package part: missing entry → "Missing {name}"; unreadable
/// (CRC/IO) → "Not a valid {kind}" (upstream routes those through
/// `_open_zip`'s BadZipFile/OSError arm).
fn zip_part(zf: &mut Zip, name: &str, kind: &str) -> Result<Vec<u8>, ExtractionError> {
    let mut entry = match zf.by_name(name) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => {
            return Err(ExtractionError(format!("Missing {name}")))
        }
        Err(e) => return Err(ExtractionError(format!("Not a valid {kind}: {e}"))),
    };
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|e| ExtractionError(format!("Not a valid {kind}: {e}")))?;
    Ok(bytes)
}

/// Optional part (upstream `optional=True`): absent or malformed → None
/// (callers treat as empty; upstream yields `ET.Element("missing")`).
fn zip_part_optional(zf: &mut Zip, name: &str) -> Option<Vec<u8>> {
    let mut entry = zf.by_name(name).ok()?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).ok()?;
    Some(bytes)
}

/// `_extract_docx` (lines 464–474): paragraphs walk with namespace-qualified
/// tags (w:p / w:t / w:tab / w:br / w:cr).
fn extract_docx(path: &str) -> Result<String, ExtractionError> {
    let mut zf = open_zip(path, "DOCX")?;
    let xml = zip_part(&mut zf, "word/document.xml", "DOCX")?;
    let xml_text = String::from_utf8_lossy(&xml).into_owned();
    let doc = roxmltree::Document::parse(&xml_text)
        .map_err(|e| ExtractionError(format!("Malformed XML in word/document.xml: {e}")))?;

    let mut lines: Vec<String> = Vec::new();
    for para in doc.descendants().filter(|n| {
        n.is_element() && n.tag_name().namespace() == Some(NS_W) && n.tag_name().name() == "p"
    }) {
        let mut buf = String::new();
        for node in para.descendants() {
            if !node.is_element() || node.tag_name().namespace() != Some(NS_W) {
                continue;
            }
            match node.tag_name().name() {
                "t" => buf.push_str(node.text().unwrap_or("")),
                "tab" => buf.push('\t'),
                "br" | "cr" => buf.push('\n'),
                _ => {}
            }
        }
        lines.extend(buf.split('\n').map(str::to_string));
    }
    joined(&lines, "DOCX contains no extractable text")
}

/// `_extract_xlsx` (lines 477–496): visible sheets only; shared strings +
/// rels are optional parts; hidden/veryHidden sheets skipped.
fn extract_xlsx(path: &str) -> Result<String, ExtractionError> {
    let mut zf = open_zip(path, "XLSX")?;
    let names: std::collections::HashSet<String> = zf.file_names().map(|s| s.to_string()).collect();
    let shared = shared_strings(&mut zf, &names);
    let rels = workbook_rels(&mut zf, &names);
    let wb_xml = zip_part(&mut zf, "xl/workbook.xml", "XLSX")?;
    let wb_text = String::from_utf8_lossy(&wb_xml).into_owned();
    let wb = roxmltree::Document::parse(&wb_text)
        .map_err(|e| ExtractionError(format!("Malformed XML in xl/workbook.xml: {e}")))?;

    let mut out: Vec<String> = Vec::new();
    for sheet in wb.descendants().filter(|n| {
        n.is_element() && n.tag_name().namespace() == Some(NS_S) && n.tag_name().name() == "sheet"
    }) {
        let state = sheet.attribute("state").unwrap_or("visible");
        if state == "hidden" || state == "veryHidden" {
            continue;
        }
        let rid = sheet.attribute((NS_R, "id")).unwrap_or("");
        let target = rels.get(rid).map(String::as_str).unwrap_or("");
        let target = target.trim_start_matches('/');
        let full = if target.starts_with("xl/") {
            target.to_string()
        } else if target.is_empty() {
            String::new()
        } else {
            format!("xl/{target}")
        };
        let part = posix_normpath(&full);
        if !names.contains(&part) {
            continue;
        }
        let sheet_name = sheet.attribute("name").unwrap_or("Sheet");
        // Upstream suppresses per-sheet parse errors.
        let Some(xml) = zip_part_optional(&mut zf, &part) else {
            continue;
        };
        let Ok(text) = String::from_utf8(xml) else {
            continue;
        };
        let Ok(rows) = sheet_rows(&text, &shared) else {
            continue;
        };
        out.push(format!("# ── Sheet: {sheet_name} ──"));
        if rows.is_empty() {
            out.push("(empty)".to_string());
        } else {
            for row in &rows {
                out.push(row.join("\t"));
            }
        }
        out.push(String::new());
    }
    joined(&out, "XLSX has no visible sheets with content")
}

fn shared_strings(zf: &mut Zip, names: &std::collections::HashSet<String>) -> Vec<String> {
    if !names.contains("xl/sharedStrings.xml") {
        return Vec::new();
    }
    let Some(xml) = zip_part_optional(zf, "xl/sharedStrings.xml") else {
        return Vec::new();
    };
    let Ok(text) = String::from_utf8(xml) else {
        return Vec::new();
    };
    let Ok(doc) = roxmltree::Document::parse(&text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for si in doc.descendants().filter(|n| {
        n.is_element() && n.tag_name().namespace() == Some(NS_S) && n.tag_name().name() == "si"
    }) {
        let text: String = si
            .descendants()
            .filter(|n| {
                n.is_element()
                    && n.tag_name().namespace() == Some(NS_S)
                    && n.tag_name().name() == "t"
            })
            .filter_map(|t| t.text())
            .collect();
        out.push(text);
    }
    out
}

fn workbook_rels(
    zf: &mut Zip,
    names: &std::collections::HashSet<String>,
) -> HashMap<String, String> {
    if !names.contains("xl/_rels/workbook.xml.rels") {
        return HashMap::new();
    }
    let Some(xml) = zip_part_optional(zf, "xl/_rels/workbook.xml.rels") else {
        return HashMap::new();
    };
    let Ok(text) = String::from_utf8(xml) else {
        return HashMap::new();
    };
    let Ok(doc) = roxmltree::Document::parse(&text) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for rel in doc.descendants().filter(|n| {
        n.is_element()
            && n.tag_name().namespace() == Some(NS_PKG_REL)
            && n.tag_name().name() == "Relationship"
    }) {
        if let Some(id) = rel.attribute("Id") {
            out.insert(
                id.to_string(),
                rel.attribute("Target").unwrap_or("").to_string(),
            );
        }
    }
    out
}

/// POSIX path normalization for zip part names (upstream
/// `posixpath.normpath` of the rel target).
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

/// 0-based column of a cell ref: `A1` → 0, `AB7` → 27 (bijective
/// base-26 letters; upstream `_col_index`, lines 499–503).
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

/// `_sheet_rows` (lines 506–521): row/col caps, sparse-cell fill, trailing
/// blank rows popped.
fn sheet_rows(xml: &str, shared: &[String]) -> Result<Vec<Vec<String>>, ExtractionError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| ExtractionError(format!("Malformed XML in sheet: {e}")))?;
    let mut rows: Vec<Vec<String>> = Vec::new();
    for row in doc.descendants().filter(|n| {
        n.is_element() && n.tag_name().namespace() == Some(NS_S) && n.tag_name().name() == "row"
    }) {
        if rows.len() >= MAX_XLSX_ROWS_PER_SHEET {
            break;
        }
        let mut cells: std::collections::BTreeMap<usize, String> =
            std::collections::BTreeMap::new();
        let mut max_col: isize = -1;
        for cell in row.descendants().filter(|n| {
            n.is_element() && n.tag_name().namespace() == Some(NS_S) && n.tag_name().name() == "c"
        }) {
            let col = match cell.attribute("r") {
                Some(r) if !r.is_empty() => col_index(r),
                _ => (max_col + 1).max(0) as usize,
            };
            if col < MAX_XLSX_COLS {
                cells.insert(col, cell_value(cell, shared));
                max_col = max_col.max(col as isize);
            }
        }
        if max_col >= 0 {
            rows.push(
                (0..=max_col as usize)
                    .map(|i| cells.get(&i).cloned().unwrap_or_default())
                    .collect(),
            );
        } else {
            rows.push(Vec::new());
        }
    }
    while rows
        .last()
        .map(|r| r.iter().all(|v| v.trim().is_empty()))
        .unwrap_or(false)
    {
        rows.pop();
    }
    Ok(rows)
}

/// `_cell_value` (lines 524–537): shared-string / inline / bool / error
/// cell decoding.
fn cell_value(cell: roxmltree::Node, shared: &[String]) -> String {
    let value = cell
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "v")
        .and_then(|v| v.text())
        .unwrap_or("")
        .to_string();
    let typ = cell.attribute("t").unwrap_or("");
    match typ {
        "s" => value
            .parse::<usize>()
            .ok()
            .and_then(|i| shared.get(i).cloned())
            .unwrap_or_default(),
        "inlineStr" => cell
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "is")
            .map(|is_node| {
                is_node
                    .descendants()
                    .filter(|n| n.is_element() && n.tag_name().name() == "t")
                    .filter_map(|t| t.text())
                    .collect::<String>()
            })
            .unwrap_or_default(),
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
        assert_eq!(
            posix_normpath("xl/worksheets/sheet1.xml"),
            "xl/worksheets/sheet1.xml"
        );
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
