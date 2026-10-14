//! Formats calendar values as labelled numbers so tests can see which detail the wording chose.

use crate::time::ClockFormat;

pub struct Clock;

impl ClockFormat for Clock {
    fn utc_day(&self, at: i64) -> String {
        format!("day {at}")
    }

    fn time(&self, at: i64) -> String {
        format!("time {at}")
    }

    fn weekday_time(&self, at: i64) -> String {
        format!("weekday {at}")
    }

    fn date_time(&self, at: i64) -> String {
        format!("date {at}")
    }

    fn same_local_day(&self, at: i64, now: i64) -> bool {
        at - now < 6 * 3600
    }
}
