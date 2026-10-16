//! Bridges shared history aggregation into the macOS chart model.

use quotaline_analytics::CHANGE_THRESHOLD;
use quotaline_analytics::SAMPLE_INTERVAL;
use quotaline_analytics::UsageSample;
use quotaline_analytics::should_record_usage;

mod store;
pub use store::*;
mod merge;
pub use merge::*;
mod chart;
pub use chart::*;
mod window;
pub use window::*;
mod navigation;
pub use navigation::*;
mod visibility;
pub use visibility::*;
mod scope;
pub use scope::*;

#[derive(Clone, Copy, Debug, PartialEq, uniffi::Record)]
pub struct HistorySamplePoint {
    pub at: f64,
    pub used: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, uniffi::Record)]
pub struct HistorySampleLimits {
    pub interval: f64,
    pub change_threshold: f64,
}

#[uniffi::export]
pub fn history_sample_limits() -> HistorySampleLimits {
    HistorySampleLimits {
        interval: SAMPLE_INTERVAL,
        change_threshold: CHANGE_THRESHOLD,
    }
}

#[uniffi::export]
pub fn history_should_record(
    current: HistorySamplePoint,
    previous: Option<HistorySamplePoint>,
    reset_changed: bool,
) -> bool {
    should_record_usage(
        UsageSample {
            at: current.at,
            used: current.used,
        },
        previous.map(|last| UsageSample {
            at: last.at,
            used: last.used,
        }),
        reset_changed,
    )
}

#[cfg(test)]
#[path = "../tests/unit/history/tests.rs"]
mod tests;
