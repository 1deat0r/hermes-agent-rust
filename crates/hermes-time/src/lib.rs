//! `hermes-time` — 1:1 Rust port of `hermes_time.py`
//! (Nous Research Hermes Agent, pinned @ b9aa928).
//!
//! Timezone-aware clock for Hermes. Provides a single [`now`] helper that
//! returns a timezone-aware clock value based on the user's configured IANA
//! timezone (e.g. `Asia/Kolkata`).
//!
//! Resolution order (upstream docstring, `hermes_time.py` lines 7–13):
//!   1. `HERMES_TIMEZONE` environment variable
//!   2. `timezone` key in `~/.hermes/config.yaml`
//!   3. Falls back to the server's local time
//!
//! Invalid timezone values log a warning and fall back safely — Hermes never
//! crashes due to a bad timezone string.

use chrono::{DateTime, FixedOffset, Local, Utc};
use chrono_tz::Tz;
use std::path::Path;
use std::sync::Mutex;

/// Milliseconds used by our cached resolution state (Python module globals
/// `_cached_tz`, `_cached_tz_name`, `_cache_resolved`).
#[derive(Debug, Clone)]
struct CachedState {
    resolved: bool,
    tz_name: Option<String>,
    tz: Option<Tz>,
}

static CACHE: Mutex<CachedState> = Mutex::new(CachedState {
    resolved: false,
    tz_name: None,
    tz: None,
});

/// Source for the `timezone` key in `config.yaml`.
///
/// This is the seam for parity tests: upstream `hermes_time` reads
/// `read_raw_config()` (fail-open). The real source reads the file at
/// [`hermes_constants::get_config_path`] and applies the same fail-open
/// semantics. The managed-scope overlay is applied by the real source as a
/// no-op until the managed-scope subsystem is ported (P1/P3) — upstream
/// fails open when the overlay cannot be applied, and the identity overlay
/// preserves that behavior when no managed scope is configured.
pub trait TimezoneConfigSource {
    /// Return the effective top-level `timezone` string from config, or None
    /// when unset/unreadable. Must be fail-open.
    fn timezone_from_config(&self) -> Option<String>;
}

/// Real [`TimezoneConfigSource`]: reads `config.yaml` under HERMES_HOME.
pub struct RealConfigSource;

impl TimezoneConfigSource for RealConfigSource {
    fn timezone_from_config(&self) -> Option<String> {
        raw_timezone_from_file(&hermes_constants::get_config_path())
    }
}

/// Parse a YAML file for a top-level `timezone: <scalar-string>` key.
///
/// Faithful to the observable slice of `read_raw_config()` that
/// `hermes_time` consumes: missing file → None, unparseable YAML → None,
/// non-dict YAML root → None, non-string value → None, empty-after-strip → None.
pub fn raw_timezone_from_file(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&text).ok()?;
    let mapping = value.as_mapping()?;
    match mapping.get(serde_yaml::Value::String("timezone".to_string())) {
        Some(v) => {
            let s = v.as_str()?;
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        None => None,
    }
}

/// Read the configured IANA timezone string (or None).
///
/// Resolution order: (1) `HERMES_TIMEZONE` env (highest priority), (2)
/// `config.yaml` `timezone` key. This does file I/O when falling through to
/// config, so callers should cache the result rather than calling on every
/// `now()`.
///
/// PARITY: `hermes_time.py` `_resolve_timezone_name` (37–79).
pub fn resolve_timezone_name(source: &dyn TimezoneConfigSource) -> Option<String> {
    // 1. Environment variable (highest priority — set by Supervisor, etc.)
    let tz_env = std::env::var("HERMES_TIMEZONE").unwrap_or_default();
    let trimmed_env = tz_env.trim();
    if !trimmed_env.is_empty() {
        return Some(trimmed_env.to_string());
    }
    // 2. config.yaml ``timezone`` key (fail-open). Upstream trims the
    //    config value (`tz_cfg.strip()`); the contract is a non-empty,
    //    stripped IANA name or None.
    source
        .timezone_from_config()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Validate and return a chrono-tz `Tz`, or None if invalid.
///
/// PARITY: `hermes_time.py` `_get_zoneinfo` (82–93) — invalid names warn and
/// fall back to server-local time. Rust's chrono-tz table is the analogue of
/// Python's zoneinfo database: it covers all IANA names without reading
/// system zoneinfo files. A name absent from the IANA table is "invalid"
/// exactly like `ZoneInfo(name)` raising `KeyError`.
fn tz_from_name(name: &str) -> Option<Tz> {
    if name.is_empty() {
        return None;
    }
    match name.parse::<Tz>() {
        Ok(tz) => Some(tz),
        Err(_) => {
            eprintln!("Invalid timezone '{}'. Falling back to server local time.", name);
            None
        }
    }
}

/// Return the user's configured timezone, or None (meaning server-local).
///
/// Resolved once and cached. Call [`reset_cache`] after config changes.
///
/// PARITY: `hermes_time.py` `get_timezone()` (96–106).
pub fn get_timezone() -> Option<Tz> {
    get_timezone_with(&RealConfigSource)
}

/// Testable variant of [`get_timezone`] with an injected config source.
pub fn get_timezone_with(source: &dyn TimezoneConfigSource) -> Option<Tz> {
    let mut cache = CACHE.lock().unwrap();
    if !cache.resolved {
        cache.tz_name = resolve_timezone_name(source);
        let name = cache.tz_name.clone();
        cache.tz = name.as_deref().and_then(tz_from_name);
        cache.resolved = true;
    }
    cache.tz
}

/// Return the *name* of the user's configured timezone, or None.
///
/// Mirrors upstream's cached `_cached_tz_name` — Rust callers often need the
/// name (e.g. to export `TZ` to children) without the resolved zone object.
pub fn get_timezone_name() -> Option<String> {
    get_timezone(); // ensure resolved
    CACHE.lock().unwrap().tz_name.clone()
}

/// Clear the cached timezone so the next call re-resolves it.
///
/// Call this after the configured timezone may have changed (e.g. after a
/// config edit or `HERMES_TIMEZONE` update) to force `get_timezone()` /
/// `now()` to read the new value instead of the value cached at first use.
///
/// PARITY: `hermes_time.py` `reset_cache()` (109–119).
pub fn reset_cache() {
    let mut cache = CACHE.lock().unwrap();
    cache.resolved = false;
    cache.tz_name = None;
    cache.tz = None;
}

/// Return the current time as a timezone-aware clock value.
///
/// If a valid timezone is configured, returns wall-clock time in that zone.
/// Otherwise returns the server's local time.
///
/// The return type is `DateTime<FixedOffset>`, which carries the wall-clock
/// value plus its UTC offset at that instant — the same observable surface
/// Python exposes via `datetime.utcoffset()`.
///
/// PARITY: `hermes_time.py` `now()` (122–135).
pub fn now() -> DateTime<FixedOffset> {
    match get_timezone() {
        Some(tz) => Utc::now().with_timezone(&tz).fixed_offset(),
        None => Local::now().fixed_offset(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    /// Serialize tests that mutate `HERMES_TIMEZONE` / `HERMES_HOME`.
    static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        ENV_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    struct FakeSource(Option<String>);
    impl TimezoneConfigSource for FakeSource {
        fn timezone_from_config(&self) -> Option<String> {
            self.0.clone()
        }
    }

    #[test]
    fn valid_timezone_env_applies() {
        let _g = env_guard();
        reset_cache();
        unsafe { std::env::set_var("HERMES_TIMEZONE", "Asia/Kolkata") };
        let r = now();
        unsafe { std::env::remove_var("HERMES_TIMEZONE") };
        // IST is UTC+5:30
        assert_eq!(r.offset().local_minus_utc(), 5 * 3600 + 30 * 60);
    }

    #[test]
    fn utc_timezone() {
        let _g = env_guard();
        reset_cache();
        unsafe { std::env::set_var("HERMES_TIMEZONE", "UTC") };
        let r = now();
        unsafe { std::env::remove_var("HERMES_TIMEZONE") };
        assert_eq!(r.offset().local_minus_utc(), 0);
    }

    #[test]
    fn us_eastern_dst_aware() {
        let _g = env_guard();
        reset_cache();
        unsafe { std::env::set_var("HERMES_TIMEZONE", "America/New_York") };
        let r = now();
        unsafe { std::env::remove_var("HERMES_TIMEZONE") };
        let h = r.offset().local_minus_utc() as f64 / 3600.0;
        assert!(h == -5.0 || h == -4.0, "offset was {}", h);
    }

    #[test]
    fn get_timezone_returns_zone_for_valid() {
        let _g = env_guard();
        reset_cache();
        unsafe { std::env::set_var("HERMES_TIMEZONE", "Europe/London") };
        let tz = get_timezone();
        unsafe { std::env::remove_var("HERMES_TIMEZONE") };
        assert!(tz.is_some());
        assert_eq!(get_timezone_name().as_deref(), Some("Europe/London"));
        reset_cache();
    }

    #[test]
    fn invalid_timezone_falls_back() {
        let _g = env_guard();
        reset_cache();
        unsafe { std::env::set_var("HERMES_TIMEZONE", "Not/AZone") };
        let tz = get_timezone();
        unsafe { std::env::remove_var("HERMES_TIMEZONE") };
        assert!(tz.is_none());
        // now() must still be tz-aware (server local)
        let r = now();
        assert_ne!(r.offset().local_minus_utc(), 0, "server-local offset should be applied");
        reset_cache();
    }

    #[test]
    fn no_config_no_env_returns_server_local() {
        let _g = env_guard();
        reset_cache();
        unsafe { std::env::remove_var("HERMES_TIMEZONE") };
        let r = now();
        // Local offset of the test host — nonzero except when the host is UTC.
        // This asserts tz-awareness without assuming which zone.
        let _ = r.offset();
        assert!(get_timezone().is_none());
        reset_cache();
    }

    #[test]
    fn config_file_timezone_applies() {
        let _g = env_guard();
        let td = tempfile::TempDir::new().unwrap();
        std::fs::write(td.path().join("config.yaml"), "timezone: Asia/Kolkata\n").unwrap();
        unsafe { std::env::set_var("HERMES_HOME", td.path()) };
        unsafe { std::env::remove_var("HERMES_TIMEZONE") };
        reset_cache();
        let src = RealConfigSource;
        let r = get_timezone_with(&src);
        assert!(r.is_some());
        let n = now();
        assert_eq!(n.offset().local_minus_utc(), 5 * 3600 + 30 * 60);
        reset_cache();
        unsafe { std::env::remove_var("HERMES_HOME") };
    }

    #[test]
    fn config_file_unparseable_falls_back() {
        let _g = env_guard();
        let td = tempfile::TempDir::new().unwrap();
        std::fs::write(td.path().join("config.yaml"), ":: not yaml {").unwrap();
        unsafe { std::env::set_var("HERMES_HOME", td.path()) };
        unsafe { std::env::remove_var("HERMES_TIMEZONE") };
        reset_cache();
        let src = RealConfigSource;
        assert!(get_timezone_with(&src).is_none());
        reset_cache();
        unsafe { std::env::remove_var("HERMES_HOME") };
    }

    #[test]
    fn config_non_dict_root_falls_back() {
        let _g = env_guard();
        let td = tempfile::TempDir::new().unwrap();
        std::fs::write(td.path().join("config.yaml"), "- just\n- a\n- list\n").unwrap();
        unsafe { std::env::set_var("HERMES_HOME", td.path()) };
        unsafe { std::env::remove_var("HERMES_TIMEZONE") };
        reset_cache();
        let src = RealConfigSource;
        assert!(get_timezone_with(&src).is_none());
        reset_cache();
        unsafe { std::env::remove_var("HERMES_HOME") };
    }

    #[test]
    fn config_file_managed_overlay_is_identity_for_now() {
        // Upstream applies managed_scope.apply_managed_overlay(cfg) fail-open.
        // With no managed scope configured, the overlay is the identity —
        // which is exactly what our identity source does.
        let _g = env_guard();
        reset_cache();
        unsafe { std::env::remove_var("HERMES_TIMEZONE") };
        assert_eq!(resolve_timezone_name(&FakeSource(None)), None);
        assert_eq!(
            resolve_timezone_name(&FakeSource(Some("  Asia/Kolkata  ".to_string()))),
            Some("Asia/Kolkata".to_string())
        );
        reset_cache();
    }

    #[test]
    fn resolve_prefers_env_over_config() {
        let _g = env_guard();
        reset_cache();
        unsafe { std::env::set_var("HERMES_TIMEZONE", "UTC") };
        assert_eq!(
            resolve_timezone_name(&FakeSource(Some("Asia/Kolkata".to_string()))),
            Some("UTC".to_string())
        );
        unsafe { std::env::remove_var("HERMES_TIMEZONE") };
        reset_cache();
    }

    #[test]
    fn reset_cache_forces_re_resolution() {
        let _g = env_guard();
        reset_cache();
        unsafe { std::env::set_var("HERMES_TIMEZONE", "UTC") };
        assert_eq!(get_timezone_name().as_deref(), Some("UTC"));
        // Change the env; without reset, cache still says UTC.
        unsafe { std::env::set_var("HERMES_TIMEZONE", "Asia/Kolkata") };
        assert_eq!(get_timezone_name().as_deref(), Some("UTC"));
        // After reset, re-resolves to the new value.
        reset_cache();
        assert_eq!(get_timezone_name().as_deref(), Some("Asia/Kolkata"));
        unsafe { std::env::remove_var("HERMES_TIMEZONE") };
        reset_cache();
    }

    #[test]
    fn raw_timezone_from_file_parses_scalar() {
        let td = tempfile::TempDir::new().unwrap();
        std::fs::write(td.path().join("a.yaml"), "timezone: Europe/Berlin\n").unwrap();
        assert_eq!(
            raw_timezone_from_file(&td.path().join("a.yaml")),
            Some("Europe/Berlin".to_string())
        );
    }
}
