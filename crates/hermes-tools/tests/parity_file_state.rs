//! Parity oracles for the cross-agent FileStateRegistry, mirroring upstream
//! tests/tools/test_file_state_registry.py @ b9aa928. (The per-path
//! `lock_path` serialization tests are deferred with the executor's
//! task-concurrency layer; the registry-map locking is single-Mutex here.)

use std::sync::Mutex;

use hermes_tools::file_state::{check_stale, known_reads, note_write, record_read, writes_since, FileStateRegistry};

static FILE_CTR: Mutex<u64> = Mutex::new(0);

fn tmp_file(content: &str) -> String {
    let n = *FILE_CTR.lock().unwrap();
    *FILE_CTR.lock().unwrap() += 1;
    let path = std::env::temp_dir().join(format!("hfs_test_{n}.txt"));
    std::fs::write(&path, content).expect("write");
    path.to_string_lossy().into_owned()
}

fn mtime_ns(path: &str) -> Option<i64> {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as i64).unwrap_or(0))
}

fn touch_mtime(path: &str) {
    // Force the file's mtime observably forward (retry until granularity
    // actually advances, since some filesystems have coarse timestamps).
    for _ in 0..5 {
        std::thread::sleep(std::time::Duration::from_millis(25));
        std::fs::write(path, "updated\n").expect("touch");
        let _ = mtime_ns(path);
    }
}

fn now_ts() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

#[test]
fn record_read_then_check_stale_returns_none() {
    let p = tmp_file("x\n");
    record_read("A", &p, false);
    assert!(check_stale("A", &p).is_none());
    std::fs::remove_file(&p).ok();
}

#[test]
fn sibling_write_flags_other_agent_as_stale() {
    let p = tmp_file("x\n");
    record_read("A", &p, false);
    std::thread::sleep(std::time::Duration::from_millis(10));
    // B writes AFTER A's read (mtime + write-ts ordering).
    std::fs::write(&p, "y\n").expect("write");
    std::thread::sleep(std::time::Duration::from_millis(10));
    note_write("B", &p);
    let warn = check_stale("A", &p);
    assert!(warn.is_some(), "expected staleness warning");
    let w = warn.unwrap();
    assert!(w.contains("B"));
    assert!(w.to_lowercase().contains("sibling"));
    std::fs::remove_file(&p).ok();
}

#[test]
fn sibling_write_without_read_warns() {
    let p = tmp_file("x\n");
    note_write("B", &p);
    let warn = check_stale("A", &p).expect("warning");
    assert!(warn.contains("never read"));
    std::fs::remove_file(&p).ok();
}

#[test]
fn net_new_file_no_warning() {
    let p = tmp_file("x\n");
    // No reads and no writes recorded for this path.
    assert!(check_stale("A", &p).is_none());
    std::fs::remove_file(&p).ok();
}

#[test]
fn external_drift_detected() {
    let p = tmp_file("v1\n");
    record_read("A", &p, false);
    touch_mtime(&p); // external edit not recorded by note_write
    let mut warn = check_stale("A", &p);
    // Sub-second timer granularity can keep the mtime identical; re-touch
    // once harder before giving up.
    if warn.is_none() {
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(&p, "updated again\n").expect("rewrite");
        warn = check_stale("A", &p);
    }
    let warn = warn.expect("drift warning");
    assert!(warn.contains("modified since you last read"));
    std::fs::remove_file(&p).ok();
}

#[test]
fn partial_read_warns_on_write() {
    let p = tmp_file("content\n".repeat(50).as_str());
    record_read("A", &p, true);
    let warn = check_stale("A", &p).expect("partial warning");
    assert!(warn.contains("partial"));
    std::fs::remove_file(&p).ok();
}

#[test]
fn writes_since_filters_agent_and_time() {
    let p1 = tmp_file("a\n");
    let p2 = tmp_file("b\n");
    let t0 = now_ts();
    note_write("B", &p1);
    note_write("C", &p2);
    let out = writes_since("A", t0, &[p1.clone(), p2.clone()]);
    // Both writers appear.
    assert!(out.contains_key("B"));
    assert!(out.contains_key("C"));
    // Excluding B leaves only C.
    let out2 = writes_since("B", t0, &[p1.to_string(), p2.to_string()]);
    assert!(!out2.contains_key("B"));
    std::fs::remove_file(&p1).ok();
    std::fs::remove_file(&p2).ok();
}

#[test]
fn known_reads_returns_paths() {
    let p = tmp_file("x\n");
    record_read("A", &p, false);
    let reads = known_reads("A");
    assert!(reads.contains(&p));
    std::fs::remove_file(&p).ok();
}

#[test]
fn kill_switch_env_disables() {
    // Set the env var for the duration (registry reads it per call).
    std::env::set_var("HERMES_DISABLE_FILE_STATE_GUARD", "1");
    let p = tmp_file("x\n");
    record_read("A", &p, false);
    note_write("B", &p);
    assert!(check_stale("A", &p).is_none());
    std::env::remove_var("HERMES_DISABLE_FILE_STATE_GUARD");
    std::fs::remove_file(&p).ok();
}

#[test]
fn docs_style_clear_resets() {
    let reg = FileStateRegistry::new();
    let p = tmp_file("x\n");
    reg.record_read("A", &p, false, None);
    assert!(!reg.known_reads("A").is_empty());
    reg.clear();
    assert!(reg.known_reads("A").is_empty());
    std::fs::remove_file(&p).ok();
}

#[test]
fn binary_extension_helpers() {
    use hermes_tools::binary_extensions::{is_binary_extension, BINARY_EXTENSIONS};
    assert!(is_binary_extension("photo.png"));
    assert!(is_binary_extension("archive.zip"));
    assert!(!is_binary_extension("notes.md"));
    assert!(BINARY_EXTENSIONS.contains(".exe"));
}
