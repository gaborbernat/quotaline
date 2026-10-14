//! Orders local usage metrics and chooses money precision.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalMetricKind {
    WindowTokens,
    WindowCost,
    CostPerHour,
    TodayTokens,
    TodayMessages,
    TodayCost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LocalMetric {
    pub kind: LocalMetricKind,
    pub money_fraction_digits: Option<u8>,
}

pub fn local_money_fraction_digits(value: f64) -> u8 {
    if value < 10.0 { 2 } else { 0 }
}

pub fn local_metrics(window_cost: f64, cost_per_hour: f64, today_cost: f64) -> Vec<LocalMetric> {
    use LocalMetricKind as Kind;
    [
        (Kind::WindowTokens, None),
        (Kind::WindowCost, Some(window_cost)),
        (Kind::CostPerHour, Some(cost_per_hour)),
        (Kind::TodayTokens, None),
        (Kind::TodayMessages, None),
        (Kind::TodayCost, Some(today_cost)),
    ]
    .into_iter()
    .map(|(kind, cost)| LocalMetric {
        kind,
        money_fraction_digits: cost.map(local_money_fraction_digits),
    })
    .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/usage/local/tests.rs"]
mod tests;
