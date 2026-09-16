//! Binary file extensions to skip for text-based operations.
//!
//! PARITY: tools/binary_extensions.py @ 5d59366 (whole module).
//!
//! Name divergence (pre-existing, kept): upstream `has_binary_extension` /
//! `has_opaque_document_extension` read as `is_*` here; the `_has_extension_in`
//! helper shape is preserved as [`has_extension_in`]. PDF is in neither set
//! (raw PDF syntax is text-authorable); the write guard handles overwrites
//! via [`is_pdf_path`].

use once_cell::sync::Lazy;
use std::collections::HashSet;

pub static BINARY_EXTENSIONS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        // Images
        ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".ico", ".webp", ".tiff", ".tif",
        // Videos
        ".mp4", ".mov", ".avi", ".mkv", ".webm", ".wmv", ".flv", ".m4v", ".mpeg", ".mpg",
        // Audio
        ".mp3", ".wav", ".ogg", ".flac", ".aac", ".m4a", ".wma", ".aiff", ".opus",
        // Archives
        ".zip", ".tar", ".gz", ".bz2", ".7z", ".rar", ".xz", ".z", ".tgz", ".iso",
        // Executables/binaries
        ".exe", ".dll", ".so", ".dylib", ".bin", ".o", ".a", ".obj", ".lib", ".app", ".msi", ".deb",
        ".rpm", // Documents (exclude .pdf)
        ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx", ".odt", ".ods", ".odp",
        // Fonts
        ".ttf", ".otf", ".woff", ".woff2", ".eot", // Bytecode / VM artifacts
        ".pyc", ".pyo", ".class", ".jar", ".war", ".ear", ".node", ".wasm", ".rlib",
        // Database files
        ".sqlite", ".sqlite3", ".db", ".mdb", ".idx", // Design / 3D
        ".psd", ".ai", ".eps", ".sketch", ".fig", ".xd", ".blend", ".3ds", ".max",
        // Flash / misc
        ".swf",
    ])
});

// PARITY: `_has_extension_in` — case-insensitive check on the final
// `.suffix`; pure string, no I/O. `rfind` on a path with no dot yields -1
// upstream → false here.
pub fn has_extension_in(path: &str, extensions: &HashSet<&str>) -> bool {
    match path.rfind('.') {
        Some(i) => extensions.contains(path[i..].to_lowercase().as_str()),
        None => false,
    }
}

/// True when *path*'s lowercase extension is in the binary set.
pub fn is_binary_extension(path: &str) -> bool {
    has_extension_in(path, &BINARY_EXTENSIONS)
}

// PARITY: `OPAQUE_DOCUMENT_EXTENSIONS` — container documents (OOXML/ODF/
// EPUB zips, OLE, RTF) a plain-text write can NEVER produce validly:
// read_file auto-extracts them, so writing text back destroys the document.
pub static OPAQUE_DOCUMENT_EXTENSIONS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        ".doc", ".docx", ".docm", ".xls", ".xlsx", ".xlsm", ".xlsb", ".ppt", ".pps", ".pot",
        ".pptx", ".pptm", ".ppsx", ".ppsm", ".odt", ".ods", ".odp", ".rtf", ".epub",
    ])
});

/// True when *path* is an opaque container document (write-guard input).
pub fn is_opaque_document_extension(path: &str) -> bool {
    has_extension_in(path, &OPAQUE_DOCUMENT_EXTENSIONS)
}

/// True for `.pdf` paths (case-insensitive). PDF is text-authorable — only
/// overwrites are dangerous, handled by the write guard.
pub fn is_pdf_path(path: &str) -> bool {
    path.to_lowercase().ends_with(".pdf")
}
