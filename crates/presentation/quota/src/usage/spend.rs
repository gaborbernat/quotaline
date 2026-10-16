//! Selects spend summary parts and ordered detail metrics for both frontends.

use serde::Serialize;

use crate::SpendControl;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SpendFacts {
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

impl SpendFacts {
    pub fn from_spend(spend: &SpendControl) -> Self {
        Self {
            used_credits: spend
                .credit_allowance
                .as_ref()
                .is_some_and(|allowance| allowance.used.is_some()),
            limit_credits: spend
                .credit_allowance
                .as_ref()
                .is_some_and(|allowance| allowance.limit.is_some()),
            used_money: spend.used.is_some(),
            limit_money: spend.limit.is_some(),
            percent: spend.percent,
            remaining_credits: spend
                .credit_allowance
                .as_ref()
                .is_some_and(|allowance| allowance.remaining.is_some()),
            allowance_source: spend
                .credit_allowance
                .as_ref()
                .is_some_and(|allowance| allowance.source.is_some()),
            balance: spend.balance.is_some(),
            reset: spend.reset.is_some(),
            auto_reload: spend.auto_reload.is_some(),
            purchase: spend.can_purchase_credits,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpendAmount {
    Credits,
    Money,
    Unavailable,
    UnreportedLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpendMetricKind {
    Remaining,
    AllowanceSource,
    Balance,
    Resets,
    AutoReload,
    Purchase,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SpendMetric {
    pub kind: SpendMetricKind,
}

impl SpendMetric {
    fn new(kind: SpendMetricKind) -> Self {
        Self { kind }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpendProjection {
    pub used: Option<SpendAmount>,
    pub limit: Option<SpendAmount>,
    pub percent: Option<f64>,
    pub metrics: Vec<SpendMetric>,
}

pub fn spend_projection(facts: SpendFacts) -> SpendProjection {
    use SpendAmount as Amount;
    use SpendMetricKind as Kind;

    let used = if facts.used_credits {
        Some(Amount::Credits)
    } else if facts.used_money {
        Some(Amount::Money)
    } else {
        None
    };
    let limit = if facts.limit_credits {
        Some(Amount::Credits)
    } else if facts.limit_money {
        Some(Amount::Money)
    } else {
        None
    };
    let amounts = used.is_some() || limit.is_some();
    let mut metrics = Vec::new();
    if facts.remaining_credits {
        metrics.push(SpendMetric::new(Kind::Remaining));
    }
    if facts.allowance_source {
        metrics.push(SpendMetric::new(Kind::AllowanceSource));
    }
    if facts.balance {
        metrics.push(SpendMetric::new(Kind::Balance));
    }
    if facts.reset {
        metrics.push(SpendMetric::new(Kind::Resets));
    }
    if facts.auto_reload {
        metrics.push(SpendMetric::new(Kind::AutoReload));
    }
    if facts.purchase {
        metrics.push(SpendMetric::new(Kind::Purchase));
    }
    SpendProjection {
        used: amounts.then_some(used.unwrap_or(Amount::Unavailable)),
        limit: amounts.then_some(limit.unwrap_or(Amount::UnreportedLimit)),
        percent: facts.percent,
        metrics,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/usage/spend/tests.rs"]
mod tests;
