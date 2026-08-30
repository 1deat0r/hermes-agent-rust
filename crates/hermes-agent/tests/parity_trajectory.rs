//! Parity tests for `agent/trajectory.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); these cases derive from the upstream code as oracle.

use serde_json::json;

use hermes_agent::trajectory::{
    convert_scratchpad_to_think, has_incomplete_scratchpad, save_trajectory_at,
};

#[test]
fn scratchpad_tags_convert_to_think() {
    assert_eq!(
        convert_scratchpad_to_think("<REASONING_SCRATCHPAD>hmm</REASONING_SCRATCHPAD>"),
        "<think>hmm</think>"
    );
    // Both tags must be present for a swap; a lone opener converts too.
    assert_eq!(
        convert_scratchpad_to_think("<REASONING_SCRATCHPAD>partial"),
        "<think>partial"
    );
}

#[test]
fn non_scratchpad_content_is_byte_identical() {
    assert_eq!(convert_scratchpad_to_think(""), "");
    assert_eq!(convert_scratchpad_to_think("plain text"), "plain text");
    // The pre-check keys on the opening tag only.
    assert_eq!(
        convert_scratchpad_to_think("no tags </REASONING_SCRATCHPAD>"),
        "no tags </REASONING_SCRATCHPAD>"
    );
}

#[test]
fn incomplete_scratchpad_detection() {
    assert!(!has_incomplete_scratchpad(""));
    assert!(!has_incomplete_scratchpad("no tags"));
    assert!(!has_incomplete_scratchpad(
        "<REASONING_SCRATCHPAD>ok</REASONING_SCRATCHPAD>"
    ));
    assert!(has_incomplete_scratchpad(
        "<REASONING_SCRATCHPAD>never closed"
    ));
    // A closing tag without an opener is not "incomplete".
    assert!(!has_incomplete_scratchpad("stray </REASONING_SCRATCHPAD>"));
}

#[test]
fn save_appends_jsonl_entry() {
    let td = tempfile::TempDir::new().unwrap();
    let path = td.path().join("traj.jsonl");
    let conversation = vec![
        json!({"from": "human", "value": "hi"}),
        json!({"from": "gpt", "value": "hello"}),
    ];
    save_trajectory_at(&conversation, "test/model", true, Some(&path));
    let line = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = line.lines().collect();
    assert_eq!(lines.len(), 1, "one append = one line");
    let entry: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(entry["conversations"], json!(conversation));
    assert_eq!(entry["model"], "test/model");
    assert_eq!(entry["completed"], true);
    // datetime.now().isoformat() grammar: naive local ISO-8601.
    let ts = entry["timestamp"].as_str().unwrap();
    assert!(
        !ts.contains('+') && !ts.contains('Z'),
        "naive local timestamp, got {ts}"
    );
}

#[test]
fn save_appends_rather_than_truncates() {
    let td = tempfile::TempDir::new().unwrap();
    let path = td.path().join("traj.jsonl");
    save_trajectory_at(&[json!(1)], "m", true, Some(&path));
    save_trajectory_at(&[json!(2)], "m", false, Some(&path));
    let contents = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines.len(), 2);
    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(second["completed"], false);
}

#[test]
fn default_filename_follows_completed_flag() {
    use hermes_agent::trajectory::default_trajectory_filename;
    assert_eq!(
        default_trajectory_filename(true),
        "trajectory_samples.jsonl"
    );
    assert_eq!(
        default_trajectory_filename(false),
        "failed_trajectories.jsonl"
    );
}

#[test]
fn save_failure_is_fail_open() {
    // Writing into a nonexistent directory logs a warning and returns
    // None instead of raising.
    let td = tempfile::TempDir::new().unwrap();
    let path = td.path().join("no/such/dir/traj.jsonl");
    assert!(save_trajectory_at(&[json!(1)], "m", true, Some(&path)).is_none());
    assert!(!path.exists());
}
