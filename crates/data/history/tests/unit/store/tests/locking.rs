//! Keeps writes working while another process holds the history file.

use std::{thread, time::Duration};

use rusqlite::Connection;

use super::*;

#[test]
fn writes_wait_for_another_connection_instead_of_failing() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("history.sqlite3");
    let store = Store::open(Some(&path), 60).unwrap();
    let other = Connection::open(&path).unwrap();
    other
        .execute_batch("BEGIN IMMEDIATE; INSERT INTO samples VALUES (500,'codex:primary','5h',1,NULL)")
        .unwrap();
    let release = thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        other.execute_batch("COMMIT").unwrap();
    });
    let rows = [sample(1000.0, 10.0, None)];
    let recorded = store.record_snapshot(snapshot("claude", Some("first"), 1000.0, &rows));
    release.join().unwrap();
    assert_eq!(recorded.unwrap(), 1);
}
