//! Maps the history preferences stored in settings to the shared history session.

use quotaline_analytics::{HistorySession, SavedHistory, history_session_from_saved, sync_saved_history};

use crate::Settings;

impl Settings {
    fn saved_history(&self) -> SavedHistory {
        SavedHistory {
            days: self.history_days,
            rollup: self.history_rollup.clone(),
            metric: self.history_metric.clone(),
            utc: self.history_utc,
        }
    }
}

pub fn history_session_from_settings(settings: &Settings, time_zone: String, offset: i32, now: f64) -> HistorySession {
    history_session_from_saved(&settings.saved_history(), time_zone, offset, now)
}

/// Copies the session's persistent choices into settings; returns whether any changed.
pub fn sync_history_settings(session: &HistorySession, settings: &mut Settings) -> bool {
    let mut saved = settings.saved_history();
    if !sync_saved_history(session, &mut saved) {
        return false;
    }
    settings.history_days = saved.days;
    settings.history_rollup = saved.rollup;
    settings.history_metric = saved.metric;
    settings.history_utc = saved.utc;
    true
}

#[cfg(test)]
#[path = "../tests/unit/history/tests.rs"]
mod tests;
