//! Isolated (own-process) oracles for env-dependent file-safety guards.
//! The env vars are process-global, so these must not share a test binary
//! with the parallel pure-path tests.

use hermes_tools::file_safety::{
    classify_write_denial, get_read_block_error, get_safe_write_roots,
    get_write_denied_error,
};

fn tmp_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "hfs_env_{label}_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn hermes_home_guard_cases() {
    // HERMES_HOME-driven cache-file + state-db guards, run sequentially in
    // one test to avoid env-var races with other tests in this binary.
    let dir = tmp_dir("hub");
    let cache = dir.join("skills/.hub/index-cache/data.json");
    std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
    std::fs::write(&cache, "{}").unwrap();
    let hub = dir.join("skills/.hub/metadata.json");
    std::fs::write(&hub, "{}").unwrap();
    let state_db = dir.join("state.db");
    std::fs::write(&state_db, "x").unwrap();

    let old = std::env::var("HERMES_HOME").ok();
    std::env::set_var("HERMES_HOME", &dir);
    {
        let error = get_read_block_error(&cache.to_string_lossy());
        assert!(error.is_some());
        assert!(error.unwrap().contains("internal Hermes cache"));

        let error = get_read_block_error(&hub.to_string_lossy());
        assert!(error.is_some());

        // Regular project .env still blocked; .env.example allowed.
        assert!(get_read_block_error("/workspace/.env").is_some());
        assert!(get_read_block_error("/workspace/.env.example").is_none());

        // state.db write denial.
        assert_eq!(classify_write_denial(&state_db.to_string_lossy()), Some("credential"));
    }
    match old {
        Some(v) => std::env::set_var("HERMES_HOME", v),
        None => std::env::remove_var("HERMES_HOME"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn safe_root_gate_cases() {
    // HERMES_WRITE_SAFE_ROOT gate, run in one test.
    let old = std::env::var("HERMES_WRITE_SAFE_ROOT").ok();
    let dir = tmp_dir("safe");
    std::env::set_var("HERMES_WRITE_SAFE_ROOT", &dir);
    {
        assert_eq!(classify_write_denial("/tmp/other/file.txt"), Some("safe_root"));
        let err = get_write_denied_error("/tmp/other/file.txt", "Write");
        assert!(err.is_some() && err.unwrap().contains("HERMES_WRITE_SAFE_ROOT"));
        assert!(classify_write_denial(&dir.join("file.txt").to_string_lossy()).is_none());
        assert!(get_safe_write_roots().contains(&dir.to_string_lossy().into_owned()));
    }
    match old {
        Some(v) => std::env::set_var("HERMES_WRITE_SAFE_ROOT", v),
        None => std::env::remove_var("HERMES_WRITE_SAFE_ROOT"),
    }
    assert!(classify_write_denial("/tmp/other/file.txt").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
