//! Parity tests for `hermes_cli/sqlite_util.py` @ b9aa928.
//!
//! Upstream has no dedicated test file (missing-test gap, noted in the
//! ledger); cases derive from the upstream code as oracle.

use rusqlite::Connection;

use hermes_cli::sqlite_util::{add_column_if_missing, write_txn};

fn open_memory() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY)")
        .unwrap();
    conn
}

#[test]
fn add_column_first_call_adds_second_call_reports_false() {
    let conn = open_memory();
    assert!(add_column_if_missing(&conn, "t", "note", "note TEXT").unwrap());
    // Idempotent: the concurrent-migrator duplicate is a false, not an
    // error (issue #21708).
    assert!(!add_column_if_missing(&conn, "t", "note", "note TEXT").unwrap());
    // The DDL landed and is usable.
    conn.execute_batch("INSERT INTO t (note) VALUES ('x')")
        .unwrap();
}

#[test]
fn add_column_accepts_full_ddl_definitions() {
    let conn = open_memory();
    assert!(add_column_if_missing(&conn, "t", "score", "score INTEGER DEFAULT 0").unwrap());
    conn.execute_batch("INSERT INTO t (score) VALUES (3)")
        .unwrap();
    let got: i64 = conn
        .query_row("SELECT score FROM t WHERE score = 3", [], |row| row.get(0))
        .unwrap();
    assert_eq!(got, 3);
}

#[test]
fn add_column_propagates_non_duplicate_errors() {
    let conn = open_memory();
    // A bad table name is a real OperationalError, not a duplicate — it
    // must propagate, not be swallowed as false.
    assert!(add_column_if_missing(&conn, "no_such_table", "c", "c TEXT").is_err());
}

#[test]
fn write_txn_commits_on_success() {
    let conn = open_memory();
    let result = write_txn(&conn, |c| {
        c.execute("INSERT INTO t (id) VALUES (1)", [])?;
        Ok(42)
    });
    assert_eq!(result.unwrap(), 42);
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM t", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "committed");
}

#[test]
fn write_txn_rolls_back_on_body_error_and_raises_original() {
    let conn = open_memory();
    let result: rusqlite::Result<String> = write_txn(&conn, |c| {
        c.execute("INSERT INTO t (id) VALUES (1)", [])?;
        Err(rusqlite::Error::InvalidQuery)
    });
    assert!(result.is_err(), "the original error propagates");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM t", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0, "rolled back");
    // The connection is still usable after the rollback (no dangling
    // transaction).
    conn.execute("INSERT INTO t (id) VALUES (2)", []).unwrap();
}

#[test]
fn write_txn_serializes_concurrent_writers() {
    // BEGIN IMMEDIATE: a second writer while one transaction is open
    // cannot interleave on the same connection; the guarded rollback path
    // keeps the first connection's error clean. Exercise the intent by
    // nesting attempts on two connections sharing one file.
    let td = tempfile::TempDir::new().unwrap();
    let path = td.path().join("shared.db");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY)")
        .unwrap();

    let guard_conn = Connection::open(&path).unwrap();
    guard_conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    guard_conn
        .execute("INSERT INTO t (id) VALUES (1)", [])
        .unwrap();

    let other = Connection::open(&path).unwrap();
    let result = write_txn(&other, |c| {
        c.execute("INSERT INTO t (id) VALUES (2)", [])?;
        Ok(())
    });
    assert!(result.is_err(), "IMMEDIATE writer loses to the open lock");
    drop(guard_conn);
}
