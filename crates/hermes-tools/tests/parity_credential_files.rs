//! Parity oracles for tools/credential_files.py, mirroring upstream
//! tests/tools/test_credential_files.py @ b9aa928 (config-based cases use the
//! set_terminal_credential_files seam since the config crate is P3).

use hermes_tools::credential_files::{
    clear_credential_files, get_cache_directory_mounts, get_credential_file_mounts,
    get_skills_directory_mount, iter_cache_files, iter_skills_files,
    map_cache_path_to_container, register_credential_file, register_credential_files,
    reset_terminal_credential_files_for_tests, set_terminal_credential_files,
};
use std::path::PathBuf;
use serde_json::json;

fn with_home(tmp: &std::path::Path, f: impl FnOnce()) {
    let token = hermes_constants::home::set_hermes_home_override(Some(tmp));
    reset_terminal_credential_files_for_tests();
    clear_credential_files();
    f();
    clear_credential_files();
    reset_terminal_credential_files_for_tests();
    hermes_constants::home::reset_hermes_home_override(token);
}

fn tmp_home(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("hermes_cred_test_{}_{}", std::process::id(), name))
}

fn init_home(name: &str) -> PathBuf {
    let h = tmp_home(name);
    let _ = std::fs::remove_dir_all(&h);
    std::fs::create_dir_all(&h).unwrap();
    h
}

#[test]
fn dict_with_path_key() {
    let home = init_home("dict_path");
    std::fs::write(home.join("token.json"), "{}").unwrap();
    with_home(&home, || {
        let missing = register_credential_files(&[json!({"path": "token.json"})], "/root/.hermes");
        assert_eq!(missing.len(), 0);
        let mounts = get_credential_file_mounts();
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].host_path, home.join("token.json").to_string_lossy());
        assert_eq!(mounts[0].container_path, "/root/.hermes/token.json");
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn path_takes_precedence_over_name() {
    let home = init_home("path_prec");
    std::fs::write(home.join("real.json"), "{}").unwrap();
    with_home(&home, || {
        let missing = register_credential_files(
            &[json!({"path": "real.json", "name": "wrong.json"})],
            "/root/.hermes",
        );
        assert_eq!(missing.len(), 0);
        let mounts = get_credential_file_mounts();
        assert!(mounts[0].container_path.contains("real.json"));
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn skills_mount_when_dir_exists() {
    let home = init_home("skills_mount");
    let sdir = home.join("skills/test-skill");
    std::fs::create_dir_all(&sdir).unwrap();
    std::fs::write(sdir.join("SKILL.md"), "# test").unwrap();
    with_home(&home, || {
        let mounts = get_skills_directory_mount("/root/.hermes");
        assert!(!mounts.is_empty());
        assert_eq!(mounts[0].host_path, home.join("skills").to_string_lossy());
        assert_eq!(mounts[0].container_path, "/root/.hermes/skills");
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn skills_custom_container_base() {
    let home = init_home("skills_base");
    std::fs::create_dir_all(home.join("skills")).unwrap();
    with_home(&home, || {
        let mounts = get_skills_directory_mount("/home/user/.hermes");
        assert_eq!(mounts[0].container_path, "/home/user/.hermes/skills");
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[cfg(unix)]
#[test]
fn skills_symlinks_are_sanitized() {
    let home = init_home("skills_symlink");
    let sdir = home.join("skills");
    std::fs::create_dir_all(&sdir).unwrap();
    std::fs::write(sdir.join("legit.md"), "# real skill").unwrap();
    let secret = home.join("secret.txt");
    std::fs::write(&secret, "TOP SECRET").unwrap();
    std::os::unix::fs::symlink(&secret, sdir.join("evil_link")).unwrap();

    with_home(&home, || {
        let mounts = get_skills_directory_mount("/root/.hermes");
        let mount = &mounts[0];
        let safe = std::path::Path::new(&mount.host_path);
        assert_ne!(safe, sdir.as_path(), "sanitized copy must not be the original");
        assert!(safe.join("legit.md").exists());
        assert_eq!(std::fs::read_to_string(safe.join("legit.md")).unwrap(), "# real skill");
        assert!(!safe.join("evil_link").exists(), "symlink must not be copied");
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[cfg(unix)]
#[test]
fn skills_no_symlinks_returns_original() {
    let home = init_home("skills_no_sym");
    let sdir = home.join("skills");
    std::fs::create_dir_all(&sdir).unwrap();
    std::fs::write(sdir.join("skill.md"), "ok").unwrap();
    with_home(&home, || {
        let mounts = get_skills_directory_mount("/root/.hermes");
        assert_eq!(mounts[0].host_path, sdir.to_string_lossy());
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[cfg(unix)]
#[test]
fn iter_skills_files_skips_symlinks() {
    let home = init_home("iter_skills");
    let d = home.join("skills/cat/myskill/scripts");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(home.join("skills/cat/myskill/SKILL.md"), "# skill").unwrap();
    std::fs::write(d.join("run.sh"), "#!/bin/bash").unwrap();
    let secret = home.join("secret");
    std::fs::write(&secret, "nope").unwrap();
    std::os::unix::fs::symlink(&secret, home.join("skills/cat/myskill/evil")).unwrap();

    with_home(&home, || {
        let files = iter_skills_files("/root/.hermes");
        let paths: Vec<String> = files.iter().map(|f| f.container_path.clone()).collect();
        assert!(paths.contains(&"/root/.hermes/skills/cat/myskill/SKILL.md".to_string()));
        assert!(paths.contains(&"/root/.hermes/skills/cat/myskill/scripts/run.sh".to_string()));
        assert!(!paths.iter().any(|p| p.contains("evil")));
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn iter_skills_empty_when_no_dir() {
    let home = init_home("iter_skills_empty");
    with_home(&home, || {
        assert_eq!(iter_skills_files("/root/.hermes").len(), 0);
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn dotdot_traversal_rejected() {
    let home = init_home("traversal_dotdot");
    std::fs::write(home.parent().unwrap().join("sensitive.json"), "{}").unwrap();
    with_home(&home, || {
        assert!(!register_credential_file("../sensitive.json", "/root/.hermes"));
        assert_eq!(get_credential_file_mounts().len(), 0);
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn deep_traversal_rejected() {
    let home = init_home("traversal_deep");
    let ssh = home.parent().unwrap().join(".ssh");
    std::fs::create_dir_all(&ssh).unwrap();
    std::fs::write(ssh.join("id_rsa"), "PRIVATE KEY").unwrap();
    with_home(&home, || {
        assert!(!register_credential_file("../../.ssh/id_rsa", "/root/.hermes"));
        assert_eq!(get_credential_file_mounts().len(), 0);
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn absolute_path_rejected() {
    let home = init_home("traversal_abs");
    let abs = home.join("absolute.json");
    std::fs::write(&abs, "{}").unwrap();
    with_home(&home, || {
        assert!(!register_credential_file(&abs.to_string_lossy(), "/root/.hermes"));
        assert_eq!(get_credential_file_mounts().len(), 0);
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn nested_subdir_inside_home_allowed() {
    let home = init_home("nested_allowed");
    let sub = home.join("creds");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("oauth.json"), "{}").unwrap();
    with_home(&home, || {
        assert!(register_credential_file("creds/oauth.json", "/root/.hermes"));
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[cfg(unix)]
#[test]
fn symlink_traversal_rejected() {
    let home = init_home("symlink_trav");
    // Upstream places the sensitive file OUTSIDE hermes_home (tmp_path).
    let sensitive = home.parent().unwrap().join("sensitive.json");
    std::fs::write(&sensitive, "{}").unwrap();
    std::os::unix::fs::symlink(&sensitive, home.join("evil_link.json")).unwrap();
    with_home(&home, || {
        assert!(!register_credential_file("evil_link.json", "/root/.hermes"));
        assert_eq!(get_credential_file_mounts().len(), 0);
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn config_traversal_rejected() {
    let home = init_home("cfg_trav");
    let sensitive = home.parent().unwrap().join("secret.json");
    std::fs::write(&sensitive, "{}").unwrap();
    with_home(&home, || {
        set_terminal_credential_files(Some(vec!["../secret.json".to_string()]));
        let mounts = get_credential_file_mounts();
        assert!(!mounts.iter().any(|m| m.host_path == sensitive.to_string_lossy()));
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn config_absolute_rejected() {
    let home = init_home("cfg_abs");
    let abs = home.join("abs.json");
    std::fs::write(&abs, "{}").unwrap();
    with_home(&home, || {
        set_terminal_credential_files(Some(vec![abs.to_string_lossy().into_owned()]));
        assert_eq!(get_credential_file_mounts().len(), 0);
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn config_legitimate_file_works() {
    let home = init_home("cfg_ok");
    std::fs::write(home.join("oauth.json"), "{}").unwrap();
    with_home(&home, || {
        set_terminal_credential_files(Some(vec!["oauth.json".to_string()]));
        let mounts = get_credential_file_mounts();
        assert_eq!(mounts.len(), 1);
        assert!(mounts[0].container_path.contains("oauth.json"));
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn cache_mounts_existing_dirs() {
    let home = init_home("cache_dirs");
    std::fs::create_dir_all(home.join("cache/documents")).unwrap();
    std::fs::create_dir_all(home.join("cache/audio")).unwrap();
    std::fs::create_dir_all(home.join("cache/videos")).unwrap();
    with_home(&home, || {
        let mounts = get_cache_directory_mounts("/root/.hermes");
        let paths: Vec<String> = mounts.iter().map(|m| m.container_path.clone()).collect();
        assert!(paths.contains(&"/root/.hermes/cache/documents".to_string()));
        assert!(paths.contains(&"/root/.hermes/cache/audio".to_string()));
        assert!(paths.contains(&"/root/.hermes/cache/videos".to_string()));
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn cache_legacy_dir_names_resolved() {
    let home = init_home("cache_legacy");
    let legacy_doc = home.join("document_cache");
    let legacy_img = home.join("image_cache");
    std::fs::create_dir_all(&legacy_doc).unwrap();
    std::fs::create_dir_all(&legacy_img).unwrap();
    std::fs::write(legacy_doc.join("cached.txt"), "x").unwrap();
    std::fs::write(legacy_img.join("cached.png"), "x").unwrap();
    with_home(&home, || {
        let mounts = get_cache_directory_mounts("/root/.hermes");
        let host_paths: Vec<String> = mounts.iter().map(|m| m.host_path.clone()).collect();
        assert!(host_paths.contains(&legacy_doc.to_string_lossy().into_owned()));
        assert!(host_paths.contains(&legacy_img.to_string_lossy().into_owned()));
        let container_paths: Vec<String> = mounts.iter().map(|m| m.container_path.clone()).collect();
        assert!(container_paths.contains(&"/root/.hermes/cache/documents".to_string()));
        assert!(container_paths.contains(&"/root/.hermes/cache/images".to_string()));
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn cache_empty_home() {
    let home = init_home("cache_empty");
    with_home(&home, || {
        assert_eq!(get_cache_directory_mounts("/root/.hermes").len(), 0);
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn images_upload_dir_is_mounted() {
    let home = init_home("images_mount");
    std::fs::create_dir_all(home.join("images")).unwrap();
    with_home(&home, || {
        let mounts = get_cache_directory_mounts("/root/.hermes");
        let by_container: std::collections::HashMap<String, String> =
            mounts.iter().map(|m| (m.container_path.clone(), m.host_path.clone())).collect();
        assert_eq!(
            by_container.get("/root/.hermes/images").unwrap(),
            &home.join("images").to_string_lossy().into_owned()
        );
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn images_upload_file_maps_into_container() {
    let home = init_home("images_map");
    let upload = home.join("images/upload_20260722_181019_1.png");
    std::fs::create_dir_all(upload.parent().unwrap()).unwrap();
    std::fs::write(&upload, [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]).unwrap();
    with_home(&home, || {
        assert_eq!(
            map_cache_path_to_container(&upload.to_string_lossy(), "/root/.hermes"),
            Some("/root/.hermes/images/upload_20260722_181019_1.png".to_string())
        );
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn map_cache_path_under_cache_dir() {
    let home = init_home("map_under");
    let img = home.join("cache/images/generated.png");
    std::fs::create_dir_all(img.parent().unwrap()).unwrap();
    std::fs::write(&img, "x").unwrap();
    with_home(&home, || {
        assert_eq!(
            map_cache_path_to_container(&img.to_string_lossy(), "/root/.hermes"),
            Some("/root/.hermes/cache/images/generated.png".to_string())
        );
        // A path not under any cache dir maps to None.
        assert_eq!(map_cache_path_to_container(&home.join("other/x.png").to_string_lossy(), "/root/.hermes"), None);
        // No cache dirs at all → None.
        let empty = init_home("map_empty");
        with_home(&empty, || {
            assert_eq!(map_cache_path_to_container(&empty.join("cache/images/x.png").to_string_lossy(), "/root/.hermes"), None);
        });
        let _ = std::fs::remove_dir_all(&empty);
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn iter_cache_files_enumerates() {
    let home = init_home("iter_cache");
    let d = home.join("cache/documents");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("upload.zip"), [0x50, 0x4b, 0x03, 0x04]).unwrap();
    std::fs::write(d.join("report.pdf"), [0x25, 0x50, 0x44, 0x46]).unwrap();
    with_home(&home, || {
        let entries = iter_cache_files("/root/.hermes");
        let names: Vec<String> = entries.iter().map(|e| {
            std::path::Path::new(&e.container_path).file_name().unwrap().to_string_lossy().into_owned()
        }).collect();
        assert!(names.contains(&"upload.zip".to_string()));
        assert!(names.contains(&"report.pdf".to_string()));
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[cfg(unix)]
#[test]
fn iter_cache_files_skips_symlinks() {
    let home = init_home("iter_cache_sym");
    let d = home.join("cache/documents");
    std::fs::create_dir_all(&d).unwrap();
    let real = d.join("real.txt");
    std::fs::write(&real, "content").unwrap();
    std::os::unix::fs::symlink(&real, d.join("link.txt")).unwrap();
    with_home(&home, || {
        let entries = iter_cache_files("/root/.hermes");
        let names: Vec<String> = entries.iter().map(|e| {
            std::path::Path::new(&e.container_path).file_name().unwrap().to_string_lossy().into_owned()
        }).collect();
        assert!(names.contains(&"real.txt".to_string()));
        assert!(!names.contains(&"link.txt".to_string()));
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn master_credential_stores_refused() {
    let home = init_home("master_stores");
    std::fs::write(home.join(".env"), "OPENAI_API_KEY=sk-REAL\n").unwrap();
    std::fs::write(home.join("auth.json"), "{}").unwrap();
    std::fs::write(home.join(".anthropic_oauth.json"), "{}").unwrap();
    std::fs::write(home.join("webhook_subscriptions.json"), "{}").unwrap();
    std::fs::create_dir_all(home.join("cache")).unwrap();
    std::fs::write(home.join("cache/bws_cache.json"), "{}").unwrap();
    std::fs::create_dir_all(home.join("mcp-tokens")).unwrap();
    std::fs::write(home.join("mcp-tokens/srv.json"), "{}").unwrap();
    std::fs::write(home.join("google_token.json"), "{}").unwrap();

    with_home(&home, || {
        for rel in [".env", "auth.json", ".anthropic_oauth.json", "webhook_subscriptions.json",
                    "cache/bws_cache.json", "mcp-tokens/srv.json"] {
            assert!(!register_credential_file(rel, "/root/.hermes"), "{rel} must not be mountable");
        }
        assert_eq!(get_credential_file_mounts().len(), 0);

        // Per-service token still mounts.
        assert!(register_credential_file("google_token.json", "/root/.hermes"));
        let mounts = get_credential_file_mounts();
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].container_path, "/root/.hermes/google_token.json");
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn refused_entry_does_not_block_rest_of_batch() {
    let home = init_home("batch_refused");
    std::fs::write(home.join(".env"), "k=v\n").unwrap();
    std::fs::write(home.join("google_token.json"), "{}").unwrap();
    with_home(&home, || {
        let missing = register_credential_files(&[json!(".env"), json!("google_token.json")], "/root/.hermes");
        let paths: Vec<String> = get_credential_file_mounts().iter().map(|m| m.container_path.clone()).collect();
        assert!(paths.contains(&"/root/.hermes/google_token.json".to_string()));
        assert!(!paths.contains(&"/root/.hermes/.env".to_string()));
        assert!(missing.contains(&".env".to_string()));
    });
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn traversal_guard_still_applies() {
    let home = init_home("trav_guard");
    let ssh = home.parent().unwrap().join(".ssh");
    std::fs::create_dir_all(&ssh).unwrap();
    std::fs::write(ssh.join("id_rsa"), "PRIVATE").unwrap();
    std::fs::write(home.join("google_token.json"), "{}").unwrap();
    with_home(&home, || {
        assert!(!register_credential_file("../../.ssh/id_rsa", "/root/.hermes"));
        assert!(!register_credential_file("/etc/passwd", "/root/.hermes"));
        assert!(register_credential_file("google_token.json", "/root/.hermes"));
    });
    let _ = std::fs::remove_dir_all(&home);
}
