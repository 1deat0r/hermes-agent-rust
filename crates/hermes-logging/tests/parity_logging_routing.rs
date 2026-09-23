//! Oracle mirrors for `tests/test_hermes_logging.py` setup/routing/mode
//! behaviors @ 5d59366 (TestSetupLogging, profile routing ×4, explicit
//! config override, gateway/gui mode, verbose idempotency, session factory,
//! add_rotating idempotency).
//!
//! Skipped upstream cases (documented in PLAN.md §7):
//! - `test_managed_mode_initial_open_sets_group_writable` — managed 0660
//!   chmod deferred with the config crate (PORT SEAMS).
//! - `TestWindowsConcurrentLogLockTimeout` windows_only rows — no CLH on
//!   POSIX; the linux_only inert row lives in the rotating unit tests.
//! - `TestLogIsolation` — pytest conftest import-time sandbox guard; Rust
//!   tests pass explicit tmp homes and never call setup at import.
//! - `TestAsyncQueueLogging` root/QueueHandler shape — custom queue replaces
//!   the Python logging stack (accessor behavior still covered here).

use hermes_logging::{
    clear_session_context, enable_profile_log_routing, flush_log_queue, log,
    rotating_file_handlers, set_session_context, setup::reset_logging_for_tests,
    setup::verbose_handler_count, setup_logging, setup_verbose_logging, Level, SetupOptions,
};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

static M: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    M.lock().unwrap_or_else(|p| p.into_inner())
}

fn read(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn setup(home: &std::path::Path, mode: Option<&str>) -> PathBuf {
    setup_logging(SetupOptions {
        hermes_home: Some(home.to_path_buf()),
        mode: mode.map(|m| m.to_string()),
        ..Default::default()
    })
}

fn count_handlers_named(name: &str) -> usize {
    rotating_file_handlers()
        .iter()
        .filter(|h| h.path.file_name().map(|n| n == name).unwrap_or(false))
        .count()
}

#[test]
fn creates_log_directory() {
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    let dir = setup(td.path(), None);
    assert_eq!(dir, td.path().join("logs"));
    assert!(dir.is_dir());
    reset_logging_for_tests();
}

#[test]
fn creates_agent_log_handler_at_info() {
    // PARITY: test_creates_agent_log_handler (76–86).
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    setup(td.path(), None);
    assert_eq!(count_handlers_named("agent.log"), 1);
    let handlers = rotating_file_handlers();
    let agent = handlers
        .iter()
        .find(|h| {
            h.path
                .file_name()
                .map(|n| n == "agent.log")
                .unwrap_or(false)
        })
        .expect("agent handler");
    assert_eq!(agent.level, Level::Info);
    reset_logging_for_tests();
}

#[test]
fn idempotent_no_duplicate_handlers() {
    // PARITY: test_idempotent_no_duplicate_handlers (89–99).
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    setup(td.path(), None);
    setup(td.path(), None);
    assert_eq!(count_handlers_named("agent.log"), 1);
    reset_logging_for_tests();
}

#[test]
fn writes_to_agent_log() {
    // PARITY: test_writes_to_agent_log (105–117).
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    let dir = setup(td.path(), None);
    log(
        Level::Info,
        "test_hermes_logging.write_test",
        "test message for agent.log",
    );
    flush_log_queue();
    assert!(read(&dir.join("agent.log")).contains("test message for agent.log"));
    reset_logging_for_tests();
}

#[test]
fn profile_routing_follows_context_home() {
    // PARITY: test_profile_routing_follows_context_home (119–143) — a cron
    // record stamped with an override home lands in THAT profile's files.
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    let home_a = td.path().join("home-a");
    let home_b = td.path().join("profile-b");
    std::fs::create_dir_all(&home_b).unwrap();

    setup(&home_a, None);
    assert!(enable_profile_log_routing(&[
        home_a.clone(),
        home_b.clone()
    ]));

    let token = hermes_constants::set_hermes_home_override(Some(&home_b));
    log(
        Level::Info,
        "cron.scheduler.profile-routing-test",
        "profile-routed cron record",
    );
    hermes_constants::reset_hermes_home_override(token);
    flush_log_queue();

    assert!(
        read(&home_b.join("logs/agent.log")).contains("profile-routed cron record"),
        "record must land in the profile home"
    );
    assert!(
        !read(&home_a.join("logs/agent.log")).contains("profile-routed cron record"),
        "record must not leak into the launch home"
    );
    reset_logging_for_tests();
}

#[test]
fn second_home_routes_instead_of_stacking_an_unfiltered_handler() {
    // PARITY: test_a_second_home_routes_instead_of_stacking_an_unfiltered_handler
    // (145–173): the second setup_logging adopts routing; no bare handler
    // remains; records isolate per stamped home.
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    let home_a = td.path().join("home-a");
    let home_b = td.path().join("profile-b");
    std::fs::create_dir_all(&home_b).unwrap();

    setup(&home_a, None);
    setup(&home_b, None); // adopt → routing on for {a, b}

    assert!(
        rotating_file_handlers().is_empty(),
        "the second home must not add an unfiltered file handler"
    );

    let token = hermes_constants::set_hermes_home_override(Some(&home_b));
    log(
        Level::Info,
        "agent.conversation_loop.second-home-test",
        "turn of profile b",
    );
    hermes_constants::reset_hermes_home_override(token);
    let token = hermes_constants::set_hermes_home_override(Some(&home_a));
    log(
        Level::Info,
        "agent.conversation_loop.second-home-test",
        "turn of the launch profile",
    );
    hermes_constants::reset_hermes_home_override(token);
    flush_log_queue();

    let a_log = read(&home_a.join("logs/agent.log"));
    let b_log = read(&home_b.join("logs/agent.log"));
    assert!(
        b_log.contains("turn of profile b") && !b_log.contains("turn of the launch profile"),
        "b: {}",
        b_log
    );
    assert!(
        a_log.contains("turn of the launch profile") && !a_log.contains("turn of profile b"),
        "a: {}",
        a_log
    );
    reset_logging_for_tests();
}

#[test]
fn setup_for_an_already_routed_home_adds_no_duplicate_writer() {
    // PARITY: test_setup_for_an_already_routed_home_adds_no_duplicate_writer
    // (175–197): routing live, then setup for a known profile home — router
    // dedup means exactly one write per record.
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    let home_a = td.path().join("home-a");
    let home_b = td.path().join("profile-b");
    std::fs::create_dir_all(&home_b).unwrap();

    setup(&home_a, None);
    assert!(enable_profile_log_routing(&[
        home_a.clone(),
        home_b.clone()
    ]));
    setup(&home_b, None);

    assert!(
        rotating_file_handlers().is_empty(),
        "routing already on — no bare handler may stack on the router"
    );

    let token = hermes_constants::set_hermes_home_override(Some(&home_b));
    log(
        Level::Info,
        "agent.conversation_loop.routed-home-test",
        "once please",
    );
    hermes_constants::reset_hermes_home_override(token);
    flush_log_queue();

    assert_eq!(
        read(&home_b.join("logs/agent.log"))
            .matches("once please")
            .count(),
        1,
        "router must write exactly once"
    );
    assert!(!read(&home_a.join("logs/agent.log")).contains("once please"));
    reset_logging_for_tests();
}

#[test]
fn a_component_log_added_after_routing_is_routed_too() {
    // PARITY: test_a_component_log_added_after_routing_is_routed_too
    // (199–221): mode="gateway" after routing wraps gateway.log into a
    // router instead of stacking a cross-home writer.
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    let home_a = td.path().join("home-a");
    let home_b = td.path().join("profile-b");
    std::fs::create_dir_all(&home_b).unwrap();

    setup(&home_a, None);
    setup(&home_b, None);
    setup(&home_a, Some("gateway"));

    assert!(
        rotating_file_handlers().is_empty(),
        "gateway.log after routing must be a routed writer, not a bare handler"
    );

    let token = hermes_constants::set_hermes_home_override(Some(&home_b));
    log(Level::Info, "gateway.run.routed-component-test", "gw-b");
    hermes_constants::reset_hermes_home_override(token);
    let token = hermes_constants::set_hermes_home_override(Some(&home_a));
    log(Level::Info, "gateway.run.routed-component-test", "gw-a");
    hermes_constants::reset_hermes_home_override(token);
    flush_log_queue();

    let a_log = read(&home_a.join("logs/gateway.log"));
    assert!(
        a_log.contains("gw-a") && !a_log.contains("gw-b"),
        "a: {}",
        a_log
    );
    let b_log = read(&home_b.join("logs/gateway.log"));
    assert!(b_log.contains("gw-b"), "b: {}", b_log);
    reset_logging_for_tests();
}

#[test]
fn explicit_params_override_config() {
    // PARITY: test_explicit_params_override_config (226–240). The config
    // path resolves through get_hermes_home → context-local override (the
    // env-var equivalent of upstream's conftest HERMES_HOME).
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    std::fs::write(td.path().join("config.yaml"), "logging:\n  level: DEBUG\n").unwrap();
    let token = hermes_constants::set_hermes_home_override(Some(td.path()));
    setup_logging(SetupOptions {
        hermes_home: Some(td.path().to_path_buf()),
        log_level: Some("WARNING".to_string()),
        ..Default::default()
    });
    hermes_constants::reset_hermes_home_override(token);

    let handlers = rotating_file_handlers();
    let agent = handlers
        .iter()
        .find(|h| {
            h.path
                .file_name()
                .map(|n| n == "agent.log")
                .unwrap_or(false)
        })
        .expect("agent handler");
    assert_eq!(agent.level, Level::Warning);
    reset_logging_for_tests();
}

#[test]
fn gateway_log_created_and_filtered() {
    // PARITY: TestGatewayMode (244–301) — created in gateway mode, absent
    // in cli mode, receives gateway components, rejects tools/agent.
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();

    setup(td.path(), Some("gateway"));
    assert_eq!(count_handlers_named("gateway.log"), 1);
    log(
        Level::Info,
        "plugins.platforms.telegram.adapter",
        "telegram connected",
    );
    log(Level::Info, "tools.terminal_tool", "running command");
    log(
        Level::Info,
        "agent.context_compressor",
        "compressing context",
    );
    flush_log_queue();
    let gw = read(&td.path().join("logs/gateway.log"));
    assert!(gw.contains("telegram connected"), "gw: {}", gw);
    assert!(!gw.contains("running command"), "gw: {}", gw);
    assert!(!gw.contains("compressing context"), "gw: {}", gw);
    reset_logging_for_tests();

    // cli mode: no gateway.log at all (eager creation is mode-gated).
    let td = tempfile::TempDir::new().unwrap();
    setup(td.path(), Some("cli"));
    assert_eq!(count_handlers_named("gateway.log"), 0);
    assert!(!td.path().join("logs/gateway.log").exists());
    reset_logging_for_tests();
}

#[test]
fn gui_log_receives_only_gui_components() {
    // PARITY: TestGuiMode (304–333).
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    setup(td.path(), Some("gui"));
    assert_eq!(count_handlers_named("gui.log"), 1);

    log(Level::Info, "hermes_cli.web_server", "dashboard online");
    log(Level::Info, "tui_gateway.ws", "ws connected");
    log(Level::Info, "gateway.run", "gateway event");
    flush_log_queue();

    let gui = read(&td.path().join("logs/gui.log"));
    assert!(gui.contains("dashboard online"), "gui: {}", gui);
    assert!(gui.contains("ws connected"), "gui: {}", gui);
    assert!(!gui.contains("gateway event"), "gui: {}", gui);
    reset_logging_for_tests();
}

#[test]
fn session_tag_in_log_output() {
    // PARITY: TestSessionContext.test_session_tag_in_log_output (339–352).
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    let dir = setup(td.path(), None);
    set_session_context("abc123");
    log(Level::Info, "test.session_tag", "tagged message");
    flush_log_queue();
    clear_session_context();
    let content = read(&dir.join("agent.log"));
    assert!(content.contains("[abc123]"), "agent: {}", content);
    assert!(content.contains("tagged message"), "agent: {}", content);
    reset_logging_for_tests();
}

#[test]
fn no_session_filter_on_handler() {
    // PARITY: test_no_session_filter_on_handler (427–450) — session rides
    // the record factory, never a per-handler filter (component stays None
    // for a plain path; the tag still formats).
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    let log_path = td.path().join("no_session_filter.log");
    hermes_logging::add_rotating_handler(log_path.clone(), Level::Info, 1024, 1, None);
    let handlers = rotating_file_handlers();
    let h = handlers
        .iter()
        .find(|h| h.path == log_path)
        .expect("handler");
    assert!(
        h.component.is_none(),
        "plain handler carries no component/session filter"
    );
    set_session_context("factory_test");
    log(Level::Info, "_test_no_session_filter", "test msg");
    flush_log_queue();
    clear_session_context();
    let content = read(&log_path);
    assert!(content.contains("[factory_test]"), "content: {}", content);
    reset_logging_for_tests();
}

#[test]
fn add_rotating_no_duplicate_for_same_path() {
    // PARITY: test_no_duplicate_for_same_path (405–425).
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    let log_path = td.path().join("test.log");
    hermes_logging::add_rotating_handler(log_path.clone(), Level::Info, 1024, 1, None);
    hermes_logging::add_rotating_handler(log_path, Level::Info, 1024, 1, None);
    assert_eq!(rotating_file_handlers().len(), 1);
    reset_logging_for_tests();
}

#[test]
fn verbose_stream_handler_added_once() {
    // PARITY: TestSetupVerboseLogging (382–398) — exactly one verbose
    // handler; a second call is guarded by the `_hermes_verbose` marker.
    let _g = lock();
    reset_logging_for_tests();
    let td = tempfile::TempDir::new().unwrap();
    setup(td.path(), None);
    setup_verbose_logging();
    setup_verbose_logging();
    assert_eq!(verbose_handler_count(), 1);
    reset_logging_for_tests();
}
