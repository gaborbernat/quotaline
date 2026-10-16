//! Exposes shared credit metric selection to Swift.

use quotaline_quota::ResetCredits;
use quotaline_quota::UsageCredits;
use quotaline_quota::usage;
use quotaline_quota::usage::CreditMetricKind as CoreKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CreditMetricKind {
    Credits,
    Overage,
    LocalMessages,
    CloudMessages,
    LimitResets,
    NextExpiry,
    ResetPurchase,
    ResetsEarned,
}

impl From<CoreKind> for CreditMetricKind {
    fn from(value: CoreKind) -> Self {
        match value {
            CoreKind::Credits => Self::Credits,
            CoreKind::Overage => Self::Overage,
            CoreKind::LocalMessages => Self::LocalMessages,
            CoreKind::CloudMessages => Self::CloudMessages,
            CoreKind::LimitResets => Self::LimitResets,
            CoreKind::NextExpiry => Self::NextExpiry,
            CoreKind::ResetPurchase => Self::ResetPurchase,
            CoreKind::ResetsEarned => Self::ResetsEarned,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CreditMetricRecord {
    pub kind: CreditMetricKind,
    pub supporting: bool,
    pub epoch: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CreditRangeInput {
    pub lower: i64,
    pub upper: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CreditUsageInput {
    pub unlimited: bool,
    pub overage_limit_reached: bool,
    pub local_messages: Option<CreditRangeInput>,
    pub cloud_messages: Option<CreditRangeInput>,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CreditResetInput {
    pub available: u64,
    pub applicable: u64,
    pub total_earned: Option<u64>,
    pub purchase_eligible: bool,
}

#[uniffi::export]
pub fn usage_credit_metrics(
    credits: Option<CreditUsageInput>,
    resets: Option<CreditResetInput>,
    expiries: Vec<i64>,
) -> Vec<CreditMetricRecord> {
    let credits = credits.map(|value| UsageCredits {
        unlimited: value.unlimited,
        has_credits: None,
        overage_limit_reached: value.overage_limit_reached,
        local_messages: value.local_messages.map(|range| (range.lower, range.upper)),
        cloud_messages: value.cloud_messages.map(|range| (range.lower, range.upper)),
    });
    let resets = resets.map(|value| ResetCredits {
        available: value.available,
        applicable: value.applicable,
        total_earned: value.total_earned,
        purchase_eligible: value.purchase_eligible,
        details: Vec::new(),
    });
    usage::credit_metrics(credits.as_ref(), resets.as_ref(), &expiries)
        .into_iter()
        .map(|metric| CreditMetricRecord {
            kind: metric.kind.into(),
            supporting: metric.supporting,
            epoch: metric.epoch,
        })
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/usage/credits/tests.rs"]
mod tests;
