//! Top-level `setup_logging`/`setup_verbose_logging` + config defaults.
//!
//! PARITY: hermes_logging.py lines 259–413 (`setup_logging`,
//! `setup_verbose_logging`) and 762–800 (`_read_logging_config`).

use crate::queue::{register_queued_handler, register_queued_target, reset_queued_handlers};
use crate::record::Level;
use crate::rotating::{ComponentFilter, RotatingHandler};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static LOGGING_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Component logger-name prefixes used for gateway.log / gui.log routing.
/// Exposed for `hermes logs --component`.
///
/// PARITY: `COMPONENT_PREFIXES` (236–257).
pub const COMPONENT_PREFIXES: [(&str, &[&str]); 6] = [
    ("gateway", &["gateway", "hermes_plugins", "plugins.platforms"]),
    ("agent", &["agent", "run_agent", "model_tools", "batch_runner"]),
    ("tools", &["tools"]),
    ("cli", &["hermes_cli", "cli"]),
    ("cron", &["cron"]),
    ("gui", &["hermes_cli.web_server", "hermes_cli.pty_bridge", "tui_gateway", "uvicorn"]),
];

/// Third-party loggers suppressed at DEBUG/INFO level.
///
/// PARITY: `_NOISY_LOGGERS` (143–162).
pub const NOISY_LOGGERS: [&str; 14] = [
    "openai", "openai._base_client", "httpx", "httpcore", "asyncio", "hpack",
    "hpack.hpack", "grpc", "modal", "urllib3", "urllib3.connectionpool",
    "websockets", "charset_normalizer", "markdown_it",
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
/// PARITY: hermes_logging.py `setup_logging` (259–363).
pub fn setup_logging(opts: SetupOptions) -> PathBuf {
    let home = opts.hermes_home.clone().unwrap_or_else(hermes_constants::get_hermes_home);
    let log_dir = home.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);

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

    // agent.log (INFO+ — the main activity log)
    add_rotating_handler(
        log_dir.join("agent.log"),
        level,
        max_bytes,
        backups,
        None,
    );

    // errors.log (WARNING+ — quick triage log): fixed 2MB / 2 backups.
    add_rotating_handler(
        log_dir.join("errors.log"),
        Level::Warning,
        2 * 1024 * 1024,
        2,
        None,
    );

    // gateway.log (INFO+, gateway component only)
    if opts.mode.as_deref() == Some("gateway") {
        add_rotating_handler(
            log_dir.join("gateway.log"),
            Level::Info,
            5 * 1024 * 1024,
            3,
            Some(ComponentFilter {
                prefixes: COMPONENT_PREFIXES
                    .iter()
                    .find(|(name, _)| *name == "gateway")
                    .map(|(_, p)| p.iter().map(|s| s.to_string()).collect())
                    .unwrap_or_default(),
            }),
        );
    }

    // gui.log (INFO+, dashboard/tui-gateway components)
    if opts.mode.as_deref() == Some("gui") {
        add_rotating_handler(
            log_dir.join("gui.log"),
            Level::Info,
            10 * 1024 * 1024,
            5,
            Some(ComponentFilter {
                prefixes: COMPONENT_PREFIXES
                    .iter()
                    .find(|(name, _)| *name == "gui")
                    .map(|(_, p)| p.iter().map(|s| s.to_string()).collect())
                    .unwrap_or_default(),
            }),
        );
    }

    if LOGGING_INITIALIZED.load(Ordering::SeqCst) && !opts.force {
        return log_dir;
    }

    LOGGING_INITIALIZED.store(true, Ordering::SeqCst);
    log_dir
}

/// Enable DEBUG-level console logging for `--verbose` / `-v` mode.
///
/// The Rust port writes verbose records to stderr with the verbose format.
///
/// PARITY: hermes_logging.py `setup_verbose_logging` (379–413).
pub fn setup_verbose_logging() {
    // Route verbose records through a dedicated stderr handler on the async
    // queue so the emitting thread never blocks on console I/O.
    let handler: Arc<dyn crate::record::LogTarget> = Arc::new(VerboseStderrHandler::new());
    register_queued_target(handler);
    LOGGING_INITIALIZED.store(true, Ordering::SeqCst);
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

/// Add a `RotatingFileHandler`, skipping if one already exists for the same
/// resolved file path (idempotent).
///
/// PARITY: `_add_rotating_handler` (721–759).
pub fn add_rotating_handler(
    path: PathBuf,
    level: Level,
    max_bytes: u64,
    backup_count: usize,
    component: Option<ComponentFilter>,
) {
    for existing in crate::queue::rotating_file_handlers() {
        // Idempotency check: same resolved path.
        if let Ok(meta) = std::fs::canonicalize(&path) {
            let _ = &existing;
            if let Ok(existing_meta) = std::fs::canonicalize(&existing.path) {
                if existing_meta == meta {
                    return;
                }
            }
        }
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let handler = match RotatingHandler::new(&path, level, max_bytes, backup_count, component) {
        Ok(h) => Arc::new(h),
        Err(_) => return, // logging must never crash startup
    };
    register_queued_handler(handler);
}

/// Best-effort read of `logging.*` from config.yaml.
///
/// Returns `(level, max_size_mb, backup_count)` — any may be `None`.
///
/// PARITY: `_read_logging_config` (762–800). Managed-scope overlay is a
/// no-op until the managed-scope subsystem is ported (P3 — upstream applies
/// it fail-open, and the identity overlay is behaviorally equivalent when no
/// managed scope is configured).
pub fn read_logging_config() -> (Option<String>, Option<u64>, Option<usize>) {
    let config_path = hermes_constants::get_config_path();
    if !config_path.exists() {
        return (None, None, None);
    }
    let text = match std::fs::read_to_string(&config_path) {
        Ok(t) => t,
        Err(_) => return (None, None, None),
    };
    let value: serde_yaml::Value = match serde_yaml::from_str(&text) {
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
    let backup = logging.get("backup_count").and_then(|v| v.as_u64()).map(|v| v as usize);
    (level, max_size, backup)
}

#[doc(hidden)]
pub fn reset_logging_for_tests() {
    LOGGING_INITIALIZED.store(false, Ordering::SeqCst);
    reset_queued_handlers();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn setup_creates_log_files() {
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
        let td = TempDir::new().unwrap();
        std::fs::write(
            td.path().join("config.yaml"),
            "logging:\n  level: DEBUG\n  max_size_mb: 9\n  backup_count: 4\n",
        )
        .unwrap();
        // read_logging_config reads HERMES_HOME-resolved config path; point it
        // there via env.
        unsafe { std::env::set_var("HERMES_HOME", td.path()) };
        let (lvl, mb, cnt) = read_logging_config();
        assert_eq!(lvl.as_deref(), Some("DEBUG"));
        assert_eq!(mb, Some(9));
        assert_eq!(cnt, Some(4));
        unsafe { std::env::remove_var("HERMES_HOME") };
    }
}
