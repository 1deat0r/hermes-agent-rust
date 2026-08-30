//! Resolve gateway `terminal.cwd` placeholder values to `TERMINAL_CWD`.
//!
//! PARITY: `gateway/cwd_placeholder.py` @ b9aa928 (whole module, lines
//! 1-50).
//!
//! When `terminal.cwd` is unset or a placeholder (`.`, `auto`, `cwd`), the
//! gateway must not blindly map host `Path.home()` into container backends.
//! Docker with workspace mounting still needs an explicit host path signal
//! (`MESSAGING_CWD` or an absolute config path) for `terminal_tool` to map
//! `/host/project` → `/workspace`.

/// PARITY: `CWD_PLACEHOLDERS` (upstream line 12), a frozenset upstream —
/// membership only, so slice order is not contract.
pub const CWD_PLACEHOLDERS: [&str; 3] = [".", "auto", "cwd"];

fn is_placeholder(value: &str) -> bool {
    CWD_PLACEHOLDERS.contains(&value)
}

/// PARITY: `_truthy_env` (upstream lines 15-16). Module-private upstream;
/// public here so the parity suite can pin its flag grammar.
pub fn truthy_env(value: Option<&str>) -> bool {
    let value = value.unwrap_or_default().trim().to_lowercase();
    matches!(value.as_str(), "true" | "1" | "yes")
}

/// Return the `TERMINAL_CWD` value to set, or `None` to leave it unset.
///
/// PARITY: `resolve_placeholder_terminal_cwd` (upstream lines 19-49).
///
/// Cases:
///   - **local** + placeholder → `MESSAGING_CWD` or `home_fallback`
///   - **docker** + placeholder + mount on + host `MESSAGING_CWD` → host
///     path (for `terminal_tool` `/workspace` mapping)
///   - **docker** + placeholder + mount off → `None` (sandbox default)
///   - other non-local backends + placeholder → `None`
pub fn resolve_placeholder_terminal_cwd(
    configured_cwd: &str,
    terminal_backend: &str,
    messaging_cwd: Option<&str>,
    docker_mount_cwd_to_workspace: bool,
    home_fallback: &str,
) -> Option<String> {
    if !configured_cwd.is_empty() && !is_placeholder(configured_cwd) {
        return Some(configured_cwd.to_string());
    }

    let backend = if terminal_backend.is_empty() {
        "local"
    } else {
        terminal_backend
    }
    .trim()
    .to_lowercase();

    if backend == "local" {
        let messaging = messaging_cwd.unwrap_or_default().trim();
        return Some(if messaging.is_empty() {
            home_fallback.to_string()
        } else {
            messaging.to_string()
        });
    }

    if backend == "docker" && docker_mount_cwd_to_workspace {
        let messaging = messaging_cwd.unwrap_or_default().trim();
        if !messaging.is_empty() && !is_placeholder(messaging) {
            return Some(messaging.to_string());
        }
    }

    None
}
