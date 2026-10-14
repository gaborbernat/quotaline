//! Exposes shared timestamp conversion and time wording to Swift; Swift formats dates in the user's locale.

use std::sync::Arc;

use quotaline_quota::ResetPrecision;
use quotaline_text::time;

mod clock_callback;
pub use clock_callback::ClockFormatter;

pub(crate) struct Clock(pub(crate) Arc<dyn ClockFormatter>);

impl time::ClockFormat for Clock {
    fn utc_day(&self, at: i64) -> String {
        self.0.utc_day(at)
    }

    fn time(&self, at: i64) -> String {
        self.0.time(at)
    }

    fn weekday_time(&self, at: i64) -> String {
        self.0.weekday_time(at)
    }

    fn date_time(&self, at: i64) -> String {
        self.0.date_time(at)
    }

    fn same_local_day(&self, at: i64, now: i64) -> bool {
        self.0.same_local_day(at, now)
    }
}

#[uniffi::export]
pub fn time_countdown(seconds: f64) -> String {
    time::countdown(seconds)
}

#[uniffi::export]
pub fn time_compact_countdown(seconds: f64) -> String {
    time::compact_countdown(seconds)
}

#[uniffi::export]
pub fn time_relative_age(seconds: f64) -> String {
    time::relative_age(seconds)
}

#[uniffi::export]
pub fn time_reset_clock(at: i64, day_precision: bool, now: i64, clock: Arc<dyn ClockFormatter>) -> String {
    let precision = if day_precision {
        ResetPrecision::Day
    } else {
        ResetPrecision::Instant
    };
    time::reset_clock(at, precision, now, &Clock(clock))
}

#[uniffi::export]
pub fn parse_iso_epoch(value: String) -> Option<f64> {
    quotaline_text::time::parse_iso_epoch(&value)
}

#[uniffi::export]
pub fn format_iso_epoch(epoch: f64) -> Option<String> {
    quotaline_text::time::format_iso_epoch(epoch)
}

#[cfg(test)]
#[path = "../tests/unit/time/tests.rs"]
mod tests;
