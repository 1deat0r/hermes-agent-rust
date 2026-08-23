//! Parity oracles for tools/mcp_schema_cache.py, mirroring upstream
//! tests/tools/test_mcp_schema_cache.py @ b9aa928.

use hermes_tools::mcp_schema_cache::{
    clear_cache_entry, config_fingerprint, get_cached_entry, has_cached_entry,
    tools_from_cache_entry, utility_tools_from_cache_entry, write_cache_entry,
};
use serde_json::{json, Value};
use std::path::PathBuf;

/// The hermes-home override is process-global; serialize the tests that
/// swap it so concurrent tests can't read each other's temp home.
static CACHE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn with_home(tmp: &std::path::Path, f: impl FnOnce()) {
    let _guard = CACHE_TEST_LOCK.lock().unwrap();
    let token = hermes_constants::home::set_hermes_home_override(Some(tmp));
    f();
    hermes_constants::home::reset_hermes_home_override(token);
}

#[test]
fn fingerprint_stable_for_same_config() {
    let a = config_fingerprint(&json!({"command": "npx", "args": ["-y", "@playwright/mcp"]}));
    let b = config_fingerprint(&json!({"command": "npx", "args": ["-y", "@playwright/mcp"]}));
    assert_eq!(a, b);
    // Exact upstream oracle: hashlib.sha256(json.dumps(sort_keys, separators)).hexdigest()[:16]
    assert_eq!(a, "89cffa4240677aac");
}

#[test]
fn fingerprint_changes_when_connection_config_changes() {
    let base = json!({"command": "npx", "args": ["-y", "@playwright/mcp"]});
    let more = json!({"command": "npx", "args": ["-y", "@playwright/mcp", "--headless"]});
    assert_ne!(config_fingerprint(&base), config_fingerprint(&more));
    assert_eq!(config_fingerprint(&more), "0c03f1bde639ec95");
    let uvx = json!({"command": "uvx"});
    assert_ne!(config_fingerprint(&base), config_fingerprint(&uvx));
    assert_eq!(config_fingerprint(&uvx), "b91c463307212bad");
    let filtered = json!({"command": "npx", "args": [], "tools": {"include": ["a"]}});
    assert_ne!(config_fingerprint(&base), config_fingerprint(&filtered));
    assert_eq!(config_fingerprint(&filtered), "70f014fd8912da92");
}

#[test]
fn fingerprint_ignores_non_connection_keys() {
    let base = json!({"command": "npx", "args": []});
    let extra = json!({"command": "npx", "args": [], "timeout": 5, "enabled": true, "lazy": true});
    assert_eq!(config_fingerprint(&base), config_fingerprint(&extra));
    assert_eq!(config_fingerprint(&base), "0faa40d7c571704d");
}

#[test]
fn fingerprint_unicode_url_matches_upstream_dumps_escaping() {
    let a = config_fingerprint(&json!({"url": "https://exämple.com/δ", "transport": "http"}));
    assert_eq!(a, "f0c434d45f89a511");
    let b = config_fingerprint(&json!({"url": "https://exämple.com/δ", "transport": "http", "lazy": true}));
    assert_eq!(a, b);
}

/// Per-test scratch home (unique so parallel tests never share a cache file,
/// even if a future harness change drops the serialization lock).
fn temp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "hermes_mcp_cache_test_{}_{name}",
        std::process::id()
    ))
}

#[test]
fn write_then_read_with_matching_fingerprint() {
    let tmp = temp("write_then_read_with_matching_fingerprint");
    let _ = std::fs::remove_dir_all(&tmp);
    with_home(&tmp, || {
        let tools = vec![json!({"name": "t1", "description": "d", "inputSchema": {"type": "object"}})];
        write_cache_entry("srv", "fp1", tools.clone(), Some(vec![]));
        let entry = get_cached_entry("srv", "fp1");
        assert!(entry.is_some());
        assert_eq!(tools_from_cache_entry(&entry.unwrap()), tools);
        let entry = get_cached_entry("srv", "fp1").unwrap();
        assert_eq!(utility_tools_from_cache_entry(&entry), Vec::<Value>::new());
        assert!(has_cached_entry("srv", "fp1"));
    });
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn fingerprint_mismatch_returns_none() {
    let tmp = temp("fingerprint_mismatch_returns_none");
    let _ = std::fs::remove_dir_all(&tmp);
    with_home(&tmp, || {
        write_cache_entry("srv", "fp1", vec![], Some(vec![]));
        assert!(get_cached_entry("srv", "OTHER").is_none());
        assert!(!has_cached_entry("srv", "OTHER"));
    });
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn missing_server_returns_none() {
    let tmp = temp("missing_server_returns_none");
    let _ = std::fs::remove_dir_all(&tmp);
    with_home(&tmp, || {
        assert!(get_cached_entry("nope", "fp").is_none());
    });
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn clear_cache_entry_test() {
    let tmp = temp("clear_cache_entry_test");
    let _ = std::fs::remove_dir_all(&tmp);
    with_home(&tmp, || {
        write_cache_entry("srv", "fp1", vec![], Some(vec![]));
        clear_cache_entry("srv");
        assert!(get_cached_entry("srv", "fp1").is_none());
    });
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn corrupt_cache_file_is_tolerated() {
    let tmp = temp("corrupt_cache_file_is_tolerated");
    let _ = std::fs::remove_dir_all(&tmp);
    with_home(&tmp, || {
        let dir = tmp.join("cache");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mcp_schema_cache.json"), "{not json").unwrap();
        assert!(get_cached_entry("srv", "fp").is_none());
        write_cache_entry("srv", "fp", vec![], Some(vec![]));
        assert!(has_cached_entry("srv", "fp"));
    });
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn malformed_entry_shapes_are_tolerated() {
    assert_eq!(tools_from_cache_entry(&json!({"tools": "nope"})), Vec::<Value>::new());
    assert_eq!(utility_tools_from_cache_entry(&json!({})), Vec::<Value>::new());
}

#[test]
fn cache_lives_under_hermes_home_cache_dir_with_0600() {
    let tmp = temp("cache_lives_under_hermes_home_cache_dir_with_0600");
    let _ = std::fs::remove_dir_all(&tmp);
    with_home(&tmp, || {
        // Path is private; write then check the canonical location + mode.
        write_cache_entry("srv", "fp", vec![], Some(vec![]));
        let path = tmp.join("cache").join("mcp_schema_cache.json");
        assert!(path.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "cache file must be user-only (0o600)");
        }
        let entry = get_cached_entry("srv", "fp");
        assert!(entry.is_some());
    });
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn identical_payload_skips_rewrite() {
    let tmp = temp("identical_payload_skips_rewrite");
    let _ = std::fs::remove_dir_all(&tmp);
    with_home(&tmp, || {
        let tools = vec![json!({"name": "t1", "description": "d", "inputSchema": {}})];
        write_cache_entry("srv", "fp1", tools.clone(), Some(vec![]));
        let path = tmp.join("cache").join("mcp_schema_cache.json");
        let mtime1 = std::fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        // Identical payload (reconnect / list_changed refresh) → no rewrite.
        write_cache_entry("srv", "fp1", tools.clone(), Some(vec![]));
        let mtime2 = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(mtime1, mtime2, "identical payload should not rewrite the file");
        std::thread::sleep(std::time::Duration::from_millis(20));
        // Changed payload → rewrite.
        write_cache_entry("srv", "fp2", tools, Some(vec![]));
        let mtime3 = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert_ne!(mtime1, mtime3, "changed payload must rewrite the file");
    });
    let _ = std::fs::remove_dir_all(&tmp);
}
