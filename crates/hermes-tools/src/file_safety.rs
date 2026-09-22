//! Shared file-safety rules used by tools and ACP shims.
//!
//! PARITY: agent/file_safety.py @ 5d59366 (whole non-compat module).
//!
//! Every guard here is defense-in-depth, NOT a security boundary: the
//! terminal tool runs as the same OS user and can read/write anything.
//! The value is a clear denial for models that respect tool errors plus a
//! visible audit trail.
//!
//! PORT SEAMS (documented divergences):
//! - `_constants_path` (upstream lines 16–23) imports `hermes_constants`
//!   locally and falls back to `~/.hermes` on ANY failure; the Rust getters
//!   are infallible, so the fallback arm has no analog (same inputs → same
//!   paths; upstream would only diverge if the import itself broke).
//! - Upstream tests monkeypatch `_hermes_home_path` / `_hermes_root_path`
//!   per-test; the Rust analog is the HERMES_HOME env var plus
//!   `get_default_hermes_root`'s profiles-parent derivation — exactly the
//!   chain upstream's own `fake_homes` fixture documents
//!   (test_file_safety_session_state.py lines 24–33).
//! - `raise_if_read_blocked` (upstream lines 365–377) swallows INTERNAL
//!   guard crashes (fail-open so local-file loading never breaks). The Rust
//!   `get_read_block_error` is infallible (no exceptions), so only the
//!   real-block → `Err` arm has an analog.
//! - `_resolve_target` returns `Some` always: Python's
//!   `Path.resolve()` exception guard (OSError/RuntimeError → None) has no
//!   failure mode in the tolerant resolver (canonicalize errors fall back
//!   to the lexical path).
//! - The sandbox-mirror path scan runs on POSIX component lists (upstream
//!   `Path.parts`); Windows drive-prefix shapes are out of the tested scope.
//! - `py_repr` covers Python `str.__repr__` quote-selection and common
//!   escapes (`\\`, quotes, `\n`/`\r`/`\t`); exotic control-char `\xNN`
//!   escapes are omitted (test paths never contain them).
//! - The PLUGIN-COMPAT block (upstream lines 470–550:
//!   `PROFILE_SCOPED_AREAS`, `classify_cross_profile_target`,
//!   `get_cross_profile_warning`) is intentionally NOT ported — in-tree
//!   compat pointers are off limits (repo rule), no in-tree caller uses
//!   them at this pin, and no oracle tests cover them. The block's
//!   dependency `_resolve_active_profile_name` (line 380, outside the
//!   block) IS ported — `system_prompt` + `skill_manager_tool` call it.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use hermes_constants::home::{get_default_hermes_root, get_hermes_home};

// ---------------------------------------------------------------------------
// Home / root helpers (upstream lines 16–75)
// ---------------------------------------------------------------------------

/// Active HERMES_HOME (profile-aware).
///
/// PARITY: `_hermes_home_path` (upstream lines 26–28).
pub fn hermes_home_path() -> PathBuf {
    get_hermes_home()
}

/// Hermes root dir (parent of any profile, never per-profile).
///
/// PARITY: `_hermes_root_path` (upstream lines 31–33).
pub fn hermes_root_path() -> PathBuf {
    get_default_hermes_root()
}

/// Python `Path.resolve()`-tolerant resolution (non-strict): canonicalize
/// what exists, keep the rest lexical. Once a component fails, every later
/// component stays on the missing tail (never re-probed as a sibling —
/// `realpath(strict=False)` semantics).
fn resolve_tolerant(path: &Path) -> PathBuf {
    let expanded = shellexpand::tilde(&path.to_string_lossy()).to_string();
    let path = PathBuf::from(expanded);
    if let Ok(c) = std::fs::canonicalize(&path) {
        return c;
    }
    let mut existing = PathBuf::new();
    let mut failed = false;
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    for comp in path.components() {
        if failed {
            rest.push(comp.as_os_str().to_os_string());
            continue;
        }
        existing.push(comp);
        if !existing.exists() {
            failed = true;
            existing.pop();
            rest.push(comp.as_os_str().to_os_string());
        }
    }
    let mut out = if existing.as_os_str().is_empty() {
        path.clone()
    } else {
        std::fs::canonicalize(&existing).unwrap_or(existing)
    };
    for r in rest {
        out.push(r);
    }
    out
}

/// `p.resolve()` for each path, skipping ones that fail to resolve
/// (upstream's `suppress(Exception)`); the tolerant resolver never fails on
/// POSIX, so every path is kept (see PORT SEAMS).
///
/// PARITY: `_resolve_each` (upstream lines 45–51).
fn resolve_each<I>(paths: I) -> Vec<PathBuf>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    paths
        .into_iter()
        .map(|p| resolve_tolerant(p.as_ref()))
        .collect()
}

/// Resolved active HERMES_HOME and global root, deduplicated. Both are
/// checked so credential stores at `<root>/…` stay guarded when running
/// under a profile (`HERMES_HOME = <root>/profiles/<name>`).
///
/// PARITY: `_hermes_dirs` (upstream lines 36–42).
pub fn hermes_dirs() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for p in resolve_each([hermes_home_path().as_path(), hermes_root_path().as_path()]) {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

/// True when `resolved` equals `base` or lies below it (both already
/// resolved) — the str branch: equality or a `base + os.sep` string prefix
/// (upstream lines 54–62, `str` inputs).
fn is_under_str(resolved: &str, base: &str) -> bool {
    resolved == base || resolved.starts_with(&format!("{base}{}", std::path::MAIN_SEPARATOR))
}

/// The `Path` branch of upstream `_is_under`: `relative_to` succeeds for
/// equal-or-below paths (component-wise, never a string-prefix match).
fn is_under_path(resolved: &Path, base: &Path) -> bool {
    resolved.starts_with(base)
}

/// `os.path.realpath(os.path.expanduser(path))` — canonicalize-or-lexical.
fn realpath_home(path: &str) -> String {
    resolve_tolerant(Path::new(path))
        .to_string_lossy()
        .into_owned()
}

/// `os.path.realpath(path)` for an already-built absolute path.
fn realpath(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

/// `(realpath(~), realpath(expanduser(path)))` — the write-guard
/// coordinate pair (upstream lines 72–74).
fn home_and_resolved(path: &str) -> (String, String) {
    (realpath_home("~"), realpath_home(path))
}

// ---------------------------------------------------------------------------
// Windows NT-namespace path guard (upstream lines 77–148)
//
// Pre-approval file accesses reject Windows NT-namespace (`\??\`) paths so
// the remaining unguarded path touches cannot be turned into an NTLM
// credential leak. Merely resolving/touching such a path makes Windows
// initiate SMB authentication to a remote host. The check MUST run on the
// RAW string before any resolve()/realpath() — it is the first check in
// both `get_read_block_error` and the write-denial classifier. Runs on
// every platform (path strings can be relayed toward Windows hosts).
// ---------------------------------------------------------------------------

/// Return true if `path` is a Windows NT-/device-namespace path — checks
/// the raw string only, never resolving (resolution is the leak trigger).
///
/// PARITY: `is_nt_namespace_path` (upstream lines 119–135).
pub fn is_nt_namespace_path(path: &str) -> bool {
    let s = path.replace('/', "\\");
    if s.starts_with("\\??\\") {
        return true;
    }
    if s.starts_with("\\\\.\\") {
        return true;
    }
    if let Some(rest) = s.strip_prefix("\\\\?\\") {
        let upper = rest.to_uppercase();
        if upper.starts_with("UNC\\") || upper.starts_with("GLOBALROOT\\") {
            return true;
        }
    }
    false
}

/// Error message when `path` uses the NT/device namespace, else None.
///
/// PARITY: `get_nt_namespace_error` (upstream lines 138–148).
pub fn get_nt_namespace_error(path: &str, verb: &str) -> Option<String> {
    if !is_nt_namespace_path(path) {
        return None;
    }
    Some(format!(
        "{verb} denied: '{path}' uses a Windows NT/device namespace prefix \
(\\??\\, \\\\.\\, \\\\?\\UNC\\, or GLOBALROOT). These paths bypass \
normal path normalization and can trigger outbound SMB \
authentication (NTLM credential leak) merely by being resolved. \
Use a normal absolute path instead."
    ))
}

// ---------------------------------------------------------------------------
// Write denial (upstream lines 151–280)
// ---------------------------------------------------------------------------

/// Exact sensitive paths that must never be written.
///
/// PARITY: `build_write_denied_paths` (upstream lines 151–181):
/// `~/.ssh/config` is deliberately NOT hard-denied (approval gate instead);
/// secret material under HERMES_HOME covers both the active profile and the
/// global root; auth.json/auth.lock/config.yaml/webhook_subscriptions.json
/// stay read-denied only (#45947).
pub fn build_write_denied_paths(home: &str) -> HashSet<String> {
    // home_files: no `.ssh/config` — it lives behind the approval gate.
    let home_files: &[&[&str]] = &[
        &[".ssh", "authorized_keys"],
        &[".ssh", "id_rsa"],
        &[".ssh", "id_ed25519"],
        &[".netrc"],
        &[".pgpass"],
        &[".npmrc"],
        &[".pypirc"],
        &[".git-credentials"],
    ];
    let hermes_files: &[&str] = &[
        ".env",
        ".anthropic_oauth.json",
        "auth/google_oauth.json",
        "cache/bws_cache.json",
        "cache/bws_cache.enc.json",
    ];
    let mut paths: Vec<String> = home_files
        .iter()
        .map(|f| {
            let mut p = PathBuf::from(home);
            for c in f.iter() {
                p.push(c);
            }
            p.to_string_lossy().into_owned()
        })
        .collect();
    for base in [hermes_home_path(), hermes_root_path()] {
        for f in hermes_files {
            paths.push(base.join(f).to_string_lossy().into_owned());
        }
    }
    paths.extend(
        ["/etc/sudoers", "/etc/passwd", "/etc/shadow"]
            .iter()
            .map(|s| s.to_string()),
    );
    paths.iter().map(|p| realpath_home(p)).collect()
}

/// Sensitive directory prefixes that must never be written.
///
/// PARITY: `build_write_denied_prefixes` (upstream lines 184–191).
pub fn build_write_denied_prefixes(home: &str) -> Vec<String> {
    let home_dirs: &[&[&str]] = &[
        &[".ssh"],
        &[".aws"],
        &[".gnupg"],
        &[".kube"],
        &[".docker"],
        &[".azure"],
        &[".config", "gh"],
        &[".config", "gcloud"],
    ];
    let mut paths: Vec<String> = home_dirs
        .iter()
        .map(|parts| {
            let mut p = PathBuf::from(home);
            for c in parts.iter() {
                p.push(c);
            }
            p.to_string_lossy().into_owned()
        })
        .collect();
    paths.push("/etc/sudoers.d".to_string());
    paths.push("/etc/systemd".to_string());
    paths
        .iter()
        .map(|p| format!("{}{}", realpath_home(p), std::path::MAIN_SEPARATOR))
        .collect()
}

/// Resolved `HERMES_WRITE_SAFE_ROOT` paths (`os.pathsep`-separated).
///
/// PARITY: `get_safe_write_roots` (upstream lines 194–200).
pub fn get_safe_write_roots() -> HashSet<String> {
    let env = std::env::var("HERMES_WRITE_SAFE_ROOT").unwrap_or_default();
    if env.is_empty() {
        return HashSet::new();
    }
    let sep = if cfg!(windows) { ';' } else { ':' };
    let mut roots = HashSet::new();
    for part in env.split(sep).filter(|p| !p.is_empty()) {
        roots.insert(realpath_home(part));
    }
    roots
}

/// Paths that need human APPROVAL to write but are not hard-denied
/// credentials — `~/.ssh/config` (routine to edit, no key bytes, but can
/// carry `ProxyCommand` / `Match exec`).
///
/// PARITY: `build_write_approval_paths` (upstream lines 203–211).
pub fn build_write_approval_paths(home: &str) -> HashSet<String> {
    let mut p = PathBuf::from(home);
    p.push(".ssh");
    p.push("config");
    [realpath(&p)].into_iter().collect()
}

/// HERMES_HOME / root subpaths the agent's generic file tools must not
/// rewrite: session transcripts are application-owned state; mcp-tokens/,
/// pairing/, vault/ and browser-profile/ hold credential material.
/// Control files (auth.json, config.yaml, webhook_subscriptions.json) are
/// deliberately NOT here (#45947): read-denied, but writable on request.
///
/// PARITY: `_HERMES_PROTECTED_SUBPATHS` (upstream line 221).
const HERMES_PROTECTED_SUBPATHS: &[&str] = &[
    "state.db",
    "sessions",
    "mcp-tokens",
    "pairing",
    "vault",
    "browser-profile",
];

/// Return `Some("nt_namespace")`, `Some("credential")`, `Some("safe_root")`,
/// or None if writes are allowed.
///
/// PARITY: `_classify_write_denial` (upstream lines 224–253): the NT check
/// runs on the RAW string first; approval-gated paths return None before
/// the `.ssh/` prefix deny can swallow them; protected subpaths use the
/// equal-or-below check against both hermes dirs.
pub fn classify_write_denial(path: &str) -> Option<&'static str> {
    // NT/device-namespace check on the RAW string, before realpath().
    if is_nt_namespace_path(path) {
        return Some("nt_namespace");
    }
    let (home, resolved) = home_and_resolved(path);

    // Approval-gated paths are allowed at this layer (interactive tools
    // prompt); checked first so the `.ssh/` prefix deny doesn't swallow them.
    if build_write_approval_paths(&home).contains(&resolved) {
        return None;
    }

    if build_write_denied_paths(&home).contains(&resolved) {
        return Some("credential");
    }
    for prefix in build_write_denied_prefixes(&home) {
        if resolved.starts_with(&prefix) {
            return Some("credential");
        }
    }

    for base in hermes_dirs() {
        for sub in HERMES_PROTECTED_SUBPATHS {
            let blocked = realpath(&base.join(sub));
            if is_under_str(&resolved, &blocked) {
                return Some("credential");
            }
        }
    }

    let safe_roots = get_safe_write_roots();
    if !safe_roots.is_empty() && !safe_roots.iter().any(|root| is_under_str(&resolved, root)) {
        return Some("safe_root");
    }
    None
}

/// True if `path` is blocked by the write denylist or safe root.
///
/// PARITY: `is_write_denied` (upstream lines 256–258).
pub fn is_write_denied(path: &str) -> bool {
    classify_write_denial(path).is_some()
}

/// User/model-facing error when writes to `path` are blocked.
///
/// PARITY: `get_write_denied_error` (upstream lines 261–272).
pub fn get_write_denied_error(path: &str, verb: &str) -> Option<String> {
    match classify_write_denial(path)? {
        "safe_root" => {
            let mut roots: Vec<String> = get_safe_write_roots().into_iter().collect();
            roots.sort();
            let sep = if cfg!(windows) { ";" } else { ":" };
            Some(format!(
                "{verb} denied: '{path}' is outside HERMES_WRITE_SAFE_ROOT \
({}). Unset the variable or add this path's directory prefix.",
                roots.join(sep)
            ))
        }
        "nt_namespace" => get_nt_namespace_error(path, verb),
        _ => Some(format!(
            "{verb} denied: '{path}' is a protected system/credential file."
        )),
    }
}

/// True if `path` is approval-gated (`~/.ssh/config`): interactive callers
/// prompt, callers without a channel treat it as a block (fail closed).
///
/// PARITY: `is_write_approval_required` (upstream lines 275–279).
pub fn is_write_approval_required(path: &str) -> bool {
    let (home, resolved) = home_and_resolved(path);
    build_write_approval_paths(&home).contains(&resolved)
}

// ---------------------------------------------------------------------------
// Read denial (upstream lines 282–377)
// ---------------------------------------------------------------------------

/// Secret-bearing project-local env file basenames, blocked anywhere on disk.
///
/// PARITY: `_BLOCKED_PROJECT_ENV_BASENAMES` (upstream lines 283–285).
pub static BLOCKED_PROJECT_ENV_BASENAMES: once_cell::sync::Lazy<HashSet<&'static str>> =
    once_cell::sync::Lazy::new(|| {
        [
            ".env",
            ".env.local",
            ".env.development",
            ".env.production",
            ".env.test",
            ".env.staging",
            ".envrc",
        ]
        .into_iter()
        .collect()
    });

const DID_SUFFIX: &str =
    " (Defense-in-depth — not a security boundary; the terminal tool can still bypass.)";

/// Exact-file credential stores under HERMES_HOME / `<root>`.
///
/// PARITY: `_CREDENTIAL_FILE_NAMES` (upstream lines 294–297).
const CREDENTIAL_FILE_NAMES: &[&str] = &[
    "auth.json",
    "auth.lock",
    ".anthropic_oauth.json",
    ".env",
    "webhook_subscriptions.json",
    "auth/google_oauth.json",
    "cache/bws_cache.json",
];

/// Directory-prefix read denies under HERMES_HOME / `<root>`:
/// (subdir, message for the directory itself, message for a file inside).
///
/// PARITY: `_READ_DENIED_DIRS` (upstream lines 302–313).
const READ_DENIED_DIRS: &[(&str, &str, &str)] = &[
    (
        "mcp-tokens",
        "is the Hermes MCP token directory and cannot be read directly.",
        "is a Hermes MCP token file and cannot be read directly.",
    ),
    (
        "browser-profile",
        "is the Hermes real-profile browser snapshot directory (copied cookies/logins) and cannot be read directly.",
        "is inside the Hermes real-profile browser snapshot (copied cookies/logins) and cannot be read directly.",
    ),
    (
        "vault",
        "is the Hermes credential vault directory and cannot be read directly (secrets are filled server-side by browser_vault_fill).",
        "is inside the Hermes credential vault (encrypted secrets + local key) and cannot be read directly (browser_vault_fill resolves them server-side).",
    ),
];

/// Error message when a read targets a denied Hermes path, or None.
///
/// PARITY: `get_read_block_error` (upstream lines 316–362): the NT check
/// runs on the RAW string before any resolve; then hub cache (no suffix),
/// exact credential stores, the directory-prefix denies (dir vs file
/// message + suffix), and finally the env-basename guard (suffix).
/// Callers resolving relative paths against a non-process cwd MUST pass an
/// absolute path (resolution anchors at the process cwd).
pub fn get_read_block_error(path: &str) -> Option<String> {
    // NT/device-namespace check on the RAW string, before resolve().
    if let Some(nt_error) = get_nt_namespace_error(path, "Read") {
        return Some(nt_error);
    }
    let resolved = resolve_tolerant(Path::new(path));
    let hermes_dirs = hermes_dirs();

    let reason: Option<String> = if hermes_dirs
        .iter()
        .any(|hd| is_under_path(&resolved, &hd.join("skills").join(".hub")))
    {
        Some(
            "is an internal Hermes cache file and cannot be read directly to prevent \
prompt injection. Use the skills_list or skill_view tools instead."
                .to_string(),
        )
    } else if hermes_dirs.iter().any(|hd| {
        CREDENTIAL_FILE_NAMES
            .iter()
            .any(|name| resolved == resolve_tolerant(&hd.join(name)))
    }) {
        Some(
            "is a Hermes credential store and cannot be read directly. Provider tools \
consume these credentials through internal channels."
                .to_string()
                + DID_SUFFIX,
        )
    } else {
        let mut dir_reason: Option<String> = None;
        'dirs: for (subdir, dir_msg, file_msg) in READ_DENIED_DIRS {
            for blocked_dir in resolve_each(hermes_dirs.iter().map(|hd| hd.join(subdir))) {
                if is_under_path(&resolved, &blocked_dir) {
                    let msg = if resolved == blocked_dir {
                        *dir_msg
                    } else {
                        *file_msg
                    };
                    dir_reason = Some(format!("{msg}{DID_SUFFIX}"));
                    break 'dirs;
                }
            }
        }
        dir_reason.or_else(|| {
            resolved
                .file_name()
                .and_then(|n| n.to_str())
                .filter(|n| BLOCKED_PROJECT_ENV_BASENAMES.contains(n.to_lowercase().as_str()))
                .map(|_| {
                    "is a secret-bearing environment file and cannot be read to prevent \
credential leakage. If you need to check the file structure, read .env.example instead."
                        .to_string()
                        + DID_SUFFIX
                })
        })
    };
    reason.map(|r| format!("Access denied: {path} {r}"))
}

/// Raise `Err(message)` if `path` is a denied Hermes read.
///
/// Shared chokepoint for provider input-loading sites (e.g. image-gen local
/// paths). PARITY: `raise_if_read_blocked` (upstream lines 365–377) —
/// upstream fail-opens on INTERNAL guard crashes (never breaks local-file
/// loading); see PORT SEAMS (the Rust guard is infallible).
pub fn raise_if_read_blocked(path: &str) -> Result<(), String> {
    match get_read_block_error(path) {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Active profile resolver (upstream lines 380–387)
// ---------------------------------------------------------------------------

/// Active profile name from HERMES_HOME: `~/.hermes` → `"default"`,
/// `~/.hermes/profiles/X` → `"X"`; `"default"` on any resolution failure.
///
/// PARITY: `_resolve_active_profile_name` (upstream lines 380–387) — called
/// by `system_prompt` (active-profile hint) and `skill_manager_tool`.
/// Rust getters are infallible; the Python `except` arm maps to the
/// strip-prefix miss (see PORT SEAMS).
pub fn resolve_active_profile_name() -> String {
    let home = resolve_tolerant(&hermes_home_path());
    let root = resolve_tolerant(&hermes_root_path());
    match home.strip_prefix(root.join("profiles")) {
        Ok(rel) => rel
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .unwrap_or_else(|| "default".to_string()),
        Err(_) => "default".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Sandbox-mirror write guard (upstream lines 390–467)
//
// Non-local terminal backends bind a sandbox-local dir to the container's
// $HOME: <HERMES_HOME>/profiles/<name>/sandboxes/<backend>/<task>/home/.hermes/…
// A host-side write there lands on a mirror the host never reads: silent
// success, divergent copies. Path-shape-only detection, independent of the
// active profile.
// ---------------------------------------------------------------------------

const SANDBOX_MIRROR_WARNING: &str = "Sandbox-mirror write blocked by soft guard: {target_path} \
sits under {mirror_root!r}, which is {body} \
Use the host-side tool for authoritative state (e.g. ``memory`` for memories), \
or address the host path directly. To bypass {bypass} with ``cross_profile=True``. \
(Defense-in-depth — not a security boundary; the terminal tool can still bypass.)";

/// Python `str.__repr__` quote-selection + common escapes (see PORT SEAMS
/// for the omitted exotic `\xNN` arm).
fn py_repr(s: &str) -> String {
    let has_single = s.contains('\'');
    let has_double = s.contains('"');
    let use_double = has_single && !has_double;
    let mut out = String::with_capacity(s.len() + 2);
    out.push(if use_double { '"' } else { '\'' });
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '"' if use_double => out.push_str("\\\""),
            '\'' if !use_double => out.push_str("\\'"),
            other => out.push(other),
        }
    }
    out.push(if use_double { '"' } else { '\'' });
    out
}

/// Join Python `Path.parts` tuples back into a path string (the first
/// component of an absolute posix path is `"/"`, which `Path` treats as the
/// root rather than a joined segment).
fn join_parts(parts: &[String]) -> String {
    if parts.first().map(|s| s == "/").unwrap_or(false) {
        format!("/{}", parts[1..].join("/"))
    } else {
        parts.join("/")
    }
}

/// Resolved classify result shared by both mirror guards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorInfo {
    pub target_path: String,
    pub mirror_root: String,
    pub inner_path: String,
}

/// `Path(expanduser(path)).resolve()`, or None when resolution fails —
/// the tolerant resolver is total (see PORT SEAMS).
fn resolve_target(path: &str) -> Option<PathBuf> {
    Some(resolve_tolerant(Path::new(path)))
}

/// Classify a write target as a sandbox-mirror of authoritative Hermes
/// state: None for non-mirror paths, else the resolved target, the
/// `…/home/.hermes` mirror root, and what the agent meant on the host.
///
/// PARITY: `classify_sandbox_mirror_target` (upstream lines 411–426):
/// needs at least `sandboxes/<backend>/<task>/home/.hermes/<thing>` —
/// backend names are unconstrained (shape is what matters).
pub fn classify_sandbox_mirror_target(path: &str) -> Option<MirrorInfo> {
    let target = resolve_target(path)?;
    let parts: Vec<String> = target
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    // inner_idx = the `.hermes` part: sandboxes(i), backend(i+1),
    // task(i+2), home(i+3), .hermes(i+4), <thing>(i+5) — needs i+5 < len.
    let inner_idx = parts
        .iter()
        .enumerate()
        .position(|(i, part)| {
            part == "sandboxes"
                && i + 5 < parts.len()
                && parts[i + 3] == "home"
                && parts[i + 4] == ".hermes"
        })
        .map(|i| i + 4)?;
    let inner = if inner_idx + 1 < parts.len() {
        join_parts(&parts[inner_idx + 1..])
    } else {
        String::new()
    };
    Some(MirrorInfo {
        target_path: target.to_string_lossy().into_owned(),
        mirror_root: join_parts(&parts[..=inner_idx]),
        inner_path: inner,
    })
}

/// Render the warning template for a classify result (body may use
/// `{inner_path!r}`).
///
/// PARITY: `_mirror_warning` (upstream lines 429–433).
fn mirror_warning(info: Option<&MirrorInfo>, body: &str, bypass: &str) -> Option<String> {
    let info = info?;
    let body = body.replace("{inner_path!r}", &py_repr(&info.inner_path));
    Some(
        SANDBOX_MIRROR_WARNING
            .replace("{target_path}", &info.target_path)
            .replace("{mirror_root!r}", &py_repr(&info.mirror_root))
            .replace("{body}", &body)
            .replace("{bypass}", bypass),
    )
}

/// Model-facing soft-guard warning when `path` lands in a sandbox mirror,
/// else None; the caller surfaces it as a tool-result error and
/// `cross_profile=True` bypasses.
///
/// PARITY: `get_sandbox_mirror_warning` (upstream lines 436–445).
pub fn get_sandbox_mirror_warning(path: &str) -> Option<String> {
    mirror_warning(
        classify_sandbox_mirror_target(path).as_ref(),
        "a per-task mirror created by a non-local terminal backend (docker/daytona/etc.). \
Writes here land on a copy that the host Hermes process never reads — the \
authoritative file is likely {inner_path!r} under the real HERMES_HOME.",
        "this guard after explicit user direction, retry the call",
    )
}

// ---------------------------------------------------------------------------
// Container-mirror write guard (upstream lines 448–467)
// ---------------------------------------------------------------------------

/// Classify a write target as a container-side sandbox mirror. Inside the
/// container the bind mount strips the `sandboxes/` prefix (the agent sees
/// plain `/root/.hermes/…`), so the caller supplies `mirror_prefix` once it
/// knows file tools run in a docker sandbox. None without a prefix or
/// outside it.
///
/// PARITY: `classify_container_mirror_target` (upstream lines 448–456).
pub fn classify_container_mirror_target(
    path: &str,
    mirror_prefix: Option<&str>,
) -> Option<MirrorInfo> {
    let target = resolve_target(path)?;
    let mirror = mirror_prefix.and_then(resolve_target)?;
    if !is_under_path(&target, &mirror) {
        return None;
    }
    let inner = target
        .strip_prefix(&mirror)
        .ok()?
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    Some(MirrorInfo {
        target_path: target.to_string_lossy().into_owned(),
        mirror_root: mirror.to_string_lossy().into_owned(),
        inner_path: inner,
    })
}

/// Model-facing soft-guard warning when `path` lands in the container's
/// mirror, else None.
///
/// PARITY: `get_container_mirror_warning` (upstream lines 459–467).
pub fn get_container_mirror_warning(path: &str, mirror_prefix: Option<&str>) -> Option<String> {
    mirror_warning(
        classify_container_mirror_target(path, mirror_prefix).as_ref(),
        "the container's bind-mounted home — a per-task mirror that the host Hermes \
process never reads. The authoritative file is {inner_path!r} under \
the real HERMES_HOME.",
        "after explicit user direction, retry",
    )
}
