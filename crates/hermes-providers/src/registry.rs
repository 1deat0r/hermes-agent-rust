//! Provider registry and discovery surface.
//!
//! PARITY: `providers/__init__.py` @ b9aa928. The Rust loader accepts an
//! explicit plugin callback because upstream plugin files are Python modules;
//! the callback is the integration seam until their Rust profiles land.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::base::ProviderProfile;

/// Where a discovered provider plugin came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderSource {
    Bundled,
    User,
    Legacy,
}

struct RegistryState {
    // Vec preserves Python dict insertion order. Replacing an existing
    // canonical name updates the slot in place, just like dict assignment.
    registry: Vec<ProviderProfile>,
    aliases: HashMap<String, String>,
    provider_list_cache: Option<Vec<ProviderProfile>>,
    discovered: bool,
}

impl RegistryState {
    fn new() -> Self {
        Self {
            registry: Vec::new(),
            aliases: HashMap::new(),
            provider_list_cache: None,
            discovered: false,
        }
    }
}

static REGISTRY: OnceLock<Mutex<RegistryState>> = OnceLock::new();

fn registry_state() -> &'static Mutex<RegistryState> {
    REGISTRY.get_or_init(|| Mutex::new(RegistryState::new()))
}

pub fn register_provider(profile: ProviderProfile) {
    let name = profile.name.clone();
    let aliases = profile.aliases.clone();
    let mut state = registry_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(existing) = state.registry.iter_mut().find(|item| item.name == name) {
        *existing = profile;
    } else {
        state.registry.push(profile);
    }
    for alias in aliases {
        state.aliases.insert(alias, name.clone());
    }
    state.provider_list_cache = None;
}

pub fn get_provider_profile(name: &str) -> Option<ProviderProfile> {
    ensure_discovered();
    let state = registry_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let canonical = state.aliases.get(name).map(String::as_str).unwrap_or(name);
    state
        .registry
        .iter()
        .find(|profile| profile.name == canonical)
        .cloned()
}

pub fn list_providers() -> Vec<ProviderProfile> {
    ensure_discovered();
    let mut state = registry_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(cached) = &state.provider_list_cache {
        return cached.clone();
    }

    // Canonical registrations are unique by name. Keep this explicit
    // identity-by-canonical-name guard for parity if a future loader inserts
    // the same profile through more than one registration path.
    let mut seen = HashSet::new();
    let result: Vec<_> = state
        .registry
        .iter()
        .filter(|profile| seen.insert(profile.name.clone()))
        .cloned()
        .collect();
    state.provider_list_cache = Some(result.clone());
    result
}

pub fn user_plugins_dir() -> Option<PathBuf> {
    // PARITY: `_user_plugins_dir` catches all resolution failures and only
    // returns an existing directory.
    std::panic::catch_unwind(|| {
        let dir = hermes_constants::get_hermes_home()
            .join("plugins")
            .join("model-providers");
        dir.is_dir().then_some(dir)
    })
    .unwrap_or(None)
}

pub fn plugin_module_name(plugin_dir: &Path, source: ProviderSource) -> String {
    let safe_name = plugin_dir
        .file_name()
        .map(|name| name.to_string_lossy().replace('-', "_"))
        .unwrap_or_default();
    match source {
        ProviderSource::Bundled => format!("plugins.model_providers.{safe_name}"),
        ProviderSource::User => format!("_hermes_user_provider_{safe_name}"),
        ProviderSource::Legacy => format!("providers.{safe_name}"),
    }
}

pub fn discover_with_loader<F>(
    bundled_dir: Option<&Path>,
    user_dir: Option<&Path>,
    legacy_dir: Option<&Path>,
    mut loader: F,
) where
    F: FnMut(&Path, ProviderSource) -> Result<Option<ProviderProfile>, String>,
{
    {
        let mut state = registry_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.discovered {
            return;
        }
        // PARITY: mark discovered before importing anything so a partially
        // failing plugin set cannot recursively trigger discovery.
        state.discovered = true;
    }

    let mut loaded_modules = HashSet::new();
    scan_plugin_dirs(
        bundled_dir,
        ProviderSource::Bundled,
        &mut loaded_modules,
        &mut loader,
    );
    scan_plugin_dirs(
        user_dir,
        ProviderSource::User,
        &mut loaded_modules,
        &mut loader,
    );
    scan_legacy_modules(legacy_dir, &mut loaded_modules, &mut loader);
}

pub fn reset_registry_for_tests() {
    let mut state = registry_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *state = RegistryState::new();
}

/// Mark the registry as already discovered for isolated registry tests.
#[doc(hidden)]
pub fn mark_discovered_for_tests() {
    registry_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .discovered = true;
}

pub fn discovered_for_tests() -> bool {
    registry_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .discovered
}

fn ensure_discovered() {
    if discovered_for_tests() {
        return;
    }

    // PARITY: the Python registry imports bundled/user Python modules here.
    // Rust profile modules are statically linked and are registered in the
    // same bundled-before-user order before the user loader seam runs.
    crate::profiles::register_builtin_profiles();
    let user_dir = user_plugins_dir();
    discover_with_loader(None, user_dir.as_deref(), None, |path, source| {
        crate::profiles::load_profile(path, source)
    });
}

fn scan_plugin_dirs<F>(
    root: Option<&Path>,
    source: ProviderSource,
    loaded_modules: &mut HashSet<String>,
    loader: &mut F,
) where
    F: FnMut(&Path, ProviderSource) -> Result<Option<ProviderProfile>, String>,
{
    let Some(root) = root else { return };
    let mut children = match fs::read_dir(root) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<Vec<_>>(),
        Err(error) => {
            log::debug!("provider plugin directory {}: {}", root.display(), error);
            return;
        }
    };
    children.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .cmp(right.file_name().unwrap_or_default())
    });

    for child in children {
        let Some(name) = child.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !child.is_dir() || name.starts_with('_') || name.starts_with('.') {
            continue;
        }
        // Python's plugin importer requires an __init__.py before it executes
        // the directory. Manifest presence is intentionally not required by
        // discovery; the upstream loader only checks the init file here.
        if !child.join("__init__.py").is_file() {
            continue;
        }
        import_plugin(&child, source, loaded_modules, loader);
    }
}

fn scan_legacy_modules<F>(root: Option<&Path>, loaded_modules: &mut HashSet<String>, loader: &mut F)
where
    F: FnMut(&Path, ProviderSource) -> Result<Option<ProviderProfile>, String>,
{
    let Some(root) = root else { return };
    let mut modules = match fs::read_dir(root) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<Vec<_>>(),
        Err(error) => {
            log::debug!("legacy provider directory {}: {}", root.display(), error);
            return;
        }
    };
    modules.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .cmp(right.file_name().unwrap_or_default())
    });

    for module in modules {
        let Some(name) = module.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.starts_with('_') || name == "base.py" || name == "__init__.py" {
            continue;
        }
        let is_python_file = module.is_file() && module.extension().is_some_and(|ext| ext == "py");
        let is_python_package = module.is_dir() && module.join("__init__.py").is_file();
        if !(is_python_file || is_python_package) {
            continue;
        }
        import_plugin(&module, ProviderSource::Legacy, loaded_modules, loader);
    }
}

fn import_plugin<F>(
    path: &Path,
    source: ProviderSource,
    loaded_modules: &mut HashSet<String>,
    loader: &mut F,
) where
    F: FnMut(&Path, ProviderSource) -> Result<Option<ProviderProfile>, String>,
{
    let module_name = plugin_module_name(path, source);
    if !loaded_modules.insert(module_name) {
        return;
    }
    match loader(path, source) {
        Ok(Some(profile)) => register_provider(profile),
        Ok(None) => {}
        Err(error) => {
            // PARITY: a broken bundled/user import is warning-only and is
            // removed from the module cache so later discovery can retry it.
            log::warn!(
                "Failed to load {:?} provider plugin {}: {}",
                source,
                path.file_name().unwrap_or_default().to_string_lossy(),
                error
            );
        }
    }
}
