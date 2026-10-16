//! Bridges shared history navigation to the macOS presenter.

use quotaline_analytics as analytics;

mod state;
pub use state::RustHistoryNavigation;

#[uniffi::export]
pub fn history_step_reading(count: u32, selected: Option<u32>, direction: i32) -> Option<u32> {
    analytics::step_reading(count as usize, selected.map(|index| index as usize), &[direction])
        .map(|index| index as u32)
}
