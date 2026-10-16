//! Converts between Unix timestamps and the `YYYY-MM-DD` UTC days that key daily analytics.

use chrono::{DateTime, Days, NaiveDate, NaiveTime};

fn date(at: f64) -> NaiveDate {
    DateTime::from_timestamp(at.floor() as i64, 0)
        .unwrap_or_default()
        .date_naive()
}

pub fn day(at: f64) -> String {
    date(at).to_string()
}

pub fn midnight(day: &str) -> Option<f64> {
    NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .ok()
        .map(|date| date.and_time(NaiveTime::MIN).and_utc().timestamp() as f64)
}

pub fn retained_utc_days(now: f64, retention_days: u32) -> (String, String) {
    let end = date(now);
    let start = end - Days::new(u64::from(retention_days.saturating_sub(1)));
    (start.to_string(), end.to_string())
}

#[cfg(test)]
#[path = "../tests/unit/utc_day/tests.rs"]
mod tests;
