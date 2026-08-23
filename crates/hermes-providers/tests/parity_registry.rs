//! Parity oracles for `providers/__init__.py`, mirroring the pinned registry
//! and discovery tests at b9aa928.
//!
//! Tier: mock/unit. The discovery callback stands in for importing a Python
//! plugin's `__init__.py` and calling `register_provider()`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use hermes_providers::registry::{
    discover_with_loader, discovered_for_tests, get_provider_profile, list_providers,
    mark_discovered_for_tests, plugin_module_name, register_provider, reset_registry_for_tests,
    user_plugins_dir, ProviderSource,
};
use hermes_providers::ProviderProfile;

static REGISTRY_TEST_LOCK: Mutex<()> = Mutex::new(());

fn profile(name: &str, aliases: &[&str]) -> ProviderProfile {
    let mut value = ProviderProfile::new(name);
    value.aliases = aliases.iter().map(|alias| (*alias).into()).collect();
    value
}

fn plugin_dir(root: &Path, name: &str) -> PathBuf {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("__init__.py"), "# test plugin\n").unwrap();
    dir
}

#[test]
fn registration_maps_names_and_aliases_and_invalidates_cache() {
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    mark_discovered_for_tests();

    register_provider(profile("alpha", &["a"]));
    assert_eq!(get_provider_profile("alpha").unwrap().name, "alpha");
    assert_eq!(get_provider_profile("a").unwrap().name, "alpha");
    assert_eq!(list_providers().len(), 1);

    let mut replacement = profile("alpha", &[]);
    replacement.display_name = "replacement".into();
    register_provider(replacement);
    assert_eq!(
        get_provider_profile("alpha").unwrap().display_name,
        "replacement"
    );
    // Python keeps the old alias mapping when an existing canonical key is
    // replaced; the alias still resolves through the canonical name.
    assert_eq!(
        get_provider_profile("a").unwrap().display_name,
        "replacement"
    );
    assert_eq!(list_providers().len(), 1);
}

#[test]
fn list_returns_copy_safe_cached_snapshot_until_registration_changes() {
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    mark_discovered_for_tests();
    let first = profile("alpha", &[]);
    register_provider(first.clone());

    let mut listed = list_providers();
    listed.clear();
    assert_eq!(list_providers(), vec![first.clone()]);

    list_providers().clear();
    assert_eq!(list_providers(), vec![first.clone()]);

    let second = profile("beta", &[]);
    register_provider(second.clone());
    assert_eq!(list_providers(), vec![first, second]);
}

#[test]
fn discovery_is_lazy_and_loader_order_matches_upstream() {
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    assert!(!discovered_for_tests());

    // A normal registry access marks discovery complete and registers the
    // statically linked bundled profiles before the user loader seam runs.
    assert_eq!(list_providers().len(), 2);
    assert!(get_provider_profile("alibaba").is_some());
    assert!(get_provider_profile("arcee").is_some());
    assert!(discovered_for_tests());
}

#[test]
fn bundled_then_user_discovery_is_sorted_and_user_wins() {
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let temp = tempfile::tempdir().unwrap();
    let bundled = temp.path().join("bundled");
    let user = temp.path().join("user");
    fs::create_dir_all(&bundled).unwrap();
    fs::create_dir_all(&user).unwrap();
    plugin_dir(&bundled, "z-provider");
    plugin_dir(&bundled, "a-provider");
    plugin_dir(&user, "z-provider");
    plugin_dir(&user, "user-only");
    fs::create_dir_all(bundled.join(".hidden")).unwrap();
    fs::create_dir_all(bundled.join("not-a-plugin")).unwrap();
    fs::write(bundled.join("plain-file"), "not a directory").unwrap();

    let mut calls = Vec::new();
    discover_with_loader(Some(&bundled), Some(&user), None, |path, source| {
        calls.push((
            source,
            path.file_name().unwrap().to_string_lossy().into_owned(),
        ));
        let name = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .replace('_', "-");
        let mut value = profile(&name, &[]);
        if source == ProviderSource::User && name == "z-provider" {
            value.base_url = "https://user-override.example/v1".into();
        }
        Ok(Some(value))
    });

    assert_eq!(
        calls,
        vec![
            (ProviderSource::Bundled, "a-provider".into()),
            (ProviderSource::Bundled, "z-provider".into()),
            (ProviderSource::User, "user-only".into()),
            (ProviderSource::User, "z-provider".into()),
        ]
    );
    let names: Vec<_> = list_providers().into_iter().map(|p| p.name).collect();
    assert_eq!(names, vec!["a-provider", "z-provider", "user-only"]);
    assert_eq!(
        get_provider_profile("z-provider").unwrap().base_url,
        "https://user-override.example/v1"
    );
}

#[test]
fn broken_plugins_fail_open_and_do_not_block_later_plugins() {
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let temp = tempfile::tempdir().unwrap();
    let bundled = temp.path().join("bundled");
    fs::create_dir_all(&bundled).unwrap();
    plugin_dir(&bundled, "broken");
    plugin_dir(&bundled, "healthy");

    discover_with_loader(Some(&bundled), None, None, |path, _| {
        if path.file_name().unwrap() == "broken" {
            Err("synthetic import failure".into())
        } else {
            Ok(Some(profile("healthy", &[])))
        }
    });
    assert!(get_provider_profile("broken").is_none());
    assert!(get_provider_profile("healthy").is_some());
}

#[test]
fn legacy_files_skip_private_base_and_non_python_entries() {
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    reset_registry_for_tests();
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("zeta.py"), "# legacy\n").unwrap();
    fs::write(temp.path().join("alpha.py"), "# legacy\n").unwrap();
    fs::write(temp.path().join("_private.py"), "# skip\n").unwrap();
    fs::write(temp.path().join("base.py"), "# skip\n").unwrap();
    fs::write(temp.path().join("README.md"), "# skip\n").unwrap();

    let mut seen = Vec::new();
    discover_with_loader(None, None, Some(temp.path()), |path, source| {
        seen.push((
            source,
            path.file_name().unwrap().to_string_lossy().into_owned(),
        ));
        Ok(Some(profile(
            path.file_stem().unwrap().to_string_lossy().as_ref(),
            &[],
        )))
    });
    assert_eq!(
        seen,
        vec![
            (ProviderSource::Legacy, "alpha.py".into()),
            (ProviderSource::Legacy, "zeta.py".into()),
        ]
    );
    assert!(get_provider_profile("alpha").is_some());
    assert!(get_provider_profile("base").is_none());
}

#[test]
fn plugin_module_names_match_bundled_and_user_rules() {
    let dir = Path::new("my-provider");
    assert_eq!(
        plugin_module_name(dir, ProviderSource::Bundled),
        "plugins.model_providers.my_provider"
    );
    assert_eq!(
        plugin_module_name(dir, ProviderSource::User),
        "_hermes_user_provider_my_provider"
    );
}

#[test]
fn user_plugins_dir_resolves_under_hermes_home_and_fails_open_when_absent() {
    let _guard = REGISTRY_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let token = hermes_constants::set_hermes_home_override(Some(temp.path()));
    assert_eq!(user_plugins_dir(), None);
    let expected = temp.path().join("plugins").join("model-providers");
    fs::create_dir_all(&expected).unwrap();
    assert_eq!(user_plugins_dir(), Some(expected));
    hermes_constants::reset_hermes_home_override(token);
}
