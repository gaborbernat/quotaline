//! Merges provider analytics while respecting account, coverage, and retention boundaries.

use std::collections::BTreeMap;

use super::{CreditEvent, Point};
use crate::utc_day::{midnight, retained_utc_days};

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct CoverageScope {
    pub metrics: Vec<String>,
    pub start_day: String,
    pub end_day: String,
}

impl CoverageScope {
    fn contains(&self, point: &Point) -> bool {
        self.metrics.contains(&point.metric) && point.day >= self.start_day && point.day <= self.end_day
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AnalyticsSnapshot {
    pub provider: String,
    pub points: Vec<Point>,
    pub credit_events: Vec<CreditEvent>,
    pub fetched_at: f64,
    pub account_fingerprint: Option<String>,
    pub covered_scopes: Vec<CoverageScope>,
}

pub fn merge_analytics(
    previous: &AnalyticsSnapshot,
    newer: AnalyticsSnapshot,
    retention_days: u32,
) -> AnalyticsSnapshot {
    let (cutoff, end) = retained_utc_days(newer.fetched_at, retention_days);
    let cutoff_at = midnight(&cutoff).expect("generated day is valid");
    let end_at = midnight(&end).expect("generated day is valid") + 86_400.0;
    let same_account = previous.provider == newer.provider && previous.account_fingerprint == newer.account_fingerprint;
    let AnalyticsSnapshot {
        provider,
        points,
        credit_events,
        fetched_at,
        account_fingerprint,
        covered_scopes,
    } = newer;

    let mut merged_points = BTreeMap::new();
    for point in previous.points.iter().filter(|point| {
        same_account
            && point.day >= cutoff
            && point.day <= end
            && !covered_scopes.iter().any(|scope| scope.contains(point))
    }) {
        merged_points.insert(
            (point.day.clone(), point.metric.clone(), point.series.clone()),
            point.clone(),
        );
    }
    for point in points
        .into_iter()
        .filter(|point| point.day >= cutoff && point.day <= end)
    {
        merged_points.insert((point.day.clone(), point.metric.clone(), point.series.clone()), point);
    }

    let mut merged_events = BTreeMap::new();
    for event in previous
        .credit_events
        .iter()
        .filter(|event| same_account && event.timestamp >= cutoff_at && event.timestamp < end_at)
    {
        merged_events.insert(event.id.clone(), event.clone());
    }
    for event in credit_events
        .into_iter()
        .filter(|event| event.timestamp >= cutoff_at && event.timestamp < end_at)
    {
        merged_events.insert(event.id.clone(), event);
    }
    let mut credit_events = merged_events.into_values().collect::<Vec<_>>();
    credit_events.sort_by(|left, right| {
        left.timestamp
            .total_cmp(&right.timestamp)
            .then_with(|| left.id.cmp(&right.id))
    });

    AnalyticsSnapshot {
        provider,
        points: merged_points.into_values().collect(),
        credit_events,
        fetched_at,
        account_fingerprint,
        covered_scopes,
    }
}

#[cfg(test)]
#[path = "../tests/unit/merge/tests.rs"]
mod tests;
