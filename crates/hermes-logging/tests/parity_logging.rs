//! End-to-end parity checks for the logging contract vs upstream
//! `tests/test_hermes_logging.py` behaviors (@ b9aa928):
//!   - setup_logging creates agent.log / errors.log (+ gateway.log in
//!     gateway mode), eagerly, under <home>/logs/
//!   - INFO records reach agent.log; DEBUG records do not (default level)
//!   - WARNING records reach both agent.log and errors.log
//!   - session context tags records `[session_id]`
//!   - records from other components are excluded from gateway.log
//!   - flush_log_queue drains before reads
//!   - rotation produces .1 backups and keeps backup_count bounded

use hermes_logging::{
    clear_session_context, log, rotating_file_handlers, set_session_context, setup_logging,
    setup::SetupOptions, Level, flush_log_queue, reset_queued_handlers,
};
use std::sync::Mutex;

static M: Mutex<()> = Mutex::new(());

fn read(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn logging_contract_end_to_end() {
    let _g = M.lock().unwrap();
    reset_queued_handlers();
    let td = tempfile::TempDir::new().unwrap();
    let home = td.path().to_path_buf();

    let dir = setup_logging(SetupOptions {
        hermes_home: Some(home.clone()),
        mode: Some("gateway".to_string()),
        ..Default::default()
    });
    assert!(dir.join("agent.log").exists());
    assert!(dir.join("errors.log").exists());
    assert!(dir.join("gateway.log").exists());

    // INFO record -> agent.log only (errors.log is WARNING+).
    clear_session_context();
    log(Level::Info, "agent.runtime", "info line");
    // WARNING -> both.
    log(Level::Warning, "agent.runtime", "warn line");
    // DEBUG -> filtered out everywhere at default INFO level.
    log(Level::Debug, "agent.runtime", "debug line");
    // Session-tagged INFO.
    set_session_context("ses-1");
    log(Level::Info, "agent.runtime", "tagged line");
    // Gateway-component record only goes to gateway.log AND agent.log.
    log(Level::Info, "gateway.run", "gateway only line");

    flush_log_queue();

    let agent = read(&dir.join("agent.log"));
    assert!(agent.contains("info line"), "agent.log: {}", agent);
    assert!(agent.contains("warn line"), "agent.log: {}", agent);
    assert!(!agent.contains("debug line"), "DEBUG filtered at INFO level: {}", agent);
    assert!(agent.contains(" INFO [ses-1] agent.runtime: tagged line"), "agent.log: {}", agent);
    assert!(agent.contains("gateway only line"), "agent.log: {}", agent);

    let errors = read(&dir.join("errors.log"));
    assert!(!errors.contains("info line"), "errors.log must be WARNING+: {}", errors);
    assert!(errors.contains("warn line"), "errors.log: {}", errors);

    let gate = read(&dir.join("gateway.log"));
    assert!(gate.contains("gateway only line"), "gateway.log: {}", gate);
    assert!(!gate.contains("info line"), "gateway.log must be component-filtered: {}", gate);
    assert!(!gate.contains("warn line"), "gateway.log must be component-filtered: {}", gate);

    clear_session_context();
    reset_queued_handlers();
}

#[test]
fn rotation_bounded_and_present() {
    let _g = M.lock().unwrap();
    reset_queued_handlers();
    let td = tempfile::TempDir::new().unwrap();
    // Small 40KB log, 2 backups: force several rotations with verbose output.
    let home = td.path().to_path_buf();

    // Use add_rotating_handler directly with a tiny max size for deterministic
    // rotation in the test, then emit many lines.
    use hermes_logging::add_rotating_handler;
    let dir = home.join("logs");
    add_rotating_handler(dir.join("rot.log"), Level::Info, 1024, 2, None);
    for i in 0..200 {
        log(Level::Info, "rotation", format!("rot line {:03} {}", i, "x".repeat(80)));
    }
    flush_log_queue();

    assert!(dir.join("rot.log").exists());
    assert!(dir.join("rot.log.1").exists(), "at least one backup");
    // bounded by backup_count=2: no .3
    assert!(!dir.join("rot.log.3").exists(), "backup_count=2 must not create .3");
    reset_queued_handlers();
}

#[test]
fn rotating_file_handlers_lists_registered() {
    let _g = M.lock().unwrap();
    reset_queued_handlers();
    let td = tempfile::TempDir::new().unwrap();
    let _dir = setup_logging(SetupOptions {
        hermes_home: Some(td.path().to_path_buf()),
        mode: Some("gui".to_string()),
        ..Default::default()
    });
    // agent.log + errors.log + gui.log
    assert_eq!(rotating_file_handlers().len(), 3);
    reset_queued_handlers();
}

#[test]
fn log_macros_work() {
    hermes_logging::info_log!("test.target", "macro info {}", 42);
    hermes_logging::warn_log!("test.target", "macro warn");
    hermes_logging::error_log!("test.target", "macro error");
    hermes_logging::debug_log!("test.target", "macro debug");
    // No-op assertions are implicit: the macros route into the queue which is
    // absent here (records dropped) — the key check is they compile + don't panic.
    let _ = Level::Debug;
}
