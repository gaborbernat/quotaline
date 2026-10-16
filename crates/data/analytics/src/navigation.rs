//! Calculates history chart windows and page boundaries.

use chrono::{DateTime, Days, FixedOffset, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

use super::WindowRollup;

mod session;
pub use session::{HistorySession, HistorySessionConfig, HistorySessionView};
mod state;
pub use state::NavigationState;
mod settings;
pub use settings::{
    HistoryAction, SavedHistory, from_saved as history_session_from_saved, sync_saved as sync_saved_history,
};

pub const HISTORY_MAX_QUERY_POINTS: usize = 800;

// The macOS app already has a Swift type with this name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[cfg_attr(feature = "uniffi", uniffi(name = "HistoryNavigationRange"))]
pub enum HistoryRange {
    Today = 1,
    Week = 7,
    Month = 30,
    TwoMonths = 60,
    Custom = 0,
}

impl HistoryRange {
    pub fn days(self) -> i64 {
        self as i64
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct HistoryNavigation {
    pub range: HistoryRange,
    pub follow_now: bool,
    pub anchor_end: Option<f64>,
    pub custom_start: f64,
    pub custom_end: f64,
    pub return_to_live_at: Option<f64>,
    pub daily_utc: bool,
    pub time_zone: String,
    pub fallback_offset: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistorySelection {
    pub navigation: HistoryNavigation,
    pub rollup: WindowRollup,
}

pub fn history_select_period(
    state: &HistoryNavigation,
    range: Option<HistoryRange>,
    rollup: WindowRollup,
    now: f64,
) -> HistorySelection {
    let mut navigation = state.clone();
    let mut rollup = rollup;
    match range {
        None => {
            navigation.follow_now = true;
            navigation.anchor_end = None;
            navigation.return_to_live_at = None;
            if navigation.range == HistoryRange::Custom {
                let duration = (navigation.custom_end - navigation.custom_start).max(60.0);
                navigation.custom_end = now;
                navigation.custom_start = now - duration;
            }
        }
        Some(range) => {
            navigation.range = range;
            navigation.follow_now = false;
            navigation.anchor_end = Some(now);
            navigation.return_to_live_at = Some(now);
            if range == HistoryRange::Today && rollup == WindowRollup::Day {
                rollup = WindowRollup::Hour;
            }
            if range == HistoryRange::Custom {
                navigation.custom_end = now;
                navigation.custom_start = navigation.custom_start.min(now - 60.0);
            }
        }
    }
    HistorySelection { navigation, rollup }
}

pub fn history_select_rollup(state: &HistoryNavigation, rollup: WindowRollup) -> HistorySelection {
    let mut navigation = state.clone();
    if rollup == WindowRollup::Day && navigation.range == HistoryRange::Today {
        navigation.range = HistoryRange::Week;
    }
    HistorySelection { navigation, rollup }
}

pub fn history_select_custom_start(state: &HistoryNavigation, now: f64, requested: f64) -> HistoryNavigation {
    let mut navigation = state.clone();
    let end = history_viewport(state, now).end;
    navigation.range = HistoryRange::Custom;
    navigation.custom_end = end;
    navigation.custom_start = requested.min(end - 60.0);
    navigation.follow_now = false;
    navigation.anchor_end = Some(end);
    navigation.return_to_live_at = None;
    navigation
}

pub fn history_select_custom_end(state: &HistoryNavigation, now: f64, requested: f64) -> HistoryNavigation {
    let mut navigation = state.clone();
    let start = history_viewport(state, now).start;
    navigation.range = HistoryRange::Custom;
    navigation.custom_start = start;
    navigation.custom_end = requested.max(start + 60.0);
    navigation.follow_now = false;
    navigation.anchor_end = Some(navigation.custom_end);
    navigation.return_to_live_at = None;
    navigation
}

pub fn history_pause(state: &HistoryNavigation, now: f64) -> HistoryNavigation {
    HistoryNavigation {
        follow_now: false,
        anchor_end: Some(now),
        return_to_live_at: None,
        ..state.clone()
    }
}

pub fn history_reset(state: &HistoryNavigation, now: f64) -> HistoryNavigation {
    HistoryNavigation {
        follow_now: true,
        anchor_end: None,
        return_to_live_at: None,
        custom_start: now - 86_400.0,
        custom_end: now,
        ..state.clone()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct HistoryViewport {
    pub start: f64,
    pub end: f64,
}

pub fn history_viewport(state: &HistoryNavigation, now: f64) -> HistoryViewport {
    let zone = Zone::new(state);
    if state.range == HistoryRange::Custom {
        let end = if state.follow_now {
            now
        } else {
            state.custom_end.min(now)
        };
        return if state.daily_utc {
            let end = zone.start_of_day(end);
            HistoryViewport {
                start: zone.start_of_day(state.custom_start).min(end),
                end,
            }
        } else {
            HistoryViewport {
                start: state.custom_start.min(end - 60.0),
                end,
            }
        };
    }
    let end = if state.follow_now {
        now
    } else {
        state.anchor_end.unwrap_or(now).min(now)
    };
    let days = state.range.days();
    if state.daily_utc {
        let end = zone.start_of_day(end);
        return HistoryViewport {
            start: end - (days - 1) as f64 * 86_400.0,
            end,
        };
    }
    HistoryViewport {
        start: if state.range == HistoryRange::Today && state.follow_now {
            zone.start_of_day(end)
        } else {
            zone.add_days(end, -days)
        },
        end,
    }
}

pub fn history_page(state: &HistoryNavigation, now: f64, forward: bool) -> HistoryNavigation {
    let view = history_viewport(state, now);
    let zone = Zone::new(state);
    let mut next = state.clone();
    if state.range == HistoryRange::Custom {
        if state.daily_utc {
            let today = zone.start_of_day(now);
            let day_count = ((view.end - view.start) / 86_400.0) as i64 + 1;
            let proposed_end = if forward {
                view.end + day_count as f64 * 86_400.0
            } else {
                view.start - 86_400.0
            };
            next.custom_end = proposed_end.min(today);
            next.custom_start = next.custom_end - (day_count - 1) as f64 * 86_400.0;
            next.follow_now = next.custom_end >= today;
            next.return_to_live_at = None;
        } else {
            let duration = (view.end - view.start).max(60.0);
            let live_edge = if state.follow_now || view.end >= now {
                Some(view.end)
            } else {
                state.return_to_live_at
            };
            next.custom_end = (if forward { view.end + duration } else { view.start }).min(now);
            next.follow_now =
                next.custom_end >= now || forward && live_edge.is_some_and(|edge| next.custom_end >= edge);
            if next.follow_now {
                next.custom_end = now;
            }
            next.custom_start = next.custom_end - duration;
            next.return_to_live_at = if next.follow_now { None } else { live_edge };
        }
        next.anchor_end = (!next.follow_now).then_some(next.custom_end);
        return next;
    }
    let live_edge = if state.follow_now {
        Some(view.end)
    } else {
        state.return_to_live_at
    };
    let next_end = if state.daily_utc {
        let today = zone.start_of_day(now);
        (if forward {
            view.end + state.range.days() as f64 * 86_400.0
        } else {
            view.start - 86_400.0
        })
        .min(today)
    } else if state.range == HistoryRange::Today && !forward && view.end > zone.start_of_day(view.end) {
        zone.start_of_day(view.end)
    } else {
        (if forward {
            zone.add_days(view.end, state.range.days())
        } else {
            view.start
        })
        .min(now)
    };
    next.follow_now = next_end >= if state.daily_utc { zone.start_of_day(now) } else { now }
        || forward && live_edge.is_some_and(|edge| next_end >= edge);
    next.anchor_end = (!next.follow_now).then_some(next_end);
    next.return_to_live_at = if next.follow_now { None } else { live_edge };
    next
}

pub fn history_query_rollup(start: f64, end: f64, selected: f64, sample_interval: f64, limit: usize) -> f64 {
    let duration = (end - start).max(0.0);
    let budget = (duration / limit as f64 / sample_interval).ceil() * sample_interval;
    selected.max(sample_interval).max(budget)
}

enum Zone {
    Named(Tz),
    Offset(FixedOffset),
}

impl Zone {
    fn new(state: &HistoryNavigation) -> Self {
        if state.daily_utc {
            return Self::Named(chrono_tz::UTC);
        }
        state.time_zone.parse().map(Self::Named).unwrap_or_else(|_| {
            Self::Offset(FixedOffset::east_opt(state.fallback_offset).expect("time zone offset is valid"))
        })
    }

    fn local(&self, timestamp: f64) -> NaiveDateTime {
        let seconds = timestamp.floor() as i64;
        let nanos = ((timestamp - seconds as f64) * 1_000_000_000.0).round() as u32;
        let utc = DateTime::<Utc>::from_timestamp(seconds, nanos).expect("history timestamp is valid");
        match self {
            Self::Named(zone) => utc.with_timezone(zone).naive_local(),
            Self::Offset(zone) => utc.with_timezone(zone).naive_local(),
        }
    }

    fn timestamp(&self, local: NaiveDateTime) -> f64 {
        let utc = match self {
            Self::Named(zone) => zone
                .from_local_datetime(&local)
                .earliest()
                .or_else(|| {
                    zone.from_local_datetime(&(local + chrono::Duration::hours(1)))
                        .earliest()
                })
                .expect("calendar date resolves in time zone")
                .with_timezone(&Utc),
            Self::Offset(zone) => zone.from_local_datetime(&local).single().unwrap().with_timezone(&Utc),
        };
        utc.timestamp() as f64 + f64::from(utc.timestamp_subsec_nanos()) / 1_000_000_000.0
    }

    fn start_of_day(&self, timestamp: f64) -> f64 {
        self.timestamp(self.local(timestamp).date().and_hms_opt(0, 0, 0).unwrap())
    }

    fn start_of_date(&self, date: chrono::NaiveDate) -> f64 {
        self.timestamp(date.and_hms_opt(0, 0, 0).unwrap())
    }

    fn add_days(&self, timestamp: f64, days: i64) -> f64 {
        let local = self.local(timestamp);
        let date = if days < 0 {
            local.date().checked_sub_days(Days::new(days.unsigned_abs()))
        } else {
            local.date().checked_add_days(Days::new(days as u64))
        }
        .expect("history date is valid");
        self.timestamp(date.and_time(local.time()))
    }
}

#[cfg(test)]
#[path = "../tests/unit/navigation/tests.rs"]
mod tests;
