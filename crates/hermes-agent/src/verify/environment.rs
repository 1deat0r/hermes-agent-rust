//! Environment manifest for project verification.
//!
//! PARITY: `agent/verify/environment.py` @ b9aa928 (whole module). Ported
//! from superagent-ai/grok-cli `src/verify/environment.ts`. The manifest
//! lives at `<project>/.hermes/environment.json` and is the user-editable
//! source of truth: when present and valid it wins over fresh static
//! detection.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::recipes::{detect_recipe, Recipe};

/// PARITY: `MANIFEST_VERSION` (upstream line 15).
pub const MANIFEST_VERSION: i64 = 1;

/// PARITY: `_MANIFEST_RELPATH` (upstream line 16).
const MANIFEST_RELPATH: &str = ".hermes/environment.json";

/// Path of the verify manifest for the project at `root`.
///
/// PARITY: `manifest_path` (upstream lines 19-21).
pub fn manifest_path(root: &Path) -> PathBuf {
    root.join(MANIFEST_RELPATH)
}

/// Load the saved recipe from the manifest, tolerating malformed files.
///
/// Mirrors grok's `loadVerifyEnvironment`: any read/parse/shape problem
/// returns `None` rather than raising, so a corrupt manifest degrades to
/// fresh detection instead of breaking `hermes verify`.
///
/// PARITY: `load_manifest` (upstream lines 24-42).
pub fn load_manifest(root: &Path) -> Option<Recipe> {
    let path = manifest_path(root);
    let raw = std::fs::read_to_string(path).ok()?;
    let manifest: Value = serde_json::from_str(&raw).ok()?;
    let manifest = manifest.as_object()?;
    // Accept both the wrapped {version, recipe} shape and a bare recipe.
    let bare = Value::Object(manifest.clone());
    let recipe_raw = manifest.get("recipe").unwrap_or(&bare);
    Recipe::from_dict(recipe_raw)
}

/// Persist `recipe` as the project's verify manifest.
///
/// Writes the versioned wrapper shape (grok's `saveVerifyEnvironment`
/// equivalent) and returns the manifest path.
///
/// PARITY: `save_manifest` (upstream lines 45-57). The timestamp is
/// timezone-aware UTC (`datetime.now(timezone.utc).isoformat()`).
pub fn save_manifest(root: &Path, recipe: &Recipe) -> std::io::Result<PathBuf> {
    let path = manifest_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    let payload = json!({
        "version": MANIFEST_VERSION,
        "recipe": recipe.to_dict(),
        "updatedAt": now,
    });
    std::fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        ),
    )?;
    Ok(path)
}

/// Return (recipe, source) where source is 'manifest' or 'detected'.
///
/// A saved manifest wins over fresh detection, matching grok-cli's
/// behavior where the environment file is the source of truth.
///
/// PARITY: `load_or_detect` (upstream lines 60-66).
pub fn load_or_detect(root: &Path) -> (Option<Recipe>, &'static str) {
    if let Some(saved) = load_manifest(root) {
        return (Some(saved), "manifest");
    }
    (detect_recipe(root), "detected")
}
