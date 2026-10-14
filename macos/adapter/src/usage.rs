//! Exposes shared usage card decisions to Swift.

use quotaline_quota::usage;
use quotaline_quota::usage::CardFacts;

mod cloud;
mod credits;
mod identity;
mod local;
mod spend;
pub use cloud::{CloudCreditFactsInput, CloudCreditProjectionRecord, CloudCreditStatus, usage_cloud_credit_projection};
pub use credits::{
    CreditMetricKind, CreditMetricRecord, CreditRangeInput, CreditResetInput, CreditUsageInput, usage_credit_metrics,
};
pub use identity::{IdentityPresentation, IdentityReplacement, usage_identity_presentation};
pub use local::{LocalMetricKind, LocalMetricRecord, usage_local_metrics, usage_local_money_fraction_digits};
pub use spend::{
    SpendAmount, SpendFactsInput, SpendMetricKind, SpendMetricRecord, SpendProjectionRecord, usage_spend_projection,
};

#[uniffi::export]
pub fn usage_card_visible(enabled: bool, snapshot: bool, analytics: bool, credential_usable: bool) -> bool {
    usage::card_visible(CardFacts {
        enabled,
        snapshot,
        analytics,
        credential_usable,
        supplemental: false,
    })
}

#[cfg(test)]
#[path = "../tests/unit/usage/tests.rs"]
mod tests;
