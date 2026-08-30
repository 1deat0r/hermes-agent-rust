//! Trajectory saving utilities and static helpers.
//!
//! PARITY: `agent/trajectory.py` @ b9aa928 (whole module).
//!
//! `_convert_to_trajectory_format` stays as an `AIAgent` method upstream
//! (batch_runner.py calls `agent._convert_to_trajectory_format`). Only the
//! static helpers and the file-write logic live here.

use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Local;
use serde_json::{json, Value};

/// Convert `<REASONING_SCRATCHPAD>` tags to `<think>` tags.
///
/// PARITY: `convert_scratchpad_to_think` (upstream lines 15-19). The `not
/// content` and substring pre-checks keep non-scratchpad content
/// byte-identical and skip the replace work.
pub fn convert_scratchpad_to_think(content: &str) -> String {
    if content.is_empty() || !content.contains("<REASONING_SCRATCHPAD>") {
        return content.to_string();
    }
    content
        .replace("<REASONING_SCRATCHPAD>", "<think>")
        .replace("</REASONING_SCRATCHPAD>", "</think>")
}

/// Check if content has an opening `<REASONING_SCRATCHPAD>` without a
/// closing tag.
///
/// PARITY: `has_incomplete_scratchpad` (upstream lines 22-26).
pub fn has_incomplete_scratchpad(content: &str) -> bool {
    if content.is_empty() {
        return false;
    }
    content.contains("<REASONING_SCRATCHPAD>") && !content.contains("</REASONING_SCRATCHPAD>")
}

/// The default output name for a trajectory: `trajectory_samples.jsonl`
/// when the conversation completed, else `failed_trajectories.jsonl`.
///
/// PARITY: the `filename = "trajectory_samples.jsonl" if completed else
/// "failed_trajectories.jsonl"` expression (upstream lines 37-38).
pub fn default_trajectory_filename(completed: bool) -> &'static str {
    if completed {
        "trajectory_samples.jsonl"
    } else {
        "failed_trajectories.jsonl"
    }
}

/// Append a trajectory entry to a JSONL file.
///
/// PARITY: `save_trajectory` (upstream lines 29-50). `filename = None`
/// defaults to `trajectory_samples.jsonl` or `failed_trajectories.jsonl`
/// based on `completed`. The entry carries the ShareGPT-format conversation
/// list verbatim, a local-time ISO timestamp, the model, and the completed
/// flag. Fail-open: an IO or serialization error logs a warning (via the
/// `log` facade — the sink stays with `hermes-logging`, matching upstream's
/// module logger) and returns `None` instead of raising.
///
/// Returns the path written, for callers/tests; upstream returns `None`
/// implicitly.
pub fn save_trajectory_at(
    trajectory: &[Value],
    model: &str,
    completed: bool,
    filename: Option<&Path>,
) -> Option<PathBuf> {
    let path: PathBuf = match filename {
        Some(path) => path.to_path_buf(),
        None => PathBuf::from(default_trajectory_filename(completed)),
    };
    let entry = json!({
        "conversations": trajectory,
        // `datetime.now().isoformat()` — naive local time, microseconds.
        "timestamp": Local::now().naive_local().format("%Y-%m-%dT%H:%M:%S%.6f").to_string(),
        "model": model,
        "completed": completed,
    });
    let result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| {
            writeln!(
                f,
                "{}",
                serde_json::to_string(&entry).unwrap_or_else(|_| "null".to_string())
            )
        });
    match result {
        Ok(()) => {
            log::info!("Trajectory saved to {}", path.display());
            Some(path)
        }
        Err(e) => {
            log::warn!("Failed to save trajectory: {e}");
            None
        }
    }
}

/// Explicit-clock-free form of [`save_trajectory_at`] using the upstream
/// default-filename logic. `filename = None` picks
/// `trajectory_samples.jsonl` / `failed_trajectories.jsonl` from
/// `completed`.
pub fn save_trajectory(trajectory: &[Value], model: &str, completed: bool) -> Option<PathBuf> {
    save_trajectory_at(trajectory, model, completed, None)
}
