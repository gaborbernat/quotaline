//! Converts history written by earlier GNOME releases, which used its own table layout, into the shared schema.

use rusqlite::{OptionalExtension, Transaction};

pub(super) fn is_gnome(tx: &Transaction<'_>) -> rusqlite::Result<bool> {
    tx.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name IN ('daily_analytics','account_history') \
         UNION SELECT 1 FROM pragma_table_info('samples') WHERE name='window_id'",
        [],
        |_| Ok(()),
    )
    .optional()
    .map(|found| found.is_some())
}

pub(super) fn detach(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    // The shared schema reuses the archive index names, and renaming a table keeps its indexes.
    tx.execute_batch(
        "DROP INDEX IF EXISTS samples_range; DROP INDEX IF EXISTS samples_latest; DROP INDEX IF EXISTS analytics_range;
         DROP INDEX IF EXISTS archived_samples_ts; DROP INDEX IF EXISTS archived_analytics_day;
         CREATE TABLE IF NOT EXISTS samples(ts INTEGER NOT NULL, provider TEXT NOT NULL, window_id TEXT NOT NULL, label TEXT NOT NULL, used REAL NOT NULL, reset TEXT);
         CREATE TABLE IF NOT EXISTS daily_analytics(day TEXT NOT NULL, provider TEXT NOT NULL, metric TEXT NOT NULL, series TEXT NOT NULL, value REAL NOT NULL);
         CREATE TABLE IF NOT EXISTS account_history(provider TEXT PRIMARY KEY, fingerprint TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS archived_samples(fingerprint TEXT NOT NULL, ts INTEGER NOT NULL, provider TEXT NOT NULL, window_id TEXT NOT NULL, label TEXT NOT NULL, used REAL NOT NULL, reset TEXT);
         CREATE TABLE IF NOT EXISTS archived_analytics(fingerprint TEXT NOT NULL, day TEXT NOT NULL, provider TEXT NOT NULL, metric TEXT NOT NULL, series TEXT NOT NULL, value REAL NOT NULL);
         ALTER TABLE samples RENAME TO gnome_samples;
         ALTER TABLE archived_samples RENAME TO gnome_archived_samples;
         ALTER TABLE archived_analytics RENAME TO gnome_archived_analytics;",
    )
}

pub(super) fn import(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    // GNOME stored resets as RFC 3339 text; unixepoch returns NULL for text it cannot parse, which the shared
    // schema already uses for an unknown reset.
    tx.execute_batch(
        "INSERT OR REPLACE INTO samples SELECT ts,provider||':'||window_id,label,used,unixepoch(reset,'subsec') FROM gnome_samples;
         INSERT OR REPLACE INTO archived_samples SELECT fingerprint,ts,provider||':'||window_id,label,used,unixepoch(reset,'subsec') FROM gnome_archived_samples;
         INSERT OR REPLACE INTO analytics SELECT provider,day,metric,series,value FROM daily_analytics;
         INSERT OR REPLACE INTO archived_analytics SELECT fingerprint,provider,day,metric,series,value FROM gnome_archived_analytics;
         INSERT OR REPLACE INTO analytics_accounts SELECT provider,fingerprint FROM account_history;
         DROP TABLE gnome_samples; DROP TABLE gnome_archived_samples; DROP TABLE gnome_archived_analytics;
         DROP TABLE daily_analytics; DROP TABLE account_history;
         DROP TABLE IF EXISTS claude_messages; DROP TABLE IF EXISTS claude_offsets;",
    )
}
