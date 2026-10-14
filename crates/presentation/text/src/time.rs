//! Converts the ISO timestamps used by provider credentials and history, and words times for display.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

mod text;
pub use text::{ClockFormat, compact_countdown, countdown, relative_age, remaining, reset_clock};

/// Whether a reset time is exact or only known to the day.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResetPrecision {
    #[default]
    Instant,
    Day,
}

pub fn parse_iso_epoch(value: &str) -> Option<f64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.timestamp_millis() as f64 / 1000.0)
}

pub fn format_iso_epoch(epoch: f64) -> Option<String> {
    if !epoch.is_finite() {
        return None;
    }
    DateTime::<Utc>::from_timestamp_millis((epoch * 1000.0).round() as i64)
        .map(|value| value.to_rfc3339_opts(SecondsFormat::Millis, true))
}

#[cfg(test)]
#[path = "../tests/unit/time/tests.rs"]
mod tests;
