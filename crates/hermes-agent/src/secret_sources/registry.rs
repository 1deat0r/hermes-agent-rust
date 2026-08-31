//! Secret-source registry + apply orchestrator.
//!
//! PARITY: `agent/secret_sources/registry.py` @ b9aa928 (whole module).
//!
//! This module owns everything that must be uniform across secret
//! backends so no individual source can get it wrong:
//!
//! * registration (name/scheme uniqueness, API-version gating)
//! * per-source wall-clock timeout enforcement around `fetch()`
//! * precedence: mapped sources beat bulk sources; within a shape,
//!   `secrets.sources` order (or registration order) decides; first claim
//!   wins — later sources never silently clobber an earlier one
//! * `override_existing` semantics (may beat .env/shell, never another
//!   secret source, never a protected var)
//! * cross-source conflict warnings (shadowed claims are always surfaced)
//! * provenance: which source supplied every applied var
//!
//! The single startup entry point is [`apply_all`], taking the target
//! environment as an injectable map (upstream defaults to `os.environ`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use serde_json::{json, Value};

use super::base::{
    get_source_env_var, reset_source_environment, set_source_environment, ErrorKind, FetchResult,
    SecretSource, SECRET_SOURCE_API_VERSION,
};

/// Provenance record for one env var the orchestrator set.
///
/// PARITY: `AppliedVar` (upstream lines 55-61).
#[derive(Debug, Clone)]
pub struct AppliedVar {
    pub name: String,
    /// SecretSource.name
    pub source: String,
    /// "mapped" | "bulk"
    pub shape: String,
    /// Replaced a pre-existing .env/shell value.
    pub overrode_env: bool,
}

/// One source's outcome within an [`ApplyReport`].
///
/// PARITY: `SourceReport` (upstream lines 64-74).
#[derive(Debug, Clone, Default)]
pub struct SourceReport {
    pub name: String,
    pub label: String,
    pub result: Option<FetchResult>,
    pub applied: Vec<String>,
    /// .env/shell won
    pub skipped_existing: Vec<String>,
    /// earlier source won
    pub skipped_claimed: Vec<String>,
    /// bootstrap-auth guard
    pub skipped_protected: Vec<String>,
    /// bad env-var name
    pub skipped_invalid: Vec<String>,
}

/// Merged outcome of one orchestrated apply pass.
///
/// PARITY: `ApplyReport` (upstream lines 77-88).
#[derive(Debug, Clone, Default)]
pub struct ApplyReport {
    pub sources: Vec<SourceReport>,
    pub provenance: HashMap<String, AppliedVar>,
    /// Human-readable warnings.
    pub conflicts: Vec<String>,
}

impl ApplyReport {
    /// PARITY: the `applied_any` property.
    pub fn applied_any(&self) -> bool {
        !self.provenance.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Ordered registry: name → source. Insertion order doubles as the default
/// apply order.
///
/// PARITY: `_SOURCES` (upstream line 50).
static SOURCES: OnceLock<Mutex<Vec<(String, Arc<dyn SecretSource>)>>> = OnceLock::new();

fn sources() -> &'static Mutex<Vec<(String, Arc<dyn SecretSource>)>> {
    SOURCES.get_or_init(|| Mutex::new(Vec::new()))
}

/// Register a secret source. Returns true on success.
///
/// Rejections are logged, never raised — a bad plugin must not take down
/// startup. `replace` allows tests / user plugins to override a bundled
/// source of the same name (last-writer-wins like model providers), but
/// scheme collisions across *different* names are always rejected.
///
/// PARITY: `register_source` (upstream lines 92-139).
pub fn register_source(source: Arc<dyn SecretSource>, replace: bool) -> bool {
    let name = source.name().to_string();
    if name.is_empty()
        || name != name.to_lowercase()
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        log::warn!("Ignoring secret source with invalid name {name:?}");
        return false;
    }
    if source.api_version() != SECRET_SOURCE_API_VERSION {
        log::warn!(
            "Ignoring secret source '{name}': built against secret-source API v{}, \
             this Hermes speaks v{SECRET_SOURCE_API_VERSION}",
            source.api_version()
        );
        return false;
    }
    let shape = source.shape().to_string();
    if shape != "mapped" && shape != "bulk" {
        log::warn!(
            "Ignoring secret source '{name}': shape must be 'mapped' or 'bulk', got {shape:?}"
        );
        return false;
    }
    let mut sources = sources().lock().unwrap_or_else(|e| e.into_inner());
    if !replace {
        if let Some(existing) = sources.iter().find(|(n, _)| *n == name) {
            let _ = existing;
            log::warn!("Secret source '{name}' already registered; ignoring duplicate");
            return false;
        }
    } else if let Some(slot) = sources.iter_mut().find(|(n, _)| *n == name) {
        *slot = (name.clone(), source);
        return true;
    }
    if let Some(scheme) = source.scheme() {
        if let Some((other_name, _)) = sources
            .iter()
            .find(|(n, other)| *n != name && other.scheme() == Some(scheme))
        {
            log::warn!(
                "Ignoring secret source '{name}': scheme '{scheme}://' is already owned by source '{other_name}'"
            );
            return false;
        }
    }
    sources.push((name, source));
    true
}

/// Return the registered source for `name`, or None.
///
/// PARITY: `get_source` (upstream lines 142-145); the lazy bundled-source
/// registration is a no-op here (no bundled backends ported yet).
pub fn get_source(name: &str) -> Option<Arc<dyn SecretSource>> {
    sources()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, s)| Arc::clone(s))
}

/// All registered sources, in registration order.
///
/// PARITY: `list_sources` (upstream lines 148-151).
pub fn list_sources() -> Vec<Arc<dyn SecretSource>> {
    sources()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .map(|(_, s)| Arc::clone(s))
        .collect()
}

/// PARITY: `_reset_registry_for_tests` (upstream lines 176-181).
pub fn reset_registry_for_tests() {
    sources().lock().unwrap_or_else(|e| e.into_inner()).clear();
}

// ---------------------------------------------------------------------------
// Orchestrated apply
// ---------------------------------------------------------------------------

/// Run source.fetch() under a wall-clock budget; never raises/panics into
/// the caller.
///
/// PARITY: `_fetch_with_timeout` (upstream lines 184-232). The budget is
/// enforced on a worker thread: a source that blows its budget is reported
/// as TIMEOUT and its (eventual) result is discarded — strictly better
/// than an unbounded hang on every `hermes` invocation.
fn fetch_with_timeout(
    source: Arc<dyn SecretSource>,
    cfg: &Value,
    home_path: PathBuf,
    environ: &HashMap<String, String>,
) -> FetchResult {
    let timeout = source.fetch_timeout_seconds(cfg);
    let cfg = cfg.clone();
    let source_for_thread = Arc::clone(&source);
    let env_snapshot: HashMap<String, String> = environ.clone();

    // Worker: install the env view, fetch, reset.
    let worker = std::thread::spawn(move || {
        let token = set_source_environment(env_snapshot);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            source_for_thread.fetch(&cfg, &home_path)
        }));
        reset_source_environment(token);
        result.unwrap_or_else(|_| FetchResult {
            error: Some("fetch panicked".to_string()),
            error_kind: Some(ErrorKind::Internal),
            ..FetchResult::default()
        })
    });

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs_f64(timeout);
    while worker.is_finished() == false {
        if Instant::now() >= deadline {
            // Upstream: cancel + report TIMEOUT; the worker may linger
            // until process exit (acceptable for a startup-only path).
            let _ = &worker;
            return FetchResult {
                error: Some(format!(
                    "fetch exceeded {timeout:.0}s budget — startup continued without this \
                     source (raise secrets.{}.timeout_seconds if the backend is just slow)",
                    source.name()
                )),
                error_kind: Some(ErrorKind::Timeout),
                ..FetchResult::default()
            };
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    match worker.join() {
        Ok(result) => result,
        Err(_) => FetchResult {
            error: Some("fetch raised an unexpected error".to_string()),
            error_kind: Some(ErrorKind::Internal),
            ..FetchResult::default()
        },
    }
}

/// Resolve which sources run, in which order.
///
/// Order: the optional `secrets.sources` list wins; sources not named
/// there follow in registration order. Enabled = the source's own
/// `is_enabled` for its config section.
///
/// PARITY: `_ordered_enabled_sources` (upstream lines 235-271).
fn ordered_enabled_sources(
    secrets_cfg: &Value,
    registered: &[(String, Arc<dyn SecretSource>)],
) -> Vec<Arc<dyn SecretSource>> {
    let mut order: Vec<String> = Vec::new();
    if let Some(explicit) = secrets_cfg.get("sources").and_then(Value::as_array) {
        for entry in explicit {
            if let Some(name) = entry.as_str() {
                if registered.iter().any(|(n, _)| n == name) && !order.contains(&name.to_string()) {
                    order.push(name.to_string());
                }
            }
        }
        let unknown: Vec<&str> = explicit
            .iter()
            .filter_map(Value::as_str)
            .filter(|e| !registered.iter().any(|(n, _)| n == e))
            .collect();
        if !unknown.is_empty() {
            let known = registered
                .iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            log::warn!(
                "secrets.sources names unknown source(s): {} (known: {})",
                unknown.join(", "),
                if known.is_empty() { "none" } else { &known }
            );
        }
    }
    for (name, _) in registered {
        if !order.contains(name) {
            order.push(name.clone());
        }
    }

    let mut enabled = Vec::new();
    for name in &order {
        if let Some((_, source)) = registered.iter().find(|(n, _)| n == name) {
            let cfg = secrets_cfg.get(name.as_str()).cloned().unwrap_or(json!({}));
            if source.is_enabled(&cfg) {
                enabled.push(Arc::clone(source));
            }
        }
    }
    enabled
}

/// Best-effort active profile name for profile-scoped secret aliases.
///
/// A named profile's HERMES_HOME is `~/.hermes/profiles/<name>`; the
/// default profile returns "".
///
/// PARITY: `_active_profile_name` (upstream lines 274-288).
fn active_profile_name(home_path: Option<&Path>) -> String {
    if let Some(home_path) = home_path {
        if home_path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n == "profiles")
            .unwrap_or(false)
            && home_path.file_name().is_some()
        {
            return home_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
        }
    }
    for env_name in ["HERMES_PROFILE_NAME", "HERMES_PROFILE"] {
        if let Ok(value) = std::env::var(env_name) {
            let value = value.trim();
            if !value.is_empty() && value != "default" {
                return value.to_string();
            }
        }
    }
    String::new()
}

/// Only credential-shaped names get auto-aliased — a random
/// profile-suffixed var should not silently hydrate an unsuffixed name.
///
/// PARITY: `_ALIAS_SUFFIXES` (upstream lines 291-292).
const ALIAS_SUFFIXES: [&str; 5] = ["_API_KEY", "_TOKEN", "_SECRET", "_KEY", "_PASSWORD"];

/// Map `FOO_<PROFILE>` to `FOO` for the active profile when safe.
///
/// PARITY: `_profile_alias_target` (upstream lines 295-305).
fn profile_alias_target(var: &str, profile: &str) -> Option<String> {
    if profile.is_empty() {
        return None;
    }
    let suffix = format!("_{}", profile.replace('-', "_").to_uppercase());
    if !var.ends_with(&suffix) {
        return None;
    }
    let alias = &var[..var.len() - suffix.len()];
    if alias.is_empty() || !super::base::is_valid_env_name(alias) {
        return None;
    }
    if !ALIAS_SUFFIXES.iter().any(|s| alias.ends_with(s)) {
        return None;
    }
    Some(alias.to_string())
}

/// Fetch from every enabled source and apply the merged result to `env`.
///
/// Precedence per env var (most-specific intent wins):
///
/// 1. `secrets.preserve_existing` names — a pre-existing env value always
///    wins for these, even against a source with `override_existing: true`.
/// 2. Pre-existing env (.env / shell) — unless the winning source has
///    `override_existing: true`.
/// 3. Mapped sources, in configured order.
/// 4. Bulk sources, in configured order.
///
/// First claim wins. A later source that also carries the var gets a
/// `skipped_claimed` entry and a conflict warning — never a silent
/// clobber, and `override_existing` never applies across sources.
///
/// Profile aliasing (#51447): when running under a named profile, an
/// applied var `FOO_<PROFILE>` (credential-shaped suffixes only) also
/// hydrates the canonical `FOO`; the alias obeys the same guards and is
/// disabled with `secrets.profile_alias: false`.
///
/// PARITY: `apply_all` (upstream lines 308-437).
pub fn apply_all(
    secrets_cfg: Option<&Value>,
    home_path: Option<&Path>,
    env: &mut HashMap<String, String>,
) -> ApplyReport {
    let mut report = ApplyReport::default();

    let empty = json!({});
    let secrets_cfg = match secrets_cfg {
        Some(cfg) if cfg.is_object() => cfg,
        _ => &empty,
    };
    let registered = {
        // `_ensure_builtin_sources()` — no bundled backends ported yet, so
        // registration order starts empty.
        let _ = sources();
        sources()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(n, s)| (n.clone(), Arc::clone(s)))
            .collect::<Vec<_>>()
    };
    let enabled = ordered_enabled_sources(secrets_cfg, &registered);
    if enabled.is_empty() {
        return report;
    }

    let preserve: Vec<String> = secrets_cfg
        .get("preserve_existing")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let alias_enabled = secrets_cfg
        .get("profile_alias")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let profile = if alias_enabled {
        active_profile_name(home_path)
    } else {
        String::new()
    };

    // Mapped sources outrank bulk sources regardless of list order.
    let mut ordered: Vec<Arc<dyn SecretSource>> = enabled
        .iter()
        .filter(|s| s.shape() == "mapped")
        .cloned()
        .collect();
    ordered.extend(enabled.iter().filter(|s| s.shape() == "bulk").cloned());

    // Fetch phase.
    let mut fetches: Vec<(Arc<dyn SecretSource>, Value, FetchResult)> = Vec::new();
    let mut protected: HashMap<String, String> = HashMap::new();
    for source in &ordered {
        let cfg = secrets_cfg
            .get(source.name())
            .cloned()
            .unwrap_or_else(|| json!({}));
        let result = fetch_with_timeout(
            Arc::clone(source),
            &cfg,
            home_path.unwrap_or(Path::new(".")).to_path_buf(),
            env,
        );
        for var in source.protected_env_vars() {
            protected
                .entry(var)
                .or_insert_with(|| source.name().to_string());
        }
        fetches.push((Arc::clone(source), cfg, result));
    }

    // Every var any source supplies directly — an alias never shadows a
    // var that some source will (or tried to) claim by its real name.
    let mut supplied_directly: std::collections::HashSet<String> = Default::default();
    for (_, _, result) in &fetches {
        if result.ok() {
            for var in result.secrets.keys() {
                supplied_directly.insert(var.clone());
            }
        }
    }

    // Apply phase — sequential, first-wins, fully attributed.
    let mut claimed: HashMap<String, String> = HashMap::new();
    // Alias-application warnings deferred until the mutable fetches pass
    // (upstream appends into `result.warnings` inline).
    let mut deferred_warnings: Vec<(usize, String)> = Vec::new();
    for (fetch_idx, (source, cfg, result)) in fetches.iter().enumerate() {
        let mut sr = SourceReport {
            name: source.name().to_string(),
            label: if source.label().is_empty() {
                source.name().to_string()
            } else {
                source.label().to_string()
            },
            result: Some(result.clone()),
            ..Default::default()
        };
        if !result.ok() {
            report.sources.push(sr);
            continue;
        }
        let override_existing = source.override_existing(cfg);

        for (var, value) in &result.secrets {
            let applied = try_apply(
                TryApplyCtx {
                    source_name: source.name(),
                    source_shape: source.shape(),
                    override_existing,
                    env,
                    protected: &protected,
                    preserve: &preserve,
                    claimed: &mut claimed,
                },
                &mut sr,
                &mut report,
                var,
                value,
            );
            if !applied || profile.is_empty() {
                continue;
            }
            if let Some(alias) = profile_alias_target(var, &profile) {
                if !supplied_directly.contains(&alias) && !claimed.contains_key(&alias) {
                    if try_apply(
                        TryApplyCtx {
                            source_name: source.name(),
                            source_shape: source.shape(),
                            override_existing,
                            env,
                            protected: &protected,
                            preserve: &preserve,
                            claimed: &mut claimed,
                        },
                        &mut sr,
                        &mut report,
                        &alias,
                        value,
                    ) {
                        deferred_warnings.push((
                            fetch_idx,
                            format!(
                                "applied profile-scoped {var} as {alias} (active profile {profile:?})"
                            ),
                        ));
                    }
                }
            }
        }
        report.sources.push(sr);
    }
    for (fetch_idx, warning) in deferred_warnings {
        fetches[fetch_idx].2.warnings.push(warning);
    }
    report
}

/// The shared guard chain for applying one var; true = applied.
///
/// PARITY: the `_try_apply` closure (upstream lines 360-410).
struct TryApplyCtx<'a> {
    source_name: &'a str,
    source_shape: &'a str,
    override_existing: bool,
    env: &'a mut HashMap<String, String>,
    protected: &'a HashMap<String, String>,
    preserve: &'a [String],
    claimed: &'a mut HashMap<String, String>,
}

fn try_apply(
    ctx: TryApplyCtx<'_>,
    sr: &mut SourceReport,
    report: &mut ApplyReport,
    var: &str,
    value: &str,
) -> bool {
    if !super::base::is_valid_env_name(var) {
        sr.skipped_invalid.push(var.to_string());
        return false;
    }
    if ctx.protected.contains_key(var) {
        sr.skipped_protected.push(var.to_string());
        return false;
    }
    if let Some(winner) = ctx.claimed.get(var) {
        sr.skipped_claimed.push(var.to_string());
        report.conflicts.push(format!(
            "{var}: kept value from {winner}; {} also supplies it (first source wins — \
             remove one binding or reorder secrets.sources)",
            ctx.source_name
        ));
        return false;
    }
    let existed = ctx.env.contains_key(var);
    if existed && ctx.preserve.iter().any(|p| p == var) {
        sr.skipped_existing.push(var.to_string());
        return false;
    }
    if existed && !ctx.override_existing {
        sr.skipped_existing.push(var.to_string());
        return false;
    }
    ctx.env.insert(var.to_string(), value.to_string());
    ctx.claimed
        .insert(var.to_string(), ctx.source_name.to_string());
    sr.applied.push(var.to_string());
    report.provenance.insert(
        var.to_string(),
        AppliedVar {
            name: var.to_string(),
            source: ctx.source_name.to_string(),
            shape: ctx.source_shape.to_string(),
            overrode_env: existed,
        },
    );
    true
}
