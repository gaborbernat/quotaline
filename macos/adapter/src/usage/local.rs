//! Exposes shared local usage rows and precision to Swift.

use quotaline_quota::usage;
use quotaline_quota::usage::LocalMetricKind as CoreKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum LocalMetricKind {
    WindowTokens,
    WindowCost,
    CostPerHour,
    TodayTokens,
    TodayMessages,
    TodayCost,
}

impl From<CoreKind> for LocalMetricKind {
    fn from(value: CoreKind) -> Self {
        match value {
            CoreKind::WindowTokens => Self::WindowTokens,
            CoreKind::WindowCost => Self::WindowCost,
            CoreKind::CostPerHour => Self::CostPerHour,
            CoreKind::TodayTokens => Self::TodayTokens,
            CoreKind::TodayMessages => Self::TodayMessages,
            CoreKind::TodayCost => Self::TodayCost,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct LocalMetricRecord {
    pub kind: LocalMetricKind,
    pub money_fraction_digits: Option<u8>,
}

#[uniffi::export]
pub fn usage_local_metrics(window_cost: f64, cost_per_hour: f64, today_cost: f64) -> Vec<LocalMetricRecord> {
    usage::local_metrics(window_cost, cost_per_hour, today_cost)
        .into_iter()
        .map(|metric| LocalMetricRecord {
            kind: metric.kind.into(),
            money_fraction_digits: metric.money_fraction_digits,
        })
        .collect()
}

#[uniffi::export]
pub fn usage_local_money_fraction_digits(value: f64) -> u8 {
    usage::local_money_fraction_digits(value)
}

#[cfg(test)]
#[path = "../../tests/unit/usage/local/tests.rs"]
mod tests;
