//! Holds macOS history navigation state behind the shared transition policy.

use quotaline_analytics::{HistoryNavigation, HistoryRange, HistoryViewport, NavigationState, WindowRollup};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(uniffi::Object)]
pub struct RustHistoryNavigation {
    state: Mutex<NavigationState>,
}

#[uniffi::export]
impl RustHistoryNavigation {
    #[uniffi::constructor]
    pub fn new(initial: HistoryNavigation) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(NavigationState::new(initial)),
        })
    }

    pub fn snapshot(&self) -> HistoryNavigation {
        self.state().snapshot()
    }

    pub fn sync_mode(&self, range: HistoryRange, daily_utc: bool, time_zone: String, fallback_offset: i32) {
        self.state().set_mode(range, daily_utc, time_zone, fallback_offset);
    }

    pub fn follow_now(&self) -> bool {
        self.state().follow_now()
    }

    pub fn set_follow_now(&self, value: bool) {
        self.state().set_follow_now(value);
    }

    pub fn custom_start(&self) -> f64 {
        self.state().custom_start()
    }

    pub fn set_custom_start(&self, value: f64) {
        self.state().set_custom_start(value);
    }

    pub fn custom_end(&self) -> f64 {
        self.state().custom_end()
    }

    pub fn set_custom_end(&self, value: f64) {
        self.state().set_custom_end(value);
    }

    pub fn anchor_end(&self) -> Option<f64> {
        self.state().anchor_end()
    }

    pub fn select_period(&self, range: Option<HistoryRange>, rollup: WindowRollup, now: f64) -> WindowRollup {
        self.state().select_period(range, rollup, now)
    }

    pub fn select_rollup(&self, rollup: WindowRollup) -> WindowRollup {
        self.state().select_rollup(rollup)
    }

    pub fn select_custom_start(&self, now: f64, requested: f64) {
        self.state().select_custom_start(now, requested);
    }

    pub fn select_custom_end(&self, now: f64, requested: f64) {
        self.state().select_custom_end(now, requested);
    }

    pub fn pause(&self, now: f64) {
        self.state().pause(now);
    }

    pub fn reset(&self, now: f64) {
        self.state().reset(now);
    }

    pub fn page(&self, now: f64, forward: bool) {
        self.state().page(now, forward);
    }

    pub fn viewport(&self, now: f64) -> HistoryViewport {
        self.state().viewport(now)
    }
}

impl RustHistoryNavigation {
    fn state(&self) -> MutexGuard<'_, NavigationState> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/history/navigation/state/tests.rs"]
mod tests;
