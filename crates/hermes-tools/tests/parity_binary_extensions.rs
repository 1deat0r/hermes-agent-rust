//! Parity tests for `tools/binary_extensions.py` @ 5d59366.
//!
//! Oracle: source-as-oracle (no dedicated test file — gap noted). Pins the
//! `_has_extension_in` shape (case-insensitive final-suffix, pure string),
//! both extension sets, and the PDF carve-out. The `has_` → `is_` rename is
//! a pre-existing port divergence (kept); both spellings are tested via the
//! canonical `is_` names.

use hermes_tools::binary_extensions::{
    is_binary_extension, is_opaque_document_extension, is_pdf_path,
};

/// Case-insensitive final-suffix matching; no-dot → false.
#[test]
fn binary_matching_shape() {
    assert!(is_binary_extension("photo.PNG"));
    assert!(is_binary_extension("archive.tar.gz"));
    assert!(!is_binary_extension("notes.txt"));
    assert!(!is_binary_extension("noextension"));
    assert!(!is_binary_extension(".pdf"));
}

/// Opaque container documents (live set @ 5d59366, incl. macro/slideshow
/// variants absent from BINARY_EXTENSIONS).
#[test]
fn opaque_document_set() {
    for ext in [".docm", ".xlsm", ".xlsb", ".pps", ".pot", ".pptm", ".ppsx", ".ppsm", ".rtf", ".epub"] {
        assert!(is_opaque_document_extension(&format!("file{ext}")), "{ext}");
    }
    // Shared members live in both sets.
    assert!(is_opaque_document_extension("a.docx"));
    assert!(is_binary_extension("a.docx"));
    // PDF is in neither set (text-authorable; guarded separately).
    assert!(!is_opaque_document_extension("a.pdf"));
    assert!(!is_binary_extension("a.pdf"));
}

/// PDF carve-out predicate.
#[test]
fn pdf_path_predicate() {
    assert!(is_pdf_path("Doc.PDF"));
    assert!(!is_pdf_path("doc.pdfx"));
    assert!(!is_pdf_path("pdf"));
}
