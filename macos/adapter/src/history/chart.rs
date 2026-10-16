//! Exposes shared analytics series to the macOS chart boundary.

use quotaline_analytics as analytics;
use quotaline_analytics::{AnalyticsChart, DailyBucket, Point};

#[uniffi::export]
pub fn chart_daily_buckets(points: Vec<Point>, metric: String) -> Vec<DailyBucket> {
    analytics::daily_buckets(&points, &metric)
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ChartSeriesKey {
    pub provider: String,
    pub series: String,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct StoredAnalyticsChartQuery {
    pub metric: String,
    pub providers: Vec<String>,
    pub start_day: String,
    pub before_day: String,
    pub hidden: Vec<String>,
    pub stacked: bool,
}

#[uniffi::export]
pub fn chart_analytics(
    rows: Vec<Point>,
    metric: String,
    hidden: Vec<ChartSeriesKey>,
    stacked: bool,
) -> Option<AnalyticsChart> {
    let hidden = hidden
        .into_iter()
        .map(|key| (key.provider, key.series))
        .collect::<Vec<_>>();
    analytics::analytics_chart(&rows, &metric, &hidden, stacked)
}

#[cfg(test)]
#[path = "../../tests/unit/history/chart/tests.rs"]
mod tests;
