//! Reads cost history and projects shared spend summaries.

use std::collections::BTreeMap;

use chrono::Utc;
use quotaline_analytics as analytics;
use quotaline_history::Store;

use super::analytics::retained_range;
use super::{History, history_analytics, history_points};

pub fn spend_report(
    store: &Store,
    providers: &BTreeMap<String, bool>,
    now: chrono::DateTime<Utc>,
) -> quotaline_history::Result<analytics::SpendReport> {
    let (start, end) = retained_range(now, 30);
    let history = history_analytics(store, "costUSD", start, end, providers)?;
    Ok(report_from_history(&history, now))
}

fn report_from_history(history: &History, now: chrono::DateTime<Utc>) -> analytics::SpendReport {
    let rows = history_points(history)
        .into_iter()
        .map(|point| analytics::SpendRow {
            day: point.day,
            provider: point.provider,
            model: point.series,
            value: point.value,
        })
        .collect::<Vec<_>>();
    analytics::spend_report(&rows, now)
}

#[cfg(test)]
#[path = "../../tests/unit/history/spend/tests.rs"]
mod tests;
