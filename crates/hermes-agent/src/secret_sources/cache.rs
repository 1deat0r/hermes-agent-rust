//! Shared substrate for external secret-source backends.
//!
//! PARITY: `agent/secret_sources/_cache.py` @ b9aa928 (whole module).
//!
//! Every backend (Bitwarden, 1Password, …) needs the same handful of
//! security-sensitive primitives: a two-layer fetch cache whose disk half
//! writes atomically with `0600` permissions and honours a TTL
//! ([`DiskCache`], [`CachedFetch`]). The atomic-write / `0600` / TTL logic
//! is audited and fixed in exactly one place instead of drifting across
//! copy-pasted per-backend modules — each backend supplies only its own
//! cache-key shape and a serializer for it.
//!
//! Nothing here ever raises into the caller's hot path: the disk layer is
//! strictly best-effort (a miss just triggers a refetch), because a cache
//! problem must never block Hermes startup.

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64_URL;
use base64::Engine;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// A set of fetched secret values plus when they were fetched.
///
/// PARITY: `CachedFetch` (upstream lines 42-52).
#[derive(Debug, Clone, PartialEq)]
pub struct CachedFetch {
    pub secrets: std::collections::BTreeMap<String, String>,
    pub fetched_at: f64,
}

impl CachedFetch {
    /// PARITY: `is_fresh` (upstream lines 53-56) — `ttl_seconds <= 0` is
    /// never fresh.
    pub fn is_fresh(&self, ttl_seconds: f64, now: f64) -> bool {
        if ttl_seconds <= 0.0 {
            return false;
        }
        (now - self.fetched_at) < ttl_seconds
    }
}

/// Resolve the Hermes home used for cache paths: an explicit `home_path`,
/// else `$HERMES_HOME` / the platform default.
///
/// PARITY: `resolve_cache_home` (upstream lines 61-70).
pub fn resolve_cache_home(home_path: Option<&Path>) -> PathBuf {
    match home_path {
        Some(path) => path.to_path_buf(),
        None => hermes_constants::get_hermes_home(),
    }
}

/// Best-effort, profile-aware on-disk cache for fetched secret values.
///
/// One JSON object per backend lives at `<hermes_home>/cache/<basename>`:
/// `{"key": "...", "secrets": {...}, "fetched_at": 1.0}`. The file holds
/// only secret *values* keyed by the serialized cache key — never raw auth
/// material. Backends fingerprint tokens/sessions *before* they reach
/// `key_serializer` so the token can't land in the key.
///
/// Writes are atomic (`mkstemp` → chmod 0600 → os.replace) and the
/// containing `cache/` directory is forced to `0700` (mkdir's mode is
/// umask-subject). Both `read` and `write` short-circuit when
/// `ttl_seconds <= 0`, so a zero TTL disables *both* cache layers
/// symmetrically: a user opting out never gets secret values written to
/// disk at all.
///
/// PARITY: `DiskCache` (upstream lines 77-197).
pub struct DiskCache {
    basename: String,
    tmp_prefix: String,
}

impl DiskCache {
    /// The temp-file prefix derives from the basename so concurrent writers
    /// for different backends in the same dir don't collide on the staging
    /// name.
    ///
    /// PARITY: `__init__` (upstream lines 80-85).
    pub fn new(basename: &str) -> Self {
        let stem = basename.split('.').next().unwrap_or(basename).to_string();
        Self {
            basename: basename.to_string(),
            tmp_prefix: format!(".{stem}_"),
        }
    }

    /// PARITY: `path` (upstream lines 87-89).
    pub fn path(&self, home_path: Option<&Path>) -> PathBuf {
        resolve_cache_home(home_path)
            .join("cache")
            .join(&self.basename)
    }

    /// Return a fresh cached entry for `key`, or None.
    ///
    /// Best-effort: any I/O or parse error, a key mismatch, or a stale
    /// entry all return None so the caller re-fetches. JSON permits
    /// non-string values; env vars need strings, so anything that isn't a
    /// str→str pair is dropped.
    ///
    /// PARITY: `read` (upstream lines 91-129).
    pub fn read(
        &self,
        key: &str,
        ttl_seconds: f64,
        home_path: Option<&Path>,
    ) -> Option<CachedFetch> {
        self.read_at(key, ttl_seconds, home_path, now_unix_f64())
    }

    /// Explicit-clock form of [`DiskCache::read`].
    pub fn read_at(
        &self,
        key: &str,
        ttl_seconds: f64,
        home_path: Option<&Path>,
        now: f64,
    ) -> Option<CachedFetch> {
        if ttl_seconds <= 0.0 {
            return None;
        }
        let path = self.path(home_path);
        let raw = std::fs::read_to_string(path).ok()?;
        let payload: Value = serde_json::from_str(&raw).ok()?;
        let map = payload.as_object()?;
        if map.get("key").and_then(Value::as_str).map(|k| k == key) != Some(true) {
            return None;
        }
        let secrets = map.get("secrets")?.as_object()?;
        let fetched_at = map.get("fetched_at")?.as_f64()?;
        // JSON permits non-string values; env vars need strings, so coerce
        // by dropping anything that isn't a str→str pair.
        let mut typed = std::collections::BTreeMap::new();
        for (k, v) in secrets {
            if let Some(v_str) = v.as_str() {
                typed.insert(k.clone(), v_str.to_string());
            }
        }
        let entry = CachedFetch {
            secrets: typed,
            fetched_at,
        };
        if !entry.is_fresh(ttl_seconds, now) {
            return None;
        }
        Some(entry)
    }

    /// Persist `entry` for `key` atomically at mode `0600`.
    ///
    /// No-op when `ttl_seconds <= 0` (so caching is genuinely off) or on
    /// any I/O error — the next invocation just re-fetches.
    ///
    /// PARITY: `write` (upstream lines 132-193).
    pub fn write(
        &self,
        key: &str,
        entry: &CachedFetch,
        ttl_seconds: f64,
        home_path: Option<&Path>,
    ) {
        self.write_at(key, entry, ttl_seconds, home_path, &rand_nonce);
    }

    /// Explicit form of [`DiskCache::write`] with an injectable temp-nonce
    /// source (the mkstemp stand-in).
    pub fn write_at(
        &self,
        key: &str,
        entry: &CachedFetch,
        ttl_seconds: f64,
        home_path: Option<&Path>,
        nonce: &dyn Fn() -> String,
    ) {
        if ttl_seconds <= 0.0 {
            return;
        }
        let path = self.path(home_path);
        let cache_dir = match path.parent() {
            Some(dir) => dir.to_path_buf(),
            None => return,
        };
        let write_result = (|| -> std::io::Result<()> {
            std::fs::create_dir_all(&cache_dir)?;
            // mkdir's mode is umask-subject; chmod the dir to 0700 so cache
            // metadata isn't exposed if HERMES_HOME is ever made traversable.
            let _ = set_dir_mode_0700(&cache_dir);

            let payload = serde_json::json!({
                "key": key,
                "secrets": entry.secrets,
                "fetched_at": entry.fetched_at,
            });
            // mkstemp equivalent: a sibling temp file with a random suffix.
            let tmp = cache_dir.join(format!("{}{}.tmp", self.tmp_prefix, nonce()));
            {
                use std::io::Write;
                let mut f = std::fs::File::create(&tmp)?;
                f.write_all(
                    serde_json::to_string(&payload)
                        .unwrap_or_default()
                        .as_bytes(),
                )?;
            }
            // tempfile honours os.umask, so chmod 0600 before the rename.
            set_file_mode_0600(&tmp);
            if std::fs::rename(&tmp, &path).is_err() {
                let _ = std::fs::remove_file(&tmp);
                return Err(std::io::Error::other("rename failed"));
            }
            Ok(())
        })();
        if write_result.is_err() {
            // best-effort — a disk-cache miss next invocation is fine
            log::debug!("secret cache write failed; ignoring");
        }
    }

    /// Delete the on-disk cache file if present (idempotent).
    ///
    /// PARITY: `clear` (upstream lines 196-201).
    pub fn clear(&self, home_path: Option<&Path>) {
        let _ = std::fs::remove_file(self.path(home_path));
    }
}

fn set_dir_mode_0700(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(path, perms)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

fn set_file_mode_0600(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn now_unix_f64() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// `secrets.token_urlsafe`-shaped random suffix for the staging file.
fn rand_nonce() -> String {
    B64_URL.encode(rand::random::<[u8; 16]>())
}
