//! Resolves aggregate history rows shared by platform charts.

use std::collections::{BTreeMap, BTreeSet};

pub const SAMPLE_INTERVAL: f64 = 300.0;
pub const CHANGE_THRESHOLD: f64 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UsageSample {
    pub at: f64,
    pub used: f64,
}

pub fn should_record_usage(current: UsageSample, previous: Option<UsageSample>, reset_changed: bool) -> bool {
    previous.is_none_or(|last| {
        current.at - last.at >= SAMPLE_INTERVAL || reset_changed || (current.used - last.used).abs() >= CHANGE_THRESHOLD
    })
}

#[derive(Clone, Debug)]
pub struct HistoryReading {
    pub provider: String,
    pub bucket: String,
    pub series: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistoryProjection {
    pub canonical_total: f64,
    pub detailed_buckets: BTreeSet<(String, String)>,
}

pub fn history_projection(readings: &[HistoryReading]) -> HistoryProjection {
    let mut buckets: BTreeMap<_, (f64, f64, f64, bool, bool)> = BTreeMap::new();
    let mut detailed = BTreeSet::new();
    for reading in readings {
        let key = (reading.provider.clone(), reading.bucket.clone());
        let sums = buckets.entry(key.clone()).or_default();
        if reading.series == "total" {
            sums.0 += reading.value;
            sums.3 = true;
        } else if reading.series.starts_with("surface:") {
            sums.1 += reading.value;
            sums.4 = true;
            detailed.insert(key);
        } else {
            sums.2 += reading.value;
            detailed.insert(key);
        }
    }
    HistoryProjection {
        canonical_total: buckets
            .values()
            .map(|(total, surfaces, other, has_total, has_surfaces)| {
                if *has_total {
                    *total
                } else if *has_surfaces {
                    *surfaces
                } else {
                    *other
                }
            })
            .sum(),
        detailed_buckets: detailed,
    }
}

#[cfg(test)]
#[path = "../tests/unit/history/tests.rs"]
mod tests;
