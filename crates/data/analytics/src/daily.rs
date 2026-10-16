//! Combines daily analytics readings for history summaries.

use std::collections::BTreeMap;

use super::Point;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct DailyBucket {
    pub day: String,
    pub series: String,
    pub value: f64,
}

pub fn daily_buckets(points: &[Point], metric: &str) -> Vec<DailyBucket> {
    let mut buckets = BTreeMap::<(&str, &str), f64>::new();
    for point in points.iter().filter(|point| point.metric == metric) {
        *buckets.entry((&point.day, &point.series)).or_default() += point.value;
    }
    buckets
        .into_iter()
        .map(|((day, series), value)| DailyBucket {
            day: day.into(),
            series: series.into(),
            value,
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/daily/tests.rs"]
mod tests;
