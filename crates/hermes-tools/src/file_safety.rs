//! Shared path-safety guards for tool implementations.
//!
//! PARITY: agent/file_safety.py @ b9aa928 (693 LOC, ported 1:1 for the
//! observable returns of write/read denial classification).
//!
//! **This is NOT a security boundary** — the terminal tool runs as the same
//! OS user and can bypass every check; these blocks are defense-in-depth
//! against model-side credential leakage.

use std::path::{Path, PathBuf};

use hermes_constants::home::{get_default_hermes_root, get_hermes_home};

fn hermes_home_path() -> PathBuf {
    get_hermes_home()
}

fn hermes_root_path() -> PathBuf {
    get_default_hermes_root()
}

/// Resolve a path like `os.path.realpath(os.path.expanduser(...))`.
fn realpath_home(path: &str) -> String {
    let p = shellexpand::tilde(path).to_string();
    std::fs::canonicalize(&p)
        .unwrap_or_else(|_| PathBuf::from(&p))
        .to_string_lossy()
        .into_owned()
}

fn realpath(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "~".to_string())
}

/// Exact sensitive paths that must never be written.
pub fn build_write_denied_paths(home: &str) -> std::collections::HashSet<String> {
    let hh = realpath(&hermes_home_path());
    let hr = realpath(&hermes_root_path());
    [
        format!("{home}/.ssh/authorized_keys"),
        format!("{home}/.ssh/id_rsa"),
        format!("{home}/.ssh/id_ed25519"),
        format!("{home}/.ssh/config"),
        format!("{hh}/.env"),
        format!("{hr}/.env"),
        format!("{hh}/.anthropic_oauth.json"),
        format!("{hr}/.anthropic_oauth.json"),
        format!("{hh}/cache/bws_cache.enc.json"),
        format!("{hr}/cache/bws_cache.enc.json"),
        format!("{home}/.netrc"),
        format!("{home}/.pgpass"),
        format!("{home}/.npmrc"),
        format!("{home}/.pypirc"),
        format!("{home}/.git-credentials"),
        "/etc/sudoers".to_string(),
        "/etc/passwd".to_string(),
        "/etc/shadow".to_string(),
    ]
    .into_iter()
    .map(|p| realpath_home(&p))
    .collect()
}

/// Sensitive directory prefixes that must never be written.
pub fn build_write_denied_prefixes(home: &str) -> Vec<String> {
    [
        format!("{home}/.ssh"),
        format!("{home}/.aws"),
        format!("{home}/.gnupg"),
        format!("{home}/.kube"),
        "/etc/sudoers.d".to_string(),
        "/etc/systemd".to_string(),
        format!("{home}/.docker"),
        format!("{home}/.azure"),
        format!("{home}/.config/gh"),
        format!("{home}/.config/gcloud"),
    ]
    .into_iter()
    .map(|p| format!("{}/", realpath_home(&p)))
    .collect()
}

/// Resolved HERMES_WRITE_SAFE_ROOT paths (os.pathsep-separated).
pub fn get_safe_write_roots() -> std::collections::HashSet<String> {
    let env = std::env::var("HERMES_WRITE_SAFE_ROOT").unwrap_or_default();
    if env.is_empty() {
        return std::collections::HashSet::new();
    }
    let sep = if cfg!(windows) { ';' } else { ':' };
    let mut roots = std::collections::HashSet::new();
    for part in env.split(sep) {
        if !part.is_empty() {
            roots.insert(realpath_home(part));
        }
    }
    roots
}

/// Returns `Some("credential")`, `Some("safe_root")`, or None if allowed.
pub fn classify_write_denial(path: &str) -> Option<&'static str> {
    let home = realpath_home(&home_dir());
    let resolved = realpath_home(path);

    if build_write_denied_paths(&home).contains(&resolved) {
        return Some("credential");
    }
    for prefix in build_write_denied_prefixes(&home) {
        if resolved.starts_with(&prefix) {
            return Some("credential");
        }
    }

    let mcp_tokens_dir_name = "mcp-tokens";
    let mut hermes_dirs: Vec<PathBuf> = Vec::new();
    for base in [hermes_home_path(), hermes_root_path()] {
        let real = realpath(&base);
        let b = PathBuf::from(&real);
        if !hermes_dirs.contains(&b) {
            hermes_dirs.push(b);
        }
    }

    for base_real in &hermes_dirs {
        // Session transcripts are application-owned state.
        if resolved == realpath(&base_real.join("state.db")) {
            return Some("credential");
        }
        let sessions_real = realpath(&base_real.join("sessions"));
        if resolved == sessions_real || resolved.starts_with(&format!("{sessions_real}/")) {
            return Some("credential");
        }
        // mcp-tokens + pairing are credential stores.
        for sub in [mcp_tokens_dir_name, "pairing"] {
            let sub_real = realpath(&base_real.join(sub));
            if resolved == sub_real || resolved.starts_with(&format!("{sub_real}/")) {
                return Some("credential");
            }
        }
    }

    let safe_roots = get_safe_write_roots();
    if !safe_roots.is_empty() {
        let mut allowed = false;
        for safe_root in &safe_roots {
            if resolved == *safe_root || resolved.starts_with(&format!("{safe_root}/")) {
                allowed = true;
                break;
            }
        }
        if !allowed {
            return Some("safe_root");
        }
    }
    None
}

pub fn is_write_denied(path: &str) -> bool {
    classify_write_denial(path).is_some()
}

pub fn get_write_denied_error(path: &str, verb: &str) -> Option<String> {
    match classify_write_denial(path) {
        None => None,
        Some("safe_root") => {
            let mut roots: Vec<String> = get_safe_write_roots().into_iter().collect();
            roots.sort();
            let sep = if cfg!(windows) { ";" } else { ":" };
            let display = roots.join(sep);
            Some(format!(
                "{verb} denied: '{path}' is outside HERMES_WRITE_SAFE_ROOT ({display}). Unset the variable or add this path's directory prefix."
            ))
        }
        Some(_) => Some(format!(
            "{verb} denied: '{path}' is a protected system/credential file."
        )),
    }
}

/// Common secret-bearing project-local environment file basenames.
pub static BLOCKED_PROJECT_ENV_BASENAMES: once_cell::sync::Lazy<std::collections::HashSet<&'static str>> =
    once_cell::sync::Lazy::new(|| {
        [
            ".env", ".env.local", ".env.development", ".env.production",
            ".env.test", ".env.staging", ".envrc",
        ]
        .into_iter()
        .collect()
    });

/// Error message when a read targets a denied Hermes path, or None.
pub fn get_read_block_error(path: &str) -> Option<String> {
    let resolved = resolve_tolerant(Path::new(path));

    let mut hermes_dirs: Vec<PathBuf> = Vec::new();
    for base in [hermes_home_path(), hermes_root_path()] {
        let real = realpath(&base);
        let b = PathBuf::from(&real);
        if !hermes_dirs.contains(&b) {
            hermes_dirs.push(b);
        }
    }

    // Skills .hub: prompt-injection carriers.
    for hd in &hermes_dirs {
        for sub in [
            "skills/.hub/index-cache",
            "skills/.hub",
        ] {
            let blocked = realpath(&hd.join(sub));
            if resolved.starts_with(&blocked) {
                return Some(format!(
                    "Access denied: {path} is an internal Hermes cache file and cannot be read directly to prevent prompt injection. Use the skills_list or skill_view tools instead."
                ));
            }
        }
    }

    // Credential / secret stores (exact-file matches).
    let credential_file_names = [
        "auth.json",
        "auth.lock",
        ".anthropic_oauth.json",
        ".env",
        "webhook_subscriptions.json",
        "auth/google_oauth.json",
        "cache/bws_cache.json",
    ];
    for hd in &hermes_dirs {
        for name in credential_file_names {
            let blocked = realpath(&hd.join(name));
            if resolved == blocked {
                return Some(format!(
                    "Access denied: {path} is a Hermes credential store and cannot be read directly. Provider tools consume these credentials through internal channels. (Defense-in-depth — not a security boundary; the terminal tool can still bypass.)"
                ));
            }
        }
    }

    // mcp-tokens/ directory prefix.
    for hd in &hermes_dirs {
        let mcp_tokens = realpath(&hd.join("mcp-tokens"));
        if resolved == mcp_tokens {
            return Some(format!(
                "Access denied: {path} is the Hermes MCP token directory and cannot be read directly. (Defense-in-depth — not a security boundary; the terminal tool can still bypass.)"
            ));
        }
        if resolved.starts_with(format!("{mcp_tokens}/")) {
            return Some(format!(
                "Access denied: {path} is a Hermes MCP token file and cannot be read directly. (Defense-in-depth — not a security boundary; the terminal tool can still bypass.)"
            ));
        }
    }

    // Project-local secret-bearing .env files anywhere on disk.
    if let Some(name) = resolved.file_name().and_then(|n| n.to_str()) {
        if BLOCKED_PROJECT_ENV_BASENAMES.contains(name.to_lowercase().as_str()) {
            return Some(format!(
                "Access denied: {path} is a secret-bearing environment file and cannot be read to prevent credential leakage. If you need to check the file structure, read .env.example instead. (Defense-in-depth — not a security boundary; the terminal tool can still bypass.)"
            ));
        }
    }
    None
}

/// Python `Path.resolve()`-tolerant resolution (expanduser + canonicalize or
/// keep the lexical path).
fn resolve_tolerant(path: &Path) -> PathBuf {
    let p = shellexpand::tilde(&path.to_string_lossy()).to_string();
    let p = PathBuf::from(p);
    std::fs::canonicalize(&p).unwrap_or(p)
}

pub fn raise_if_read_blocked(path: &str) -> Result<(), String> {
    if let Some(err) = get_read_block_error(path) {
        Err(err)
    } else {
        Ok(())
    }
}
