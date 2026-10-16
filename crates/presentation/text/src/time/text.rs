//! Words durations and reset times the same way on every desktop; each desktop formats dates in its own locale.

use super::ResetPrecision;

pub fn countdown(seconds: f64) -> String {
    if seconds <= 0.0 {
        return "reset due".into();
    }
    let minutes = (seconds / 60.0) as i64;
    let (days, hours) = (minutes / 1440, minutes % 1440 / 60);
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours} hr {} min", minutes % 60)
    } else if minutes > 0 {
        format!("{minutes} min")
    } else {
        "< 1 min".into()
    }
}

pub fn compact_countdown(seconds: f64) -> String {
    if seconds <= 0.0 {
        return "0m".into();
    }
    let minutes = (seconds / 60.0) as i64;
    let (days, hours) = (minutes / 1440, minutes % 1440 / 60);
    if days > 0 {
        format!("{days}d{hours}h")
    } else if hours > 0 {
        format!("{hours}h{:02}m", minutes % 60)
    } else {
        format!("{minutes}m")
    }
}

pub fn remaining(seconds: i64) -> String {
    let seconds = seconds.max(0);
    if seconds >= 86_400 {
        format!("{} d", seconds / 86_400)
    } else if seconds >= 3600 {
        format!("{} h", seconds / 3600)
    } else {
        format!("{} min", (seconds / 60).max(1))
    }
}

pub fn relative_age(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    if seconds < 5.0 {
        "just now".into()
    } else if seconds < 60.0 {
        format!("{}s ago", seconds as i64)
    } else if seconds < 3600.0 {
        format!("{} min ago", (seconds / 60.0) as i64)
    } else if seconds < 86_400.0 {
        format!("{} hr ago", (seconds / 3600.0) as i64)
    } else {
        format!("{} d ago", (seconds / 86_400.0) as i64)
    }
}

/// Formats calendar values in the desktop's locale and time zone.
pub trait ClockFormat {
    /// The abbreviated month and day of `at` read in UTC.
    fn utc_day(&self, at: i64) -> String;
    /// The local time of day.
    fn time(&self, at: i64) -> String;
    /// The abbreviated local weekday and time of day.
    fn weekday_time(&self, at: i64) -> String;
    /// The abbreviated local month, day, and time of day.
    fn date_time(&self, at: i64) -> String;
    fn same_local_day(&self, at: i64, now: i64) -> bool;
}

pub fn reset_clock(at: i64, precision: ResetPrecision, now: i64, clock: &dyn ClockFormat) -> String {
    if at <= now {
        return "now".into();
    }
    // A day-precision reset is stored as midnight UTC, so only a UTC read-back names the day the provider published.
    if precision == ResetPrecision::Day {
        return clock.utc_day(at);
    }
    if clock.same_local_day(at, now) {
        clock.time(at)
    } else if at - now < 6 * 86_400 {
        clock.weekday_time(at)
    } else {
        clock.date_time(at)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/time/text/tests.rs"]
mod tests;
