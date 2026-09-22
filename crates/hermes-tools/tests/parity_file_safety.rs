//! Parity oracles for the path-safety guards (pure-path cases), mirroring
//! upstream tests/agent/test_file_safety.py +
//! tests/agent/test_nt_namespace_guard.py (file_safety chokepoint part) +
//! tests/agent/test_file_safety_sandbox_mirror.py +
//! tests/agent/test_file_safety_container_mirror.py @ 5d59366. Env-mutating
//! cases (HERMES_HOME / HERMES_WRITE_SAFE_ROOT / profile layouts) live in
//! the isolated parity_file_safety_env.rs binary. The upstream NT test's
//! second half (file-tool entry rejections) belongs to the file-tools /
//! tool-executor rows and is covered there.
//! Tier: `unit`.

use hermes_tools::file_safety::{
    build_write_approval_paths, classify_container_mirror_target, classify_sandbox_mirror_target,
    classify_write_denial, get_container_mirror_warning, get_nt_namespace_error,
    get_read_block_error, get_sandbox_mirror_warning, get_write_denied_error, is_nt_namespace_path,
    is_write_approval_required, is_write_denied,
};

// ── Project-local .env blocking (upstream TestEnvFileReadBlocking) ──────

#[test]
fn blocked_env_basenames() {
    for basename in [
        ".env",
        ".env.local",
        ".env.development",
        ".env.production",
        ".env.test",
        ".env.staging",
        ".envrc",
    ] {
        let path = format!("/tmp/project/{basename}");
        let error = get_read_block_error(&path);
        assert!(error.is_some(), "{basename} should be blocked");
        let e = error.unwrap();
        assert!(e.contains("Access denied"));
        let lower = e.to_lowercase();
        assert!(
            lower.contains("secret-bearing") || lower.contains("environment file"),
            "message: {e}"
        );
    }
}

#[test]
fn blocked_env_basenames_case_insensitive() {
    for basename in [".ENV", ".Env.Local", ".ENV.PRODUCTION", ".ENVRC"] {
        let path = format!("/tmp/project/{basename}");
        let error = get_read_block_error(&path);
        assert!(error.is_some(), "{basename} should be blocked");
        let e = error.unwrap().to_lowercase();
        assert!(e.contains("environment file"), "message: {e}");
    }
}

#[test]
fn allowed_env_example() {
    // .env.example is documentation, not a secret.
    assert!(get_read_block_error("/tmp/project/.env.example").is_none());
}

// ── Write denial (upstream test_file_safety_write_credentials shape) ────

#[test]
fn write_denies_credential_paths() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    assert_eq!(
        classify_write_denial(&format!("{home}/.ssh/id_rsa")),
        Some("credential")
    );
    // ORACLE @ 5d59366: `~/.ssh/config` is deliberately NOT hard-denied —
    // it moved to the approval gate (`build_write_approval_paths`); the
    // `.ssh/` prefix deny must not swallow it (approval is checked first).
    assert_eq!(classify_write_denial(&format!("{home}/.ssh/config")), None);
    assert!(is_write_approval_required(&format!("{home}/.ssh/config")));
    assert_eq!(classify_write_denial("/etc/passwd"), Some("credential"));
    assert_eq!(classify_write_denial("/etc/sudoers"), Some("credential"));
    assert!(is_write_denied(&format!("{home}/.netrc")));
    // A key file is denied outright, not approval-gated.
    assert!(!is_write_approval_required(&format!("{home}/.ssh/id_rsa")));
    // Approval-path construction matches the upstream shape.
    let approval = build_write_approval_paths(&real_home());
    assert!(approval.contains(&realpath_of(&format!("{home}/.ssh/config"))));
}

#[test]
fn write_allows_regular_paths() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    assert!(classify_write_denial(&format!("{home}/code/notes.txt")).is_none());
    assert!(classify_write_denial("/tmp/scratch/data.txt").is_none());
    // Control files stay writable (#45947) — outside HERMES_HOME they are
    // never credential-denied (hermes-home-scoped cases live in the env binary).
    assert!(classify_write_denial("/tmp/project/auth.json").is_none());
    assert!(classify_write_denial("/tmp/project/vault/vault.key").is_none());
    assert!(classify_write_denial("/tmp/project/cache/bws_cache.json").is_none());
}

fn real_home() -> String {
    let p = std::fs::canonicalize(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
        .unwrap_or_default();
    p.to_string_lossy().into_owned()
}

fn realpath_of(path: &str) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| std::path::PathBuf::from(path))
        .to_string_lossy()
        .into_owned()
}

// ── NT namespace guard (upstream test_nt_namespace_guard, chokepoint half)

const NT_BLOCKED: &[&str] = &[
    "\\??\\UNC\\attacker.example\\share\\x",
    "\\??\\C:\\Windows\\System32\\config\\SAM",
    "/??/UNC/attacker.example/share/x",
    "\\\\.\\PhysicalDrive0",
    "\\\\.\\pipe\\evil",
    "//./pipe/evil",
    "\\\\?\\UNC\\attacker.example\\share\\x",
    "\\\\?\\unc\\attacker.example\\share\\x",
    "\\\\?\\GLOBALROOT\\Device\\HarddiskVolume1\\x",
    "//?/UNC/attacker.example/share/x",
];

const NT_ALLOWED: &[&str] = &[
    "/tmp/test.py",
    "C:\\Users\\me\\notes.txt",
    "~/projects/readme.md",
    "relative/path.txt",
    "\\\\?\\C:\\Users\\me\\notes.txt",
    "\\\\server\\share\\file.txt",
    "//server/share/file.txt",
    "/tmp/??/weird-dir/file",
];

#[test]
fn nt_namespace_predicate_and_both_chokepoint_classifiers() {
    // ORACLE: `test_predicate_and_both_chokepoints` — fires on the RAW
    // string for read + write; allowed paths never carry the NT text.
    for p in NT_BLOCKED {
        assert!(is_nt_namespace_path(p), "{p}");
        let read = get_read_block_error(p).unwrap_or_default();
        assert!(read.contains("NT/device namespace"), "{p}: {read}");
        let write = get_write_denied_error(p, "Write").unwrap_or_default();
        assert!(write.contains("NT/device namespace"), "{p}: {write}");
        assert!(write.contains("Write denied"), "{p}: {write}");
    }
    for p in NT_ALLOWED {
        assert!(!is_nt_namespace_path(p), "{p}");
        let read = get_read_block_error(p).unwrap_or_default();
        assert!(!read.contains("NT/device namespace"), "{p}: {read}");
        let write = get_write_denied_error(p, "Write").unwrap_or_default();
        assert!(!write.contains("NT/device namespace"), "{p}: {write}");
    }
}

#[test]
fn nt_namespace_error_wording_and_verb() {
    // Oracle shape: custom verb rides the message; non-NT path → None.
    let msg = get_nt_namespace_error("\\??\\x", "Read").expect("nt");
    assert!(msg.starts_with("Read denied:"));
    assert!(msg.contains("NT/device namespace"));
    assert!(msg.contains("NTLM credential leak"));
    assert!(get_nt_namespace_error("/tmp/x", "Write").is_none());
}

// ── Sandbox-mirror guard (upstream test_file_safety_sandbox_mirror) ─────

#[test]
fn sandbox_mirror_classifies_docker_shape() {
    // ORACLE: `test_docker_mirror_soul_md_classified` — the exact #32049 shape.
    let base = tempfile::TempDir::new().unwrap();
    let target = base
        .path()
        .join("profiles/group1/sandboxes/docker/default/home/.hermes/profiles/group1/SOUL.md");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "# mirror copy\n").unwrap();
    let resolved_target = std::fs::canonicalize(&target).unwrap();

    let result = classify_sandbox_mirror_target(&target.to_string_lossy())
        .expect("mirror shape must classify");
    assert_eq!(
        result.target_path,
        resolved_target.to_string_lossy().into_owned()
    );
    assert!(
        result
            .mirror_root
            .ends_with("sandboxes/docker/default/home/.hermes"),
        "mirror_root={}",
        result.mirror_root
    );
    assert_eq!(result.inner_path, "profiles/group1/SOUL.md");
}

#[test]
fn sandbox_mirror_is_backend_agnostic() {
    // ORACLE: `test_other_backends_and_inner_files_match` (parametrized).
    for (backend, inner) in [
        ("docker", "profiles/coder/memories/MEMORY.md"),
        ("daytona", "profiles/default/cron/jobs.json"),
        ("podman", ".env"),
    ] {
        let base = tempfile::TempDir::new().unwrap();
        let target = base
            .path()
            .join(format!("sandboxes/{backend}/task-42/home/.hermes/{inner}"));
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "x").unwrap();

        let result =
            classify_sandbox_mirror_target(&target.to_string_lossy()).expect("must classify");
        assert_eq!(result.inner_path, inner, "{backend}");
        assert!(
            result.mirror_root.contains(backend),
            "mirror_root={}",
            result.mirror_root
        );
    }
}

#[test]
fn sandbox_mirror_warning_contents() {
    // ORACLE: TestGetSandboxMirrorWarning — None for non-mirror; mirror
    // warning names the root, the inner path, the bypass kwarg, and the
    // defense-in-depth self-documentation.
    let base = tempfile::TempDir::new().unwrap();
    let real = base.path().join(".hermes/profiles/group1/SOUL.md");
    std::fs::create_dir_all(real.parent().unwrap()).unwrap();
    std::fs::write(&real, "# real SOUL\n").unwrap();
    assert!(get_sandbox_mirror_warning(&real.to_string_lossy()).is_none());

    let mirror = base
        .path()
        .join("profiles/group1/sandboxes/docker/default/home/.hermes/profiles/group1/SOUL.md");
    std::fs::create_dir_all(mirror.parent().unwrap()).unwrap();
    std::fs::write(&mirror, "# mirror copy\n").unwrap();
    let warn = get_sandbox_mirror_warning(&mirror.to_string_lossy()).expect("mirror warning");
    assert!(
        warn.contains("sandboxes/docker/default/home/.hermes"),
        "{warn}"
    );
    assert!(warn.contains("profiles/group1/SOUL.md"), "{warn}");
    assert!(warn.contains("cross_profile=True"), "{warn}");
    assert!(
        warn.to_lowercase().contains("not a security boundary"),
        "{warn}"
    );
    assert!(warn.contains("Sandbox-mirror write blocked"), "{warn}");
}

#[test]
fn sandbox_mirror_fires_on_same_profile_shape() {
    // ORACLE: `test_same_profile_mirror_still_flagged` — pure shape
    // detection, independent of the active profile.
    let base = tempfile::TempDir::new().unwrap();
    let target = base
        .path()
        .join("profiles/group1/sandboxes/docker/default/home/.hermes/profiles/group1/SOUL.md");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "x").unwrap();
    assert!(classify_sandbox_mirror_target(&target.to_string_lossy()).is_some());
    // Probe-pinned: the shape must continue PAST `.hermes` (needs the
    // `<thing>` segment) — a path ending AT `.hermes` is not a mirror.
    let at_hermes = base.path().join("sandboxes/docker/task/home/.hermes");
    std::fs::create_dir_all(&at_hermes).unwrap();
    assert!(classify_sandbox_mirror_target(&at_hermes.to_string_lossy()).is_none());
    // Probe-pinned: spaces in the inner path survive verbatim.
    let spaced = base.path().join("sandboxes/d/t/home/.hermes/a b");
    std::fs::create_dir_all(spaced.parent().unwrap()).unwrap();
    std::fs::write(&spaced, "x").unwrap();
    let info = classify_sandbox_mirror_target(&spaced.to_string_lossy()).expect("classified");
    assert_eq!(info.inner_path, "a b");
}

// ── Container-mirror guard (upstream test_file_safety_container_mirror) ─

#[test]
fn container_mirror_classifies_with_context() {
    // ORACLE: `test_catches_soul_md_with_context` — bind-mount-stripped
    // `/root/.hermes/…` shape needs the caller-supplied prefix.
    let result = classify_container_mirror_target(
        "/root/.hermes/profiles/group1/SOUL.md",
        Some("/root/.hermes"),
    )
    .expect("classified");
    assert!(
        result
            .mirror_root
            .replace('\\', "/")
            .ends_with("root/.hermes"),
        "mirror_root={}",
        result.mirror_root
    );
    assert_eq!(result.inner_path, "profiles/group1/SOUL.md");

    for inner in ["SOUL.md", "memories/MEMORY.md"] {
        let path = format!("/root/.hermes/{inner}");
        let result =
            classify_container_mirror_target(&path, Some("/root/.hermes")).expect("classified");
        assert_eq!(result.inner_path, inner);
    }
}

#[test]
fn container_mirror_warning_names_inner_and_bypass() {
    // ORACLE: `test_warning_names_inner_path_and_bypass`.
    let warn = get_container_mirror_warning(
        "/root/.hermes/profiles/group1/SOUL.md",
        Some("/root/.hermes"),
    )
    .expect("warning");
    assert!(warn.contains("profiles/group1/SOUL.md"), "{warn}");
    assert!(warn.contains("cross_profile=True"), "{warn}");
    assert!(
        warn.to_lowercase().contains("not a security boundary"),
        "{warn}"
    );
}

#[test]
fn container_mirror_orthogonal_without_prefix() {
    // ORACLE: `test_inner_container_path_caught_by_context_guard` — no
    // prefix → None; with prefix → classified.
    let path = "/root/.hermes/profiles/group1/SOUL.md";
    assert!(classify_container_mirror_target(path, None).is_none());
    assert!(classify_container_mirror_target(path, Some("/root/.hermes")).is_some());
}
