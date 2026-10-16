//! Converts history files written by earlier GNOME releases.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::*;

const GNOME_SCHEMA: &str = "
    CREATE TABLE samples(ts INTEGER NOT NULL, provider TEXT NOT NULL, window_id TEXT NOT NULL, label TEXT NOT NULL, used REAL NOT NULL, reset TEXT, PRIMARY KEY(ts,provider,window_id));
    CREATE INDEX samples_range ON samples(ts,provider);
    CREATE INDEX samples_latest ON samples(provider,window_id,ts DESC);
    CREATE TABLE daily_analytics(day TEXT NOT NULL, provider TEXT NOT NULL, metric TEXT NOT NULL, series TEXT NOT NULL, value REAL NOT NULL, PRIMARY KEY(day,provider,metric,series));
    CREATE INDEX analytics_range ON daily_analytics(metric,day);
    CREATE TABLE account_history(provider TEXT PRIMARY KEY, fingerprint TEXT NOT NULL);
    CREATE TABLE archived_samples(fingerprint TEXT NOT NULL, ts INTEGER NOT NULL, provider TEXT NOT NULL, window_id TEXT NOT NULL, label TEXT NOT NULL, used REAL NOT NULL, reset TEXT, PRIMARY KEY(fingerprint,ts,provider,window_id));
    CREATE INDEX archived_samples_ts ON archived_samples(ts);
    CREATE TABLE archived_analytics(fingerprint TEXT NOT NULL, day TEXT NOT NULL, provider TEXT NOT NULL, metric TEXT NOT NULL, series TEXT NOT NULL, value REAL NOT NULL, PRIMARY KEY(fingerprint,day,provider,metric,series));
    CREATE INDEX archived_analytics_day ON archived_analytics(day);
    CREATE TABLE claude_messages(id TEXT);
    INSERT INTO samples VALUES(1000,'claude','session','Current session',42,'2026-10-10T09:30:00.373598+00:00');
    INSERT INTO samples VALUES(1060,'claude','session','Current session',43,'not a date');
    INSERT INTO samples VALUES(1120,'codex','primary','5h',7,NULL);
    INSERT INTO daily_analytics VALUES('2026-10-10','codex','turns','total',12);
    INSERT INTO account_history VALUES('codex','first');
    INSERT INTO archived_samples VALUES('second',900,'codex','primary','5h',3,'2026-10-14T03:28:13+00:00');
    INSERT INTO archived_analytics VALUES('second','2026-10-09','codex','turns','total',5);";

fn gnome_file(folder: &Path) -> PathBuf {
    let path = folder.join("history.sqlite3");
    Connection::open(&path).unwrap().execute_batch(GNOME_SCHEMA).unwrap();
    path
}

fn keys(rows: &[Sample]) -> Vec<(&str, f64, f64, Option<f64>)> {
    rows.iter()
        .map(|row| (row.key.as_str(), row.at, row.used, row.reset_at))
        .collect()
}

#[test]
fn gnome_samples_keep_their_values_and_parse_reset_times() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(Some(&gnome_file(folder.path())), 60).unwrap();
    let rows = store.samples(query(0.0, 2000.0, None)).unwrap();
    assert_eq!(
        keys(&rows),
        [
            ("claude:session", 1000.0, 42.0, Some(1_791_624_600.374)),
            ("claude:session", 1060.0, 43.0, None),
            ("codex:primary", 1120.0, 7.0, None),
        ]
    );
}

#[test]
fn gnome_analytics_carry_over() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(Some(&gnome_file(folder.path())), 60).unwrap();
    let rows = store.analytics("codex", "2026-10-01", "2026-10-31").unwrap();
    assert_eq!(
        rows,
        [AnalyticsPoint {
            provider: "codex".into(),
            day: "2026-10-10".into(),
            metric: "turns".into(),
            series: "total".into(),
            value: 12.0,
        }]
    );
}

#[test]
fn gnome_archived_accounts_restore_after_conversion() {
    let folder = tempfile::tempdir().unwrap();
    let store = Store::open(Some(&gnome_file(folder.path())), 60).unwrap();
    store.activate_account("codex", "second").unwrap();
    let rows = store
        .samples(query(0.0, 2000.0, Some(&["codex:primary".into()])))
        .unwrap();
    assert_eq!(keys(&rows), [("codex:primary", 900.0, 3.0, Some(1_791_948_493.0))]);
    assert_eq!(
        store.analytics("codex", "2026-10-01", "2026-10-31").unwrap()[0].value,
        5.0
    );
}

#[test]
fn gnome_tables_are_replaced_by_the_shared_schema() {
    let folder = tempfile::tempdir().unwrap();
    let path = gnome_file(folder.path());
    drop(Store::open(Some(&path), 60).unwrap());
    let db = Connection::open(&path).unwrap();
    let tables: Vec<String> = db
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(
        tables,
        [
            "analytics",
            "analytics_accounts",
            "archived_analytics",
            "archived_samples",
            "samples"
        ]
    );
    let version: i32 = db.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    assert_eq!(version, SCHEMA_VERSION);
}

#[test]
fn failed_gnome_conversion_keeps_the_file_and_retries_on_the_next_open() {
    let folder = tempfile::tempdir().unwrap();
    let path = gnome_file(folder.path());
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE analytics(unrelated TEXT)").unwrap();
    assert!(Store::open(Some(&path), 60).is_err());
    let kept: i64 = db
        .query_row("SELECT COUNT(*) FROM samples", [], |row| row.get(0))
        .unwrap();
    assert_eq!(kept, 3);
    db.execute_batch("DROP TABLE analytics").unwrap();
    drop(db);
    let store = Store::open(Some(&path), 60).unwrap();
    assert_eq!(store.stats().unwrap().sample_count, 3);
}
