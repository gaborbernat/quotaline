//! Exposes the shared window chart projection to Swift.

use quotaline_analytics::{WindowChart, WindowChartRequest, WindowSample, window_chart, window_nearest};

#[uniffi::export]
pub fn chart_window(samples: Vec<WindowSample>, request: WindowChartRequest) -> WindowChart {
    window_chart(&samples, &request)
}

#[uniffi::export]
pub fn chart_window_nearest(dates: Vec<f64>, at: f64) -> Option<f64> {
    window_nearest(&dates, at)
}

#[cfg(test)]
#[path = "../../tests/unit/history/window/tests.rs"]
mod tests;
