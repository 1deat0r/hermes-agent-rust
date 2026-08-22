//! Binary file extensions to skip for text-based operations.
//!
//! PARITY: tools/binary_extensions.py @ b9aa928 (42 LOC, ported 1:1).

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
        ".exe", ".dll", ".so", ".dylib", ".bin", ".o", ".a", ".obj", ".lib",
        ".app", ".msi", ".deb", ".rpm",
        // Documents (exclude .pdf)
        ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx",
        ".odt", ".ods", ".odp",
        // Fonts
        ".ttf", ".otf", ".woff", ".woff2", ".eot",
        // Bytecode / VM artifacts
        ".pyc", ".pyo", ".class", ".jar", ".war", ".ear", ".node", ".wasm", ".rlib",
        // Database files
        ".sqlite", ".sqlite3", ".db", ".mdb", ".idx",
        // Design / 3D
        ".psd", ".ai", ".eps", ".sketch", ".fig", ".xd", ".blend", ".3ds", ".max",
        // Flash / misc
        ".swf",
    ])
});

/// True when *path*'s lowercase extension is in the binary set.
pub fn is_binary_extension(path: &str) -> bool {
    let ext = match path.rfind('.') {
        Some(i) => &path[i..],
        None => return false,
    };
    BINARY_EXTENSIONS.contains(ext.to_lowercase().as_str())
}
