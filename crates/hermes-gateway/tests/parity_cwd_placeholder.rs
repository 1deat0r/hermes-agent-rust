// Tier: unit — mirrors `tests/gateway/test_cwd_placeholder.py` (both oracle
// cases) plus the source branches the oracle leaves unexercised: explicit
// configured paths, backend normalization, the docker mount-on arm with
// placeholder/empty host paths, non-local backends, and `_truthy_env`.

use hermes_gateway::cwd_placeholder::{resolve_placeholder_terminal_cwd, truthy_env};

fn resolve(
    configured_cwd: &str,
    terminal_backend: &str,
    messaging_cwd: Option<&str>,
    docker_mount_cwd_to_workspace: bool,
    home_fallback: &str,
) -> Option<String> {
    resolve_placeholder_terminal_cwd(
        configured_cwd,
        terminal_backend,
        messaging_cwd,
        docker_mount_cwd_to_workspace,
        home_fallback,
    )
}

// Oracle: test_local_placeholder_uses_messaging_cwd.
#[test]
fn local_placeholder_uses_messaging_cwd() {
    assert_eq!(
        resolve(
            ".",
            "local",
            Some("/home/user/project"),
            false,
            "/home/user"
        ),
        Some("/home/user/project".to_string())
    );
}

// Oracle: test_docker_placeholder_mount_off_unset.
#[test]
fn docker_placeholder_with_mount_off_stays_unset() {
    assert_eq!(
        resolve(".", "docker", Some("/home/user"), false, "/home/user"),
        None
    );
}

// An explicit configured path wins over every arm, unchanged.
#[test]
fn explicit_configured_cwd_is_returned_verbatim() {
    assert_eq!(
        resolve(
            "/opt/explicit",
            "docker",
            Some("/host/project"),
            true,
            "/home/user"
        ),
        Some("/opt/explicit".to_string())
    );
    assert_eq!(
        resolve("/opt/explicit", "local", None, false, "/home/user"),
        Some("/opt/explicit".to_string())
    );
}

// Every placeholder spelling falls through, and an empty configured cwd is
// treated like a placeholder.
#[test]
fn all_placeholder_forms_fall_through() {
    for placeholder in ["", ".", "auto", "cwd"] {
        assert_eq!(
            resolve(
                placeholder,
                "local",
                Some("/messaging"),
                false,
                "/home/user"
            ),
            Some("/messaging".to_string()),
            "{placeholder:?}"
        );
    }
}

// `(terminal_backend or "local").strip().lower()`.
#[test]
fn backend_is_normalized_and_defaults_to_local() {
    assert_eq!(
        resolve(".", "  Local ", None, false, "/home/user"),
        Some("/home/user".to_string())
    );
    assert_eq!(
        resolve(".", "", None, true, "/home/user"),
        Some("/home/user".to_string())
    );
    // Docker case-insensitively, with the mount gate still applying.
    assert_eq!(
        resolve(".", "DOCKER", Some("/host/project"), true, "/home/user"),
        Some("/host/project".to_string())
    );
}

// Docker + mount on: the host messaging path feeds terminal_tool's
// /workspace mapping, but only when it is a real host path.
#[test]
fn docker_mount_on_requires_a_real_host_messaging_path() {
    assert_eq!(
        resolve(".", "docker", Some("/host/project"), true, "/home/user"),
        Some("/host/project".to_string())
    );
    // A placeholder messaging cwd is not a host path signal.
    assert_eq!(
        resolve(".", "docker", Some("auto"), true, "/home/user"),
        None
    );
    assert_eq!(resolve(".", "docker", Some(""), true, "/home/user"), None);
    assert_eq!(resolve(".", "docker", None, true, "/home/user"), None);
}

// Other non-local backends never map a placeholder, mount or not.
#[test]
fn other_backends_stay_unset() {
    for backend in ["tmux", "ssh", "serial"] {
        assert_eq!(
            resolve(".", backend, Some("/host/project"), true, "/home/user"),
            None,
            "{backend}"
        );
    }
}

// Local strips surrounding whitespace from the messaging cwd before
// preferring it; `_truthy_env` accepts only true/1/yes case-insensitively.
#[test]
fn local_strips_messaging_cwd_and_truthy_env_parses_flags() {
    assert_eq!(
        resolve(".", "local", Some("  /m  "), false, "/home/user"),
        Some("/m".to_string())
    );
    assert_eq!(
        resolve(".", "local", Some("   "), false, "/home/user"),
        Some("/home/user".to_string())
    );

    for value in [Some("true"), Some("  YES "), Some("1")] {
        assert!(truthy_env(value), "{value:?}");
    }
    for value in [None, Some(""), Some("0"), Some("false"), Some("on")] {
        assert!(!truthy_env(value), "{value:?}");
    }
}
