//! Isolated (own process) oracle for the HERMES_DISABLE_FILE_STATE_GUARD
//! kill switch — the env var is process-global, so it can't share a test
//! binary with the parallel registry tests.

use hermes_tools::file_state::{check_stale, known_reads, note_write, record_read};

fn tmp_file(content: &str) -> String {
    static CTR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = CTR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("hfs_kill_{n}.txt"));
    std::fs::write(&path, content).expect("write");
    path.to_string_lossy().into_owned()
}

#[test]
fn kill_switch_env_disables() {
    std::env::set_var("HERMES_DISABLE_FILE_STATE_GUARD", "1");
    let p = tmp_file("x\n");
    record_read("A", &p, false);
    note_write("B", &p);
    assert!(
        check_stale("A", &p).is_none(),
        "kill switch must disable the guard"
    );
    assert!(known_reads("A").is_empty());
    std::env::remove_var("HERMES_DISABLE_FILE_STATE_GUARD");
    std::fs::remove_file(&p).ok();
}
