//! Decides which provider cards and empty states appear in usage views.

use crate::Provider;

mod cloud;
mod credits;
mod identity;
mod local;
mod spend;
pub use cloud::{CloudCreditFacts, CloudCreditProjection, CloudCreditStatus, cloud_credit_projection};
pub use credits::{CreditMetric, CreditMetricKind, credit_metrics};
pub use identity::{IdentityPresentation, IdentityReplacement, identity_presentation};
pub use local::{LocalMetric, LocalMetricKind, local_metrics, local_money_fraction_digits};
pub use spend::{SpendAmount, SpendFacts, SpendMetric, SpendMetricKind, SpendProjection, spend_projection};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CardFacts {
    pub enabled: bool,
    pub snapshot: bool,
    pub analytics: bool,
    pub credential_usable: bool,
    pub supplemental: bool,
}

pub fn card_visible(facts: CardFacts) -> bool {
    facts.enabled && (facts.snapshot || facts.analytics || facts.credential_usable || facts.supplemental)
}

pub fn provider_card_visible(provider: &Provider, enabled: bool, local_usage: bool) -> bool {
    card_visible(CardFacts {
        enabled,
        snapshot: !provider.windows.is_empty(),
        analytics: local_usage,
        credential_usable: provider.credential_usable,
        supplemental: !provider.source.is_empty()
            || provider.spend.is_some()
            || provider.credit_balance.is_some()
            || provider.usage_credits.is_some()
            || provider.reset_credits.is_some()
            || provider.cloud_credits.is_some()
            || !provider.details.is_empty()
            || !provider.notices.is_empty()
            || !provider.warnings.is_empty(),
    })
}

#[cfg(test)]
#[path = "../tests/unit/usage/tests.rs"]
mod tests;
