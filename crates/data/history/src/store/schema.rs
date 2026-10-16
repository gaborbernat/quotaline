//! Migrates existing history files to the current schema without discarding stored rows.

use rusqlite::{Connection, TransactionBehavior};

use super::{HistoryError, Result, SCHEMA_VERSION, legacy};

const INITIAL_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS samples (ts REAL NOT NULL, key TEXT NOT NULL, label TEXT NOT NULL, used REAL NOT NULL, resets_at REAL, PRIMARY KEY (key, ts));
    CREATE TABLE IF NOT EXISTS analytics (provider TEXT NOT NULL, day TEXT NOT NULL, metric TEXT NOT NULL, series TEXT NOT NULL, value REAL NOT NULL, PRIMARY KEY (provider, day, metric, series));
    CREATE TABLE IF NOT EXISTS analytics_accounts (provider TEXT PRIMARY KEY, fingerprint TEXT NOT NULL);
    CREATE INDEX IF NOT EXISTS samples_ts ON samples (ts);
    CREATE INDEX IF NOT EXISTS analytics_metric_day_provider ON analytics (metric, day, provider);
    CREATE INDEX IF NOT EXISTS analytics_day ON analytics (day);";

const ARCHIVE_SCHEMA: &str = "
    CREATE TABLE archived_samples (fingerprint TEXT NOT NULL, ts REAL NOT NULL, key TEXT NOT NULL, label TEXT NOT NULL, used REAL NOT NULL, resets_at REAL, PRIMARY KEY(fingerprint, key, ts));
    CREATE TABLE archived_analytics (fingerprint TEXT NOT NULL, provider TEXT NOT NULL, day TEXT NOT NULL, metric TEXT NOT NULL, series TEXT NOT NULL, value REAL NOT NULL, PRIMARY KEY(fingerprint, provider, day, metric, series));
    CREATE INDEX archived_samples_ts ON archived_samples(ts);
    CREATE INDEX archived_analytics_day ON archived_analytics(day);";

pub(super) fn migrate(db: &mut Connection) -> Result<()> {
    // Read the version inside the write lock so two processes opening the same file cannot both migrate it.
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: i32 = tx.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(HistoryError::Schema(version));
    }
    if version == SCHEMA_VERSION {
        return Ok(());
    }
    let gnome = version == 0 && legacy::is_gnome(&tx)?;
    if gnome {
        legacy::detach(&tx)?;
    }
    if version == 0 {
        tx.execute_batch(INITIAL_SCHEMA)?;
    }
    tx.execute_batch(ARCHIVE_SCHEMA)?;
    if gnome {
        legacy::import(&tx)?;
    }
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()?;
    Ok(())
}
