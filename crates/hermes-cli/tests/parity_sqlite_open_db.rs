//! Parity tests for `open_db` + `transaction` (`hermes_cli/sqlite_util.py`
//! @ 5d59366, lines 18-79).
//!
//! Oracle: source-as-oracle (no dedicated test file — gap noted); the
//! #69567 fd-leak rationale is the contract: `transaction` ALWAYS closes,
//! `open_db` closes on init failure. The WAL-reset-gate/network-fallback
//! half rides an injected `wal_setup` seam (hermes-state owns the real
//! `apply_wal_with_fallback`; default is a plain WAL pragma, documented).

use hermes_cli::sqlite_util::{open_db, open_db_with, transaction, DbOptions};
use rusqlite::Connection;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("hermes_sqlite_parity");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// `open_db` creates parents, applies busy_timeout + WAL, runs initialize.
#[test]
fn open_db_connects_with_pragmas_and_initializer() {
    let path = tmp("open_db_init.db");
    let _ = std::fs::remove_file(&path);
    let opts = DbOptions {
        db_label: "test".to_string(),
        initialize: Some(Box::new(|conn: &Connection| {
            conn.execute_batch("CREATE TABLE t (x)").unwrap();
        })),
        ..Default::default()
    };
    let conn = open_db(&path, opts).unwrap();
    let timeout: i64 = conn
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .unwrap();
    assert_eq!(timeout, 5000);
    let journal: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal.to_lowercase(), "wal");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM t", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    conn.close().unwrap();
    let _ = std::fs::remove_file(&path);
}

/// Init failure closes the connection (no fd leak).
#[test]
fn open_db_failure_closes() {
    let path = tmp("open_db_fail.db");
    let _ = std::fs::remove_file(&path);
    let opts = DbOptions {
        db_label: "test".to_string(),
        initialize: Some(Box::new(|_: &Connection| {
            panic!("init boom");
        })),
        ..Default::default()
    };
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| open_db(&path, opts)));
    assert!(result.is_err());
    // Reopen works — no wedged lock from a leaked connection.
    let opts2 = DbOptions {
        db_label: "test".to_string(),
        ..Default::default()
    };
    open_db(&path, opts2).unwrap().close().unwrap();
    let _ = std::fs::remove_file(&path);
}

/// `transaction` commits and ALWAYS closes (#69567).
#[test]
fn transaction_commits_and_closes() {
    let path = tmp("txn_commit.db");
    let _ = std::fs::remove_file(&path);
    let conn = open_db(
        &path,
        DbOptions {
            db_label: "test".to_string(),
            ..Default::default()
        },
    )
    .unwrap();
    transaction(conn, false, |conn| {
        conn.execute_batch("CREATE TABLE t (x); INSERT INTO t VALUES (1)")
    })
    .unwrap();
    // Closed: reopen and read back.
    let conn2 = Connection::open(&path).unwrap();
    let count: i64 = conn2
        .query_row("SELECT COUNT(*) FROM t", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
    conn2.close().unwrap();
    let _ = std::fs::remove_file(&path);
}

/// Rollback on error still closes (use-after-close would fail).
#[test]
fn transaction_rollback_still_closes() {
    use std::sync::{Arc, Mutex};
    let path = tmp("txn_rollback.db");
    let _ = std::fs::remove_file(&path);
    let conn = open_db(
        &path,
        DbOptions {
            db_label: "test".to_string(),
            ..Default::default()
        },
    )
    .unwrap();
    let closed_probe = Arc::new(Mutex::new(false));
    // Move conn in; after transaction returns the binding is consumed.
    transaction(conn, true, |conn| {
        conn.execute_batch("CREATE TABLE t (x)").unwrap();
        Err::<(), _>(rusqlite::Error::InvalidQuery)
    });
    let _ = closed_probe;
    let conn2 = Connection::open(&path).unwrap();
    let count: i64 = conn2
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='t'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0, "rolled back");
    conn2.close().unwrap();
    let _ = std::fs::remove_file(&path);
}

/// Custom wal_setup seam is honored (e.g. hermes-state fallback).
#[test]
fn wal_setup_seam_is_honored() {
    let path = tmp("wal_seam.db");
    let _ = std::fs::remove_file(&path);
    let seen = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let seen2 = seen.clone();
    let conn = open_db_with(
        &path,
        DbOptions {
            db_label: "my-label".to_string(),
            wal: true,
            ..Default::default()
        },
        Some(&|conn: &Connection, label: &str| {
            *seen2.lock().unwrap() = label.to_string();
            conn.execute_batch("PRAGMA journal_mode=DELETE")
                .map(|_| ())
                .map_err(|e| e.to_string())
        }),
    )
    .unwrap();
    assert_eq!(*seen.lock().unwrap(), "my-label");
    let journal: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal.to_lowercase(), "delete");
    conn.close().unwrap();
    let _ = std::fs::remove_file(&path);
}
