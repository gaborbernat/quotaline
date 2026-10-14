//! Exposes shared quota reset decisions to Swift.

use quotaline_quota::ResetPrecision;
use quotaline_quota::reset_groups as quota;

#[derive(Clone, Copy, Debug, PartialEq, uniffi::Record)]
pub struct ResetGroupInput {
    pub reset_at: Option<f64>,
    pub day_precision: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ResetGroupEdge {
    pub with_previous: bool,
    pub with_next: bool,
}

#[uniffi::export]
pub fn reset_group_edges(windows: Vec<ResetGroupInput>) -> Vec<ResetGroupEdge> {
    quota::reset_group_edges(
        &windows
            .into_iter()
            .map(|window| quota::ResetGroupInput {
                reset_at: window.reset_at,
                precision: if window.day_precision {
                    ResetPrecision::Day
                } else {
                    ResetPrecision::Instant
                },
            })
            .collect::<Vec<_>>(),
    )
    .into_iter()
    .map(|edge| ResetGroupEdge {
        with_previous: edge.with_previous,
        with_next: edge.with_next,
    })
    .collect()
}

#[uniffi::export]
pub fn quota_has_reset(
    previous_reset: Option<f64>,
    current_reset: Option<f64>,
    previous_used: f64,
    current_used: f64,
) -> bool {
    quota::has_reset(previous_reset, current_reset, previous_used, current_used)
}

#[cfg(test)]
#[path = "../tests/unit/quota/tests.rs"]
mod tests;
