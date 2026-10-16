//! Exposes shared History visibility state to the macOS presenter.

use quotaline_analytics::HistoryVisibility;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(uniffi::Object)]
pub struct RustHistoryVisibility {
    state: Mutex<HistoryVisibility>,
}

#[uniffi::export]
impl RustHistoryVisibility {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(HistoryVisibility::default()),
        })
    }

    pub fn clear(&self) {
        self.state().clear();
    }

    pub fn set_hidden(&self, metric: String, ids: Vec<String>) {
        self.state().set_hidden(&metric, ids);
    }

    pub fn hidden(&self, metric: String) -> Vec<String> {
        self.state().hidden(&metric)
    }

    pub fn visible(&self, metric: String, id: String) -> bool {
        self.state().visible(&metric, &id)
    }

    pub fn toggle(&self, metric: String, id: String, series: Vec<String>) -> bool {
        self.state().toggle(&metric, &metric, &id, &series)
    }

    pub fn isolate(&self, metric: String, id: String, series: Vec<String>) -> bool {
        self.state().isolate(&metric, &metric, &id, &series, false)
    }
}

impl RustHistoryVisibility {
    fn state(&self) -> MutexGuard<'_, HistoryVisibility> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/history/visibility/tests.rs"]
mod tests;
