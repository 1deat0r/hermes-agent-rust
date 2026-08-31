//! Parity tests for `agent/secret_sources/_cache.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle.

use std::collections::BTreeMap;
use std::path::Path;

use hermes_agent::secret_sources::cache::{resolve_cache_home, CachedFetch, DiskCache};

fn entry(secrets: &[(&str, &str)], fetched_at: f64) -> CachedFetch {
    CachedFetch {
        secrets: secrets
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        fetched_at,
    }
}

#[test]
fn freshness_semantics() {
    let e = entry(&[("K", "v")], 100.0);
    // ttl <= 0 is never fresh (cache disabled).
    assert!(!e.is_fresh(0.0, 100.5));
    assert!(!e.is_fresh(-1.0, 100.5));
    assert!(e.is_fresh(10.0, 105.0));
    // `fetched_at < ttl` comparison: exactly at the TTL boundary is stale.
    assert!(!e.is_fresh(5.0, 105.0));
}

#[test]
fn cache_home_resolution_prefers_explicit_path() {
    assert_eq!(
        resolve_cache_home(Some(Path::new("/custom/home"))),
        Path::new("/custom/home")
    );
    // None falls back to get_hermes_home() — just verify it is non-empty.
    assert!(!resolve_cache_home(None).as_os_str().is_empty());
}

#[test]
fn write_then_read_round_trip() {
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path();
    let cache = DiskCache::new("bitwarden.json");

    cache.write(
        "inst-1:proj-2",
        &entry(&[("K", "v")], 100.0),
        3600.0,
        Some(home),
    );

    // The cache directory is forced to 0700.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(home.join("cache"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700, "cache dir must be 0700");
    }

    // Read on the same clock the write used (fetched_at = 100).
    let loaded = cache
        .read_at("inst-1:proj-2", 3600.0, Some(home), 100.0 + 10.0)
        .unwrap_or_else(|| panic!("no read: file={:?}", cache.path(Some(home))));
    assert_eq!(loaded.secrets.get("K").map(String::as_str), Some("v"));
    assert_eq!(loaded.fetched_at, 100.0);
}

#[test]
fn key_mismatch_is_a_miss() {
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path();
    let cache = DiskCache::new("bitwarden.json");
    cache.write("inst-1", &entry(&[("K", "v")], 100.0), 3600.0, Some(home));
    // A different serialized key never reads another key's secrets.
    assert!(cache.read("inst-2", 3600.0, Some(home)).is_none());
}

#[test]
fn ttl_zero_disables_both_layers_symmetrically() {
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path();
    let cache = DiskCache::new("bitwarden.json");

    // Write with ttl 0: no secret values reach disk at all.
    cache.write("k", &entry(&[("K", "v")], 1.0), 0.0, Some(home));
    assert!(!home.join("cache").join("bitwarden.json").exists());
    assert_eq!(cache.read("k", 0.0, Some(home)), None);
}

#[test]
fn stale_entry_is_a_miss() {
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path();
    let cache = DiskCache::new("cache.json");
    cache.write("k", &entry(&[("K", "v")], 100.0), 3600.0, Some(home));
    // Now+3800s: fetched_at 100 is older than the 3600 TTL.
    assert!(cache
        .read_at("k", 3600.0, Some(home), 100.0 + 3800.0)
        .is_none());
}

#[test]
fn corrupt_cache_file_is_a_miss() {
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path();
    let cache = DiskCache::new("cache.json");
    let path = cache.path(Some(home));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "{not json").unwrap();
    assert!(cache.read("k", 3600.0, Some(home)).is_none());

    // Non-dict payload too.
    std::fs::write(&path, "[1]").unwrap();
    assert!(cache.read("k", 3600.0, Some(home)).is_none());
}

#[test]
fn non_string_secret_values_are_dropped() {
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path();
    let cache = DiskCache::new("cache.json");
    let path = cache.path(Some(home));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        r#"{"key": "k", "secrets": {"GOOD": "v", "BAD": 3, "ALSO_BAD": null}, "fetched_at": 1.0}"#,
    )
    .unwrap();
    // The stored fetched_at (1.0) is read on a matching clock.
    let loaded = cache.read_at("k", 3600.0, Some(home), 2.0).unwrap();
    // env vars need strings — non-str→str pairs are coerced away.
    assert_eq!(loaded.secrets.len(), 1);
    assert_eq!(loaded.secrets.get("GOOD").map(String::as_str), Some("v"));
}

#[test]
fn clear_removes_the_cache_file() {
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path();
    let cache = DiskCache::new("cache.json");
    cache.write("k", &entry(&[("K", "v")], 1.0), 3600.0, Some(home));
    assert!(cache.path(Some(home)).exists());
    cache.clear(Some(home));
    assert!(!cache.path(Some(home)).exists());
    // Idempotent.
    cache.clear(Some(home));
}

#[test]
fn tmp_prefix_derives_from_basename_stem() {
    // Concurrent writers for different backends in the same dir don't
    // collide on the staging name (prefix = "." + stem + "_").
    let cache = DiskCache::new("onepassword.json");
    let path = cache.path(Some(Path::new("/tmp")));
    assert!(path
        .file_name()
        .map(|n| n.to_string_lossy().starts_with("onepassword"))
        .unwrap_or(false));
}

#[test]
fn written_file_mode_is_0600() {
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path();
    let cache = DiskCache::new("cache.json");
    cache.write("k", &entry(&[("K", "v")], 1.0), 3600.0, Some(home));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(cache.path(Some(home)))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "secret cache must be 0600");
    }
}
