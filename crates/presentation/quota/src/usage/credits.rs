//! Selects ordered credit metrics and the earliest reset expiry.

use serde::Serialize;

use crate::{ResetCredits, UsageCredits};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
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

impl CreditMetricKind {
    fn supporting(self) -> bool {
        matches!(self, Self::LocalMessages | Self::CloudMessages | Self::ResetsEarned)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CreditMetric {
    pub kind: CreditMetricKind,
    pub supporting: bool,
    pub epoch: Option<i64>,
}

fn metric(kind: CreditMetricKind, epoch: Option<i64>) -> CreditMetric {
    CreditMetric {
        kind,
        supporting: kind.supporting(),
        epoch,
    }
}

pub fn credit_metrics(
    credits: Option<&UsageCredits>,
    resets: Option<&ResetCredits>,
    expiries: &[i64],
) -> Vec<CreditMetric> {
    use CreditMetricKind as Kind;

    let mut metrics = Vec::new();
    if let Some(credits) = credits {
        metrics.push(metric(Kind::Credits, None));
        if credits.overage_limit_reached {
            metrics.push(metric(Kind::Overage, None));
        }
        for (kind, range) in [
            (Kind::LocalMessages, credits.local_messages),
            (Kind::CloudMessages, credits.cloud_messages),
        ] {
            if range.is_some() {
                metrics.push(metric(kind, None));
            }
        }
    }
    if let Some(resets) = resets {
        metrics.push(metric(Kind::LimitResets, None));
        if let Some(expiry) = expiries.iter().min() {
            metrics.push(metric(Kind::NextExpiry, Some(*expiry)));
        }
        if resets.purchase_eligible {
            metrics.push(metric(Kind::ResetPurchase, None));
        }
        if resets.total_earned.is_some() {
            metrics.push(metric(Kind::ResetsEarned, None));
        }
    }
    metrics
}

#[cfg(test)]
#[path = "../../tests/unit/usage/credits/tests.rs"]
mod tests;
