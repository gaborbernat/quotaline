//! Exposes the shared analytics merge policy to Swift.

use quotaline_analytics::{AnalyticsSnapshot, merge_analytics};

#[uniffi::export]
pub fn merge_provider_analytics(
    previous: AnalyticsSnapshot,
    newer: AnalyticsSnapshot,
    retention_days: u32,
) -> AnalyticsSnapshot {
    merge_analytics(&previous, newer, retention_days)
}

#[cfg(test)]
#[path = "../../tests/unit/history/merge/tests.rs"]
mod tests;
