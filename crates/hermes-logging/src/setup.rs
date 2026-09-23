//! Top-level `setup_logging`/`setup_verbose_logging` + config defaults.
//!
//! PARITY: hermes_logging.py lines 203–284 (`setup_logging`,
//! `setup_verbose_logging`) and 687–710 (`_read_logging_config`).

use crate::profile::ProfileRouter;
use crate::queue::{register_queued_handler, register_queued_router, register_queued_target};
use crate::record::Level;
use crate::rotating::{ComponentFilter, RotatingHandler};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

static LOGGING_INITIALIZED: AtomicBool = AtomicBool::new(false);
static VERBOSE_INSTALLED: AtomicBool = AtomicBool::new(false);
static VERBOSE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Component logger-name prefixes used for gateway.log / gui.log routing.
/// Exposed for `hermes logs --component`.
///
/// PARITY: `COMPONENT_PREFIXES` (159–169).
pub const COMPONENT_PREFIXES: [(&str, &[&str]); 6] = [
    (
        "gateway",
        &["gateway", "hermes_plugins", "plugins.platforms"],
    ),
    (
        "agent",
        &["agent", "run_agent", "model_tools", "batch_runner"],
    ),
    ("tools", &["tools"]),
    ("cli", &["hermes_cli", "cli"]),
    ("cron", &["cron"]),
    (
        "gui",
        &[
            "hermes_cli.web_server",
            "hermes_cli.pty_bridge",
            "tui_gateway",
            "uvicorn",
        ],
    ),
];

/// Third-party loggers suppressed at DEBUG/INFO level.
///
/// PARITY: `_NOISY_LOGGERS` (92–97). PORT SEAMS: upstream pins these Python
/// `logging` loggers at WARNING via `_quiet_noisy_loggers` (100–103); the
/// port has no Python logging root — the const is exported for CLI consumers
/// and future logger-name keyed filters.
pub const NOISY_LOGGERS: [&str; 14] = [
    "openai",
    "openai._base_client",
    "httpx",
    "httpcore",
    "asyncio",
    "hpack",
    "hpack.hpack",
    "grpc",
    "modal",
    "urllib3",
    "urllib3.connectionpool",
    "websockets",
    "charset_normalizer",
    "markdown_it",
];

/// Options mirroring the keyword arguments of upstream `setup_logging`.
#[derive(Debug, Clone, Default)]
pub struct SetupOptions {
    pub hermes_home: Option<PathBuf>,
    pub log_level: Option<String>,
    pub max_size_mb: Option<u64>,
    pub backup_count: Option<usize>,
    pub mode: Option<String>,
    pub force: bool,
}

/// Configure the Hermes logging subsystem.
///
/// Safe to call multiple times — the second call is a no-op unless `force`.
///
/// PARITY: hermes_logging.py `setup_logging` (203–263): adopt early-return
/// runs BEFORE config/handler registration (226–227); handler registration
/// runs BEFORE the `_logging_initialized` guard (246–256).
///
/// Divergence (PORT SEAMS): upstream's `mkdir_under_hermes_home` refuses
/// tombstoned named-profile homes (`assert_named_profile_home_live`, 282–287);
/// that guard lives in the hermes-constants profiles surface (not yet
/// ported) — `create_dir_all` fails open until then.
pub fn setup_logging(opts: SetupOptions) -> PathBuf {
    // Upstream installs the RedactingFormatter on every log handler; the
    // port's process-wide redactor seam gets the same real redactor here
    // (first install wins — PARITY: hermes_logging.py formatter wiring).
    crate::record::install_redactor(Box::new(crate::redact::RedactingFormatter));
    let home = opts
        .hermes_home
        .clone()
        .unwrap_or_else(hermes_constants::get_hermes_home);
    let log_dir = home.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);

    // A second Hermes home in a process that already logs for another one —
    // PARITY: `_adopt_secondary_home(home)` early return (226–227).
    if crate::profile::adopt_secondary_home(&home) {
        return log_dir;
    }

    // Best-effort config defaults (config may not be loaded yet).
    let (cfg_level, cfg_max_size, cfg_backup) = read_logging_config();

    let level_name = opts
        .log_level
        .clone()
        .or(cfg_level)
        .unwrap_or_else(|| "INFO".to_string());
    let level = Level::parse(&level_name);
    let max_bytes = opts.max_size_mb.or(cfg_max_size).unwrap_or(5) * 1024 * 1024;
    let backups = opts.backup_count.or(cfg_backup).unwrap_or(3);

    // (filename, level, max_bytes, backup_count, component) — PARITY:
    // `handler_specs` (240–253).
    add_rotating_handler(log_dir.join("agent.log"), level, max_bytes, backups, None);
    add_rotating_handler(
        log_dir.join("errors.log"),
        Level::Warning,
        2 * 1024 * 1024,
        2,
        None,
    );
    if opts.mode.as_deref() == Some("gateway") {
        add_rotating_handler(
            log_dir.join("gateway.log"),
            Level::Info,
            5 * 1024 * 1024,
            3,
            Some(component_filter("gateway")),
        );
    }
    if opts.mode.as_deref() == Some("gui") {
        add_rotating_handler(
            log_dir.join("gui.log"),
            Level::Info,
            10 * 1024 * 1024,
            5,
            Some(component_filter("gui")),
        );
    }

    // PARITY: the `_logging_initialized and not force` guard runs AFTER
    // handler registration (255–256) — subsequent calls adopt routers and
    // dedup per path rather than skipping wholesale.
    if LOGGING_INITIALIZED.load(Ordering::SeqCst) && !opts.force {
        return log_dir;
    }

    // PORT SEAMS: upstream sets the Python root logger level + quiets noisy
    // loggers here (258–261); the port has no root logger (see NOISY_LOGGERS).
    LOGGING_INITIALIZED.store(true, Ordering::SeqCst);
    log_dir
}

fn component_filter(name: &str) -> ComponentFilter {
    ComponentFilter {
        prefixes: COMPONENT_PREFIXES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, p)| p.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default(),
    }
}

/// Enable DEBUG-level console logging for `--verbose` / `-v` mode.
///
/// The Rust port writes verbose records to stderr with the verbose format.
///
/// PARITY: hermes_logging.py `setup_verbose_logging` (266–284): a second
/// call is a no-op while a `_hermes_verbose` handler exists (272–273).
///
/// PORT SEAMS: upstream also lowers the Python root level to DEBUG, quiets
/// noisy loggers, and pins `rex-deploy` at INFO (280–284) — no Python
/// logging root exists in the port; `VerboseStderrHandler` accepts every
/// level (its Python counterpart's DEBUG floor is the global floor).
pub fn setup_verbose_logging() {
    if VERBOSE_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    VERBOSE_COUNT.fetch_add(1, Ordering::SeqCst);
    let handler: Arc<dyn crate::record::LogTarget> = Arc::new(VerboseStderrHandler::new());
    register_queued_target(handler);
    LOGGING_INITIALIZED.store(true, Ordering::SeqCst);
}

/// How many verbose stderr handlers have been installed (oracle:
/// exactly one across repeated `setup_verbose_logging` calls).
#[doc(hidden)]
pub fn verbose_handler_count() -> usize {
    VERBOSE_COUNT.load(Ordering::SeqCst)
}

/// Minimal stderr handler for verbose mode (writes via the async queue).
pub struct VerboseStderrHandler;

impl Default for VerboseStderrHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl VerboseStderrHandler {
    pub fn new() -> Self {
        Self
    }
}

impl crate::record::LogTarget for VerboseStderrHandler {
    fn accepts(&self, _record: &crate::record::LogRecord) -> bool {
        true
    }

    fn emit(&self, record: &crate::record::LogRecord) {
        eprintln!("{}", crate::record::redact(&record.format_verbose()));
    }
}

/// Register a queued `RotatingFileHandler` for *path*; idempotent per
/// resolved path, router-aware when profile routing is live.
///
/// PARITY: `_add_rotating_handler` (644–684):
/// - dedup by resolved path (bare) / `_hermes_routed_log_path` / router
///   filename + home coverage (654–666) — resolution is
///   `resolve_tolerant`, matching non-strict `Path.resolve()`, so the check
///   works for paths renamed away (Allen repro, 672 comment);
/// - wrap into a router when routing is already on (674–682).
pub fn add_rotating_handler(
    path: PathBuf,
    level: Level,
    max_bytes: u64,
    backup_count: usize,
    component: Option<ComponentFilter>,
) {
    let resolved = hermes_constants::resolve_tolerant(&path);
    let (bare, routers) = crate::queue::file_targets_snapshot();

    for existing in &bare {
        if hermes_constants::resolve_tolerant(&existing.path) == resolved {
            return;
        }
    }
    for router in &routers {
        // PARITY: `_hermes_routed_log_path == resolved` (658–661) — first.
        if router.routed_path() == resolved {
            return;
        }
        // PARITY: router filename + default/profile home coverage (663–666).
        let fname = resolved
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
        if let Some(fname) = fname {
            if router.filename() == fname {
                if let Some(home_root) = resolved.parent().and_then(|p| p.parent()) {
                    if home_root == router.default_home() || router.homes_contains(home_root) {
                        return;
                    }
                }
            }
        }
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let handler = match RotatingHandler::new(&path, level, max_bytes, backup_count, component) {
        Ok(h) => h,
        Err(_) => return, // logging must never crash startup
    };

    if routers.is_empty() {
        register_queued_handler(Arc::new(handler));
        return;
    }

    // Routing already on: wrap the new handler so it takes every known
    // home's records instead of stacking a bare handler — PARITY: 674–682.
    let mut homes: Vec<PathBuf> = Vec::new();
    for router in &routers {
        let d = router.default_home().to_path_buf();
        if !homes.contains(&d) {
            homes.push(d);
        }
        for h in router.profile_homes() {
            if !homes.contains(&h) {
                homes.push(h);
            }
        }
    }
    homes.sort(); // PARITY: `sorted(homes)` (680)
    let router = Arc::new(ProfileRouter::new(&handler, &homes));
    drop(handler); // PARITY: `_quietly(handler.close)` (681) — fd closes on drop
    register_queued_router(router);
}

/// Best-effort read of `logging.*` from config.yaml.
///
/// Returns `(level, max_size_mb, backup_count)` — any may be `None`.
///
/// PARITY: `_read_logging_config` (687–710). PORT SEAMS: upstream prefers
/// the shared `config_effective` cache (managed overlay) and falls back to a
/// direct `fast_safe_load` parse; the cache lives in `hermes_cli` (upward
/// import impossible from this floor crate) — the port runs the upstream
/// FALLBACK path directly (same values when no managed overlay is set; the
/// overlay lands with the config crate). Parse goes through
/// `hermes_utils::fast_safe_load` (same safe-tag contract).
pub fn read_logging_config() -> (Option<String>, Option<u64>, Option<usize>) {
    let config_path = hermes_constants::get_config_path();
    if !config_path.exists() {
        return (None, None, None);
    }
    let text = match std::fs::read_to_string(&config_path) {
        Ok(t) => t,
        Err(_) => return (None, None, None),
    };
    let value: serde_yaml::Value = match hermes_utils::fast_safe_load(&text) {
        Ok(v) => v,
        Err(_) => return (None, None, None),
    };
    let Some(logging) = value.get("logging") else {
        return (None, None, None);
    };
    if !logging.is_mapping() {
        return (None, None, None);
    }
    let level = logging
        .get("level")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let max_size = logging.get("max_size_mb").and_then(|v| v.as_u64());
    let backup = logging
        .get("backup_count")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    (level, max_size, backup)
}

#[doc(hidden)]
pub fn reset_logging_for_tests() {
    LOGGING_INITIALIZED.store(false, Ordering::SeqCst);
    VERBOSE_INSTALLED.store(false, Ordering::SeqCst);
    VERBOSE_COUNT.store(0, Ordering::SeqCst);
    crate::queue::reset_queued_handlers();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn queue_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::queue::TEST_QUEUE_MUTEX
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    #[test]
    fn setup_creates_log_files() {
        let _g = queue_guard();
        reset_logging_for_tests();
        let td = TempDir::new().unwrap();
        let dir = setup_logging(SetupOptions {
            hermes_home: Some(td.path().to_path_buf()),
            mode: Some("gateway".to_string()),
            ..Default::default()
        });
        assert_eq!(dir, td.path().join("logs"));
        // Eager file creation matches Python handler construction.
        assert!(dir.join("agent.log").exists(), "agent.log");
        assert!(dir.join("errors.log").exists(), "errors.log");
        assert!(dir.join("gateway.log").exists(), "gateway.log");
        reset_logging_for_tests();
    }

    #[test]
    fn config_defaults_read() {
        let _g = queue_guard();
        let td = TempDir::new().unwrap();
        std::fs::write(
            td.path().join("config.yaml"),
            "logging:\n  level: DEBUG\n  max_size_mb: 9\n  backup_count: 4\n",
        )
        .unwrap();
        // Context-local override (thread-local — safer than mutating the
        // process env inside parallel unit tests; amateur-code fix over the
        // previous env-var version).
        let token = hermes_constants::set_hermes_home_override(Some(td.path()));
        let (lvl, mb, cnt) = read_logging_config();
        hermes_constants::reset_hermes_home_override(token);
        assert_eq!(lvl.as_deref(), Some("DEBUG"));
        assert_eq!(mb, Some(9));
        assert_eq!(cnt, Some(4));
    }

    #[test]
    fn config_absent_returns_none_triple() {
        // PARITY: `test_returns_none_when_no_config` (544–548).
        let _g = queue_guard();
        let td = TempDir::new().unwrap();
        let token = hermes_constants::set_hermes_home_override(Some(td.path()));
        let (lvl, mb, cnt) = read_logging_config();
        hermes_constants::reset_hermes_home_override(token);
        assert_eq!((lvl.as_deref(), mb, cnt), (None, None, None));
    }

    #[test]
    fn add_rotating_dedup_works_for_renamed_path() {
        // PARITY: Allen repro (646–682) at the dedup layer — the target path
        // no longer exists (renamed away) yet a second registration for the
        // same resolved path must still no-op (lexical resolve, no fs hit).
        let _g = queue_guard();
        reset_logging_for_tests();
        let td = TempDir::new().unwrap();
        let path: PathBuf = td.path().join("gateway.log");
        add_rotating_handler(path.clone(), Level::Info, 1024 * 1024, 3, None);
        assert_eq!(crate::queue::rotating_file_handlers().len(), 1);
        std::fs::rename(&path, td.path().join("gateway.log.1")).unwrap();
        add_rotating_handler(path, Level::Info, 1024 * 1024, 3, None);
        assert_eq!(
            crate::queue::rotating_file_handlers().len(),
            1,
            "missing-on-disk path must still dedup"
        );
        reset_logging_for_tests();
    }
}
