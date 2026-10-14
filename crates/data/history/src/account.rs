//! Switches the active account for a provider while archiving the previous account's rows.

use rusqlite::{OptionalExtension, Transaction, params};

pub(crate) fn active(tx: &Transaction<'_>, provider: &str) -> rusqlite::Result<Option<String>> {
    tx.query_row(
        "SELECT fingerprint FROM analytics_accounts WHERE provider=?1",
        [provider],
        |row| row.get(0),
    )
    .optional()
}

pub(crate) fn activate(tx: &Transaction<'_>, provider: &str, fingerprint: &str) -> rusqlite::Result<bool> {
    let previous = active(tx, provider)?;
    if previous.as_deref() == Some(fingerprint) {
        return Ok(false);
    }
    let keys = format!("{provider}:*");
    if let Some(previous) = previous {
        tx.execute(
            "INSERT OR REPLACE INTO archived_samples SELECT ?1,ts,key,label,used,resets_at FROM samples WHERE key GLOB ?2",
            params![previous, keys],
        )?;
        tx.execute(
            "INSERT OR REPLACE INTO archived_analytics SELECT ?1,provider,day,metric,series,value FROM analytics WHERE provider=?2",
            params![previous, provider],
        )?;
        tx.execute("DELETE FROM samples WHERE key GLOB ?1", [&keys])?;
        tx.execute("DELETE FROM analytics WHERE provider=?1", [provider])?;
        tx.execute(
            "INSERT OR REPLACE INTO samples SELECT ts,key,label,used,resets_at FROM archived_samples WHERE fingerprint=?1 AND key GLOB ?2",
            params![fingerprint, keys],
        )?;
        tx.execute(
            "INSERT OR REPLACE INTO analytics SELECT provider,day,metric,series,value FROM archived_analytics WHERE fingerprint=?1 AND provider=?2",
            params![fingerprint, provider],
        )?;
        tx.execute(
            "DELETE FROM archived_samples WHERE fingerprint=?1 AND key GLOB ?2",
            params![fingerprint, keys],
        )?;
        tx.execute(
            "DELETE FROM archived_analytics WHERE fingerprint=?1 AND provider=?2",
            params![fingerprint, provider],
        )?;
    }
    tx.execute(
        "INSERT OR REPLACE INTO analytics_accounts VALUES(?1,?2)",
        params![provider, fingerprint],
    )?;
    Ok(true)
}
