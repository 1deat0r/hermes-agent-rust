//! Preventive SSL CA certificate checks for Hermes Agent.
//!
//! PARITY: `agent/ssl_guard.py` @ b9aa928 (whole module).
//!
//! Catches broken CA bundle paths before HTTP-client construction turns
//! them into opaque "No such file or directory" failures.
//!
//! TRANSLATION NOTE: upstream's certifi leg (`ssl.create_default_context`
//! + `get_ca_certs()`) becomes a PEM block-presence check — a bundle with
//! zero `BEGIN CERTIFICATE` blocks "did not load any certificates". True
//! x509 parsing is left to the TLS stack at client-construction time; this
//! guard's contract is catching missing/empty/truncated bundles early.

use std::path::Path;

use crate::errors::SSLConfigurationError;

/// PARITY: `_CA_BUNDLE_ENV_VARS` (upstream lines 17-23).
pub const CA_BUNDLE_ENV_VARS: [&str; 4] = [
    "HERMES_CA_BUNDLE",
    "SSL_CERT_FILE",
    "REQUESTS_CA_BUNDLE",
    "CURL_CA_BUNDLE",
];

/// PARITY: `_SKIP_VALUES` (upstream line 25).
const SKIP_VALUES: [&str; 4] = ["1", "true", "yes", "on"];

/// PARITY: `_skip_ssl_guard_enabled` (upstream lines 27-29).
fn skip_ssl_guard_enabled() -> bool {
    let value = std::env::var("HERMES_SKIP_SSL_GUARD").unwrap_or_default();
    let value = value.trim().to_lowercase();
    SKIP_VALUES.contains(&value.as_str())
}

/// PARITY: `_repair_hint` (upstream lines 32-36).
fn repair_hint() -> String {
    "Repair: run `hermes doctor --fix` (auto-reinstalls certifi), or \
     manually: python -m pip install --force-reinstall certifi openai httpx\n\
     If you configured a custom corporate CA bundle, fix or unset the \
     broken CA bundle environment variable."
        .to_string()
}

/// PARITY: `_ssl_err` — a consistent, user-actionable SSL error.
fn ssl_err(message: &str) -> SSLConfigurationError {
    SSLConfigurationError {
        message: format!("{message}\n{}", repair_hint()),
    }
}

/// Count PEM certificate blocks (`BEGIN CERTIFICATE` … `END CERTIFICATE`).
fn count_cert_blocks(text: &str) -> usize {
    text.matches("-----BEGIN CERTIFICATE-----").count()
}

/// PARITY: `_validate_bundle_path` (upstream lines 43-59) — the checks in
/// order: exists → is a file → (optionally) substantial size → loads
/// certificates. `platform_bundle_pem` stands in for the loaded-context
/// certificate count; when `None`, the load arm degrades to the PEM-block
/// count from the file contents.
fn validate_bundle_path(
    label: &str,
    value: &str,
    require_substantial: bool,
    platform_bundle_pem: Option<&str>,
) -> Result<(), SSLConfigurationError> {
    let expanded = expanduser(value);
    let path = Path::new(&expanded);
    if !path.exists() {
        return Err(ssl_err(&format!(
            "{label} points to a missing CA bundle: {value}"
        )));
    }
    if !path.is_file() {
        return Err(ssl_err(&format!(
            "{label} does not point to a CA bundle file: {value}"
        )));
    }
    if require_substantial {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if size < 1024 {
            return Err(ssl_err(&format!(
                "{label} at {value} appears corrupted (too small)"
            )));
        }
    }
    // `ssl.create_default_context(cafile=...)` + `get_ca_certs()` — the
    // bundle must yield at least one certificate.
    let contents = std::fs::read_to_string(path).map_err(|e| {
        ssl_err(&format!(
            "{label} CA bundle at {value} cannot be loaded: {e}"
        ))
    })?;
    let loaded = platform_bundle_pem
        .map(|pem| count_cert_blocks(pem))
        .unwrap_or_else(|| {
            // No platform parser wired: count PEM blocks in the bundle itself.
            count_cert_blocks(&contents)
        });
    if loaded == 0 {
        return Err(ssl_err(&format!(
            "{label} CA bundle at {value} did not load any certificates"
        )));
    }
    Ok(())
}

/// `Path.expanduser` equivalent (a leading `~` becomes `$HOME`).
fn expanduser(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return format!("{}/{}", home.to_string_lossy(), rest);
        }
    }
    value.to_string()
}

/// Verify configured and bundled CA certificates are present and loadable.
///
/// Errors (`SSLConfigurationError`) when an explicit CA-bundle environment
/// variable points at a bad path, or when the platform's bundled
/// `cacert.pem` equivalent is missing/corrupt.
///
/// PARITY: `verify_ca_bundle` (upstream lines 62-84). Upstream's certifi
/// leg takes the platform bundle path as a parameter here; `None` skips
/// the platform-bundle leg (matching an install where no bundled store
/// exists).
pub fn verify_ca_bundle(platform_bundle: Option<&Path>) -> Result<(), SSLConfigurationError> {
    if skip_ssl_guard_enabled() {
        log::debug!("SSL CA bundle guard skipped via HERMES_SKIP_SSL_GUARD");
        return Ok(());
    }

    for env_var in CA_BUNDLE_ENV_VARS {
        if let Ok(value) = std::env::var(env_var) {
            if !value.is_empty() {
                validate_bundle_path(env_var, &value, false, None)?;
            }
        }
    }

    let Some(platform_bundle) = platform_bundle else {
        // `import certifi` failure arm: no bundled store to validate —
        // surface the actionable error.
        return Err(ssl_err(
            "certifi is not importable: no bundled CA store is configured for this platform",
        ));
    };
    validate_bundle_path("certifi", &platform_bundle.to_string_lossy(), true, None)
}

/// Backward-compatible wrapper for older call sites — the old PR name
/// mentioned a platform fallback, but allowing startup with a broken
/// certifi bundle still leaves call sites failing later.
///
/// PARITY: `verify_ca_bundle_with_fallback` (upstream lines 87-94).
pub fn verify_ca_bundle_with_fallback(
    platform_bundle: Option<&Path>,
) -> Result<(), SSLConfigurationError> {
    verify_ca_bundle(platform_bundle)
}
