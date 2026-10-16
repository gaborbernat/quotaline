//! Tracks quota resets and consecutive windows sharing a reset.

use serde::Serialize;

use crate::ResetPrecision;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResetGroupInput {
    pub reset_at: Option<f64>,
    pub precision: ResetPrecision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ResetGroupEdge {
    pub with_previous: bool,
    pub with_next: bool,
}

pub fn reset_group_edges(windows: &[ResetGroupInput]) -> Vec<ResetGroupEdge> {
    windows
        .iter()
        .enumerate()
        .map(|(index, window)| ResetGroupEdge {
            with_previous: index > 0 && same_reset(&windows[index - 1], window),
            with_next: windows.get(index + 1).is_some_and(|next| same_reset(window, next)),
        })
        .collect()
}

fn same_reset(left: &ResetGroupInput, right: &ResetGroupInput) -> bool {
    left.reset_at.is_some() && left.reset_at == right.reset_at && left.precision == right.precision
}

pub fn has_reset(
    previous_reset: Option<f64>,
    current_reset: Option<f64>,
    previous_used: f64,
    current_used: f64,
) -> bool {
    match previous_reset.zip(current_reset) {
        Some((previous, current)) => current > previous,
        None => current_used < previous_used - 1.0,
    }
}

#[cfg(test)]
#[path = "../tests/unit/reset_groups/tests.rs"]
mod tests;
