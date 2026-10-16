//! Formats notification times in the session's time zone and clock preference.

use chrono::{DateTime, TimeZone, Utc};
use quotaline_text::time::ClockFormat;

pub struct SessionClock<Zone> {
    zone: Zone,
    time: &'static str,
}

impl<Zone: TimeZone> SessionClock<Zone>
where
    Zone::Offset: std::fmt::Display,
{
    pub fn new(zone: Zone, twelve_hour: bool) -> Self {
        Self {
            zone,
            time: if twelve_hour { "%-I:%M %p" } else { "%H:%M" },
        }
    }

    fn local(&self, at: i64, pattern: &str) -> String {
        DateTime::<Utc>::from_timestamp(at, 0)
            .unwrap_or_default()
            .with_timezone(&self.zone)
            .format(pattern)
            .to_string()
    }
}

impl<Zone: TimeZone> ClockFormat for SessionClock<Zone>
where
    Zone::Offset: std::fmt::Display,
{
    fn utc_day(&self, at: i64) -> String {
        DateTime::<Utc>::from_timestamp(at, 0)
            .unwrap_or_default()
            .format("%b %-d")
            .to_string()
    }

    fn time(&self, at: i64) -> String {
        self.local(at, self.time)
    }

    fn weekday_time(&self, at: i64) -> String {
        self.local(at, &format!("%a {}", self.time))
    }

    fn date_time(&self, at: i64) -> String {
        self.local(at, &format!("%b %-d {}", self.time))
    }

    fn same_local_day(&self, at: i64, now: i64) -> bool {
        self.local(at, "%F") == self.local(now, "%F")
    }
}

#[cfg(test)]
#[path = "../tests/unit/clock/tests.rs"]
mod tests;
