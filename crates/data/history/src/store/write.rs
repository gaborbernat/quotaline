//! Records and prunes history rows.

use std::collections::BTreeMap;

use quotaline_analytics::UsageSample;
use quotaline_analytics::should_record_usage;
use quotaline_analytics::utc_day::{day, retained_utc_days};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

use super::{AnalyticsPoint, CoverageScope, PruneResult, Result, SampleWrite, SnapshotWrite, State, Store};
use crate::account;

// Deferred transactions that read before writing fail with SQLITE_BUSY instead of waiting when another connection
// commits in between, so every write takes the lock up front.
fn write(state: &mut State) -> rusqlite::Result<Transaction<'_>> {
    state.db.transaction_with_behavior(TransactionBehavior::Immediate)
}

impl Store {
    pub fn active_account(&self, provider: &str) -> Result<Option<String>> {
        let mut state = self.state();
        Ok(account::active(&state.db.transaction()?, provider)?)
    }

    pub fn activate_account(&self, provider: &str, fingerprint: &str) -> Result<()> {
        let mut state = self.state();
        let tx = write(&mut state)?;
        account::activate(&tx, provider, fingerprint)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_snapshot(&self, input: SnapshotWrite<'_>) -> Result<u64> {
        let SnapshotWrite {
            provider,
            fingerprint,
            now,
            fetched_at,
            network,
            credit_balance,
            credit_unlimited,
            windows,
        } = input;
        let mut state = self.state();
        let tx = write(&mut state)?;
        if let Some(fingerprint) = fingerprint {
            account::activate(&tx, provider, fingerprint)?;
        }
        let mut recorded = 0;
        {
            let mut latest =
                tx.prepare_cached("SELECT ts,used,resets_at FROM samples WHERE key=?1 ORDER BY ts DESC LIMIT 1")?;
            let mut insert = tx.prepare_cached(
                "INSERT OR REPLACE INTO samples (ts,key,label,used,resets_at) VALUES (?1,?2,?3,?4,?5)",
            )?;
            for row in windows {
                let previous: Option<(f64, f64, Option<f64>)> = latest
                    .query_row([&row.sample.key], |result| {
                        Ok((result.get(0)?, result.get(1)?, result.get(2)?))
                    })
                    .optional()?;
                let reset_changed = previous
                    .as_ref()
                    .is_some_and(|(_, _, reset)| *reset != row.sample.reset_at);
                if !should_record_usage(
                    UsageSample {
                        at: now,
                        used: row.sample.used,
                    },
                    previous.map(|(at, used, _)| UsageSample { at, used }),
                    reset_changed,
                ) {
                    continue;
                }
                insert.execute(params![
                    now,
                    row.sample.key,
                    row.label,
                    row.sample.used,
                    row.sample.reset_at
                ])?;
                recorded += 1;
            }
        }
        if provider == "codex"
            && network
            && !credit_unlimited
            && let Some(balance) = credit_balance
        {
            tx.execute(
                "INSERT OR REPLACE INTO analytics VALUES (?1,?2,'creditBalance','Available credits',?3)",
                params![provider, day(fetched_at), balance],
            )?;
            recorded += 1;
        }
        tx.commit()?;
        prune_if_needed(&mut state, now)?;
        Ok(recorded)
    }

    pub fn seed(&self, rows: &[SampleWrite]) -> Result<()> {
        let mut state = self.state();
        let tx = write(&mut state)?;
        {
            let mut insert = tx.prepare_cached(
                "INSERT OR REPLACE INTO samples (ts,key,label,used,resets_at) VALUES (?1,?2,?3,?4,?5)",
            )?;
            for row in rows {
                insert.execute(params![
                    row.sample.at,
                    row.sample.key,
                    row.label,
                    row.sample.used,
                    row.sample.reset_at
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn record_analytics(
        &self,
        provider: &str,
        fingerprint: Option<&str>,
        fetched_at: f64,
        points: &[AnalyticsPoint],
        scopes: &[CoverageScope],
    ) -> Result<u64> {
        let mut state = self.state();
        let (cutoff, _) = retained_utc_days(fetched_at, state.retention_days);
        let mut totals = BTreeMap::<(&str, &str, &str), f64>::new();
        for point in points.iter().filter(|point| point.day >= cutoff) {
            *totals.entry((&point.day, &point.metric, &point.series)).or_default() += point.value;
        }
        prune_if_needed(&mut state, fetched_at)?;
        let tx = write(&mut state)?;
        if let Some(fingerprint) = fingerprint {
            account::activate(&tx, provider, fingerprint)?;
        }
        for scope in scopes {
            let start = scope.start_day.as_str().max(&cutoff);
            if start > scope.end_day.as_str() {
                continue;
            }
            for metric in &scope.metrics {
                tx.execute(
                    "DELETE FROM analytics WHERE provider=?1 AND metric=?2 AND day>=?3 AND day<=?4",
                    params![provider, metric, start, scope.end_day],
                )?;
            }
        }
        {
            let mut insert = tx.prepare_cached(
                "INSERT OR REPLACE INTO analytics (provider,day,metric,series,value) VALUES (?1,?2,?3,?4,?5)",
            )?;
            for ((day, metric, series), value) in &totals {
                insert.execute(params![provider, day, metric, series, value])?;
            }
        }
        tx.commit()?;
        Ok(totals.len() as u64)
    }

    pub fn clear(&self) -> Result<u64> {
        let mut state = self.state();
        let tx = write(&mut state)?;
        let mut removed = 0;
        for table in ["samples", "analytics", "archived_samples", "archived_analytics"] {
            removed += tx.execute(&format!("DELETE FROM {table}"), [])? as u64;
        }
        tx.execute("DELETE FROM analytics_accounts", [])?;
        tx.commit()?;
        Ok(removed)
    }

    pub fn apply_retention_days(&self, days: u32, now: f64) -> Result<PruneResult> {
        let mut state = self.state();
        apply_retention_days(
            &mut state,
            quotaline_settings::normalize_history_retention_days(days),
            now,
        )
    }
}

fn prune_if_needed(state: &mut State, now: f64) -> Result<()> {
    if state.last_prune.is_some_and(|last| now - last < 3600.0) {
        return Ok(());
    }
    apply_retention_days(state, state.retention_days, now)?;
    Ok(())
}

fn apply_retention_days(state: &mut State, days: u32, now: f64) -> Result<PruneResult> {
    let cutoff = now - f64::from(days) * 86_400.0;
    let (analytics_cutoff, _) = retained_utc_days(now, days);
    let tx = write(state)?;
    let samples = tx.execute("DELETE FROM samples WHERE ts < ?1", [cutoff])?
        + tx.execute("DELETE FROM archived_samples WHERE ts < ?1", [cutoff])?;
    let analytics = tx.execute("DELETE FROM analytics WHERE day < ?1", [&analytics_cutoff])?
        + tx.execute("DELETE FROM archived_analytics WHERE day < ?1", [&analytics_cutoff])?;
    if samples + analytics > 0 {
        tx.execute_batch("PRAGMA incremental_vacuum(256)")?;
    }
    tx.commit()?;
    state.retention_days = days;
    state.last_prune = Some(now);
    Ok(PruneResult {
        samples: samples as u64,
        analytics: analytics as u64,
    })
}
