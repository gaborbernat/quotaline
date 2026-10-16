//! Bridges shared spend-card decisions into Swift records.

use quotaline_quota::usage;
use quotaline_quota::usage::SpendAmount as CoreAmount;
use quotaline_quota::usage::SpendFacts;
use quotaline_quota::usage::SpendMetricKind as CoreKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum SpendAmount {
    Credits,
    Money,
    Unavailable,
    UnreportedLimit,
}

impl From<CoreAmount> for SpendAmount {
    fn from(value: CoreAmount) -> Self {
        match value {
            CoreAmount::Credits => Self::Credits,
            CoreAmount::Money => Self::Money,
            CoreAmount::Unavailable => Self::Unavailable,
            CoreAmount::UnreportedLimit => Self::UnreportedLimit,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum SpendMetricKind {
    Remaining,
    AllowanceSource,
    Balance,
    Resets,
    AutoReload,
    Purchase,
}

impl From<CoreKind> for SpendMetricKind {
    fn from(value: CoreKind) -> Self {
        match value {
            CoreKind::Remaining => Self::Remaining,
            CoreKind::AllowanceSource => Self::AllowanceSource,
            CoreKind::Balance => Self::Balance,
            CoreKind::Resets => Self::Resets,
            CoreKind::AutoReload => Self::AutoReload,
            CoreKind::Purchase => Self::Purchase,
        }
    }
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SpendFactsInput {
    pub used_credits: bool,
    pub limit_credits: bool,
    pub used_money: bool,
    pub limit_money: bool,
    pub percent: Option<f64>,
    pub remaining_credits: bool,
    pub allowance_source: bool,
    pub balance: bool,
    pub reset: bool,
    pub auto_reload: bool,
    pub purchase: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SpendMetricRecord {
    pub kind: SpendMetricKind,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SpendProjectionRecord {
    pub used: Option<SpendAmount>,
    pub limit: Option<SpendAmount>,
    pub percent: Option<f64>,
    pub metrics: Vec<SpendMetricRecord>,
}

#[uniffi::export]
pub fn usage_spend_projection(input: SpendFactsInput) -> SpendProjectionRecord {
    let projection = usage::spend_projection(SpendFacts {
        used_credits: input.used_credits,
        limit_credits: input.limit_credits,
        used_money: input.used_money,
        limit_money: input.limit_money,
        percent: input.percent,
        remaining_credits: input.remaining_credits,
        allowance_source: input.allowance_source,
        balance: input.balance,
        reset: input.reset,
        auto_reload: input.auto_reload,
        purchase: input.purchase,
    });
    SpendProjectionRecord {
        used: projection.used.map(Into::into),
        limit: projection.limit.map(Into::into),
        percent: projection.percent,
        metrics: projection
            .metrics
            .into_iter()
            .map(|metric| SpendMetricRecord {
                kind: metric.kind.into(),
            })
            .collect(),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/usage/spend/tests.rs"]
mod tests;
