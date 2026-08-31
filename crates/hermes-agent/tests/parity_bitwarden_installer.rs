//! Parity tests for the `agent/secret_sources/bitwarden.py` installer
//! (partial port: pure installer logic + staged install with an
//! injectable download seam) @ b9aa928. The real HTTPS layer stays
//! caller-supplied; cases derive from the upstream code as oracle.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use hermes_agent::secret_sources::bitwarden::{
    expected_sha256, install_bws_at, pick_zip_member, platform_asset_name, BWS_CHECKSUM_NAME,
};

#[cfg(unix)]
fn write_executable(path: &Path, contents: &[u8]) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, contents).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(not(unix))]
fn write_executable(path: &Path, contents: &[u8]) {
    fs::write(path, contents).unwrap();
}

/// Build a minimal zip containing the given (name → bytes) members.
fn make_zip(path: &Path, members: &[(&str, &[u8])]) {
    let file = fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::SimpleFileOptions = Default::default();
    for (name, contents) in members {
        zip.start_file(*name, options).unwrap();
        zip.write_all(contents).unwrap();
    }
    let _ = zip.finish();
}

fn make_checksums(entries: &[(&str, &str)]) -> String {
    entries
        .iter()
        .map(|(name, hex)| format!("{hex}  {name}\n"))
        .collect()
}

fn sha256_of(data: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(data);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn platform_asset_name_matches_the_triple_convention() {
    // The grammar is pinned for the running platform; the other platform
    // arms are compile-time (cfg) so only this one is observable.
    let asset = platform_asset_name().unwrap();
    if cfg!(target_os = "linux") {
        let is_gnu = asset.contains("-unknown-linux-gnu-");
        let is_musl = asset.contains("-unknown-linux-musl-");
        assert!(is_gnu ^ is_musl, "{asset}");
        let arch_ok = asset.contains("x86_64") ^ asset.contains("aarch64");
        assert!(arch_ok, "{asset}");
    } else if cfg!(target_os = "macos") {
        assert_eq!(asset, format!("bws-macos-universal-{}", "2.0.0"));
    }
}

#[test]
fn checksum_file_parsing() {
    let td = tempfile::TempDir::new().unwrap();
    let checksums = td.path().join(BWS_CHECKSUM_NAME);
    fs::write(
        &checksums,
        make_checksums(&[
            ("bws-aarch64-unknown-linux-gnu-2.0.0.zip", "aaaa"),
            ("bws-x86_64-unknown-linux-gnu-2.0.0.zip", "bbbb"),
        ]),
    )
    .unwrap();
    assert_eq!(
        expected_sha256(&checksums, "bws-x86_64-unknown-linux-gnu-2.0.0.zip").unwrap(),
        "bbbb"
    );
    // Missing entry is an error (never a silent empty match).
    assert!(expected_sha256(&checksums, "nope.zip").is_err());
}

#[test]
fn zip_member_pick_prefers_shortest_path() {
    let members = vec![
        "nested/deep/bws".to_string(),
        "bws".to_string(),
        "other".to_string(),
    ];
    assert_eq!(pick_zip_member(&members, "bws").unwrap(), "bws");
    assert!(pick_zip_member(&members, "missing").is_err());
}

#[test]
fn zip_slip_members_are_refused() {
    let td = tempfile::TempDir::new().unwrap();
    let archive = td.path().join("evil.zip");
    make_zip(
        &archive,
        &[("../../etc/cron.d/evil", b"pwned"), ("good", b"ok")],
    );
    // safe_extract_member refuses a traversal member before touching the
    // disk.
    let result = hermes_agent::secret_sources::bitwarden::safe_extract_member(
        &archive,
        "../../etc/cron.d/evil",
        td.path(),
    );
    assert!(result.is_err(), "zip-slip member refused: {result:?}");
    // And nothing escaped the extraction dir.
    assert!(!Path::new("/etc/cron.d/evil").exists());
}

#[test]
fn install_stages_checksum_verified_binary() {
    let td = tempfile::TempDir::new().unwrap();
    let bin_dir = td.path().join("bin");
    let download_dir = td.path().join("downloads");
    fs::create_dir_all(&download_dir).unwrap();

    let binary = b"#!/bin/sh\necho bws\n";
    let asset_name = platform_asset_name().unwrap();
    let zip_bytes: Vec<u8> = {
        let zip_path = download_dir.join("stage.zip");
        // The archive carries the binary member; the zip FILE is named
        // after the asset.
        make_zip(&zip_path, &[("bws", binary), ("README", b"readme")]);
        fs::read(&zip_path).unwrap()
    };
    let hex = sha256_of(&zip_bytes);

    // The "network": copies the staged fixture into the requested dest.
    let downloaded: HashMap<String, Vec<u8>> = [
        (format!("{asset_name}"), zip_bytes.clone()),
        (
            BWS_CHECKSUM_NAME.to_string(),
            make_checksums(&[(&asset_name, &hex)]).into_bytes(),
        ),
    ]
    .into_iter()
    .collect();
    let downloaded = std::sync::Arc::new(downloaded);
    let download = {
        let downloaded = Arc::clone(&downloaded);
        move |url: &str, dest: &Path| -> Result<(), String> {
            let name = url.rsplit('/').next().unwrap();
            let data = downloaded.get(name).ok_or("url not staged")?;
            fs::write(dest, data).map_err(|e| e.to_string())
        }
    };

    let installed = install_bws_at(&bin_dir, false, &download).unwrap();
    assert!(installed.exists());
    assert_eq!(fs::read(&installed).unwrap(), binary);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&installed).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755, "installed binary must be 0755");
    }
}

#[test]
fn install_existing_target_short_circuits_without_download() {
    let td = tempfile::TempDir::new().unwrap();
    let bin_dir = td.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    write_executable(&bin_dir.join("bws"), b"already installed");

    // A downloader that fails the test if ever invoked.
    let download = |_url: &str, _dest: &Path| -> Result<(), String> {
        panic!("no download when the target exists");
    };
    let installed = install_bws_at(&bin_dir, false, &download).unwrap();
    assert_eq!(fs::read(&installed).unwrap(), b"already installed".to_vec());
}

#[test]
fn checksum_mismatch_aborts_the_install() {
    let td = tempfile::TempDir::new().unwrap();
    let bin_dir = td.path().join("bin");
    let download_dir = td.path().join("downloads");
    fs::create_dir_all(&download_dir).unwrap();

    let asset_name = platform_asset_name().unwrap();
    let zip_bytes: Vec<u8> = {
        let zip_path = download_dir.join("stage.zip");
        make_zip(&zip_path, &[(asset_name.as_str(), b"payload")]);
        fs::read(&zip_path).unwrap()
    };
    let wrong_hex = sha256_of(b"other");
    let staged: HashMap<String, Vec<u8>> = [
        (format!("{asset_name}"), zip_bytes),
        (
            BWS_CHECKSUM_NAME.to_string(),
            make_checksums(&[(&asset_name, &wrong_hex)]).into_bytes(),
        ),
    ]
    .into_iter()
    .collect();
    let download = move |url: &str, dest: &Path| -> Result<(), String> {
        let name = url.rsplit('/').next().unwrap();
        let data = staged.get(name).ok_or("url not staged")?;
        fs::write(dest, data).map_err(|e| e.to_string())
    };

    let err = install_bws_at(&bin_dir, false, &download).unwrap_err();
    assert!(err.contains("Checksum mismatch"), "{err}");
    // The target was never installed.
    assert!(!bin_dir.join("bws").exists());
}
