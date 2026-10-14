//! Bridges shared cloud credit availability into Swift records.

use quotaline_quota::usage;
use quotaline_quota::usage::CloudCreditFacts;
use quotaline_quota::usage::CloudCreditStatus as CoreStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CloudCreditStatus {
    Active,
    Locked,
    Expired,
}

impl From<CoreStatus> for CloudCreditStatus {
    fn from(value: CoreStatus) -> Self {
        match value {
            CoreStatus::Active => Self::Active,
            CoreStatus::Locked => Self::Locked,
            CoreStatus::Expired => Self::Expired,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, uniffi::Record)]
pub struct CloudCreditFactsInput {
    pub limit: f64,
    pub remaining: f64,
    pub expires_at: Option<i64>,
    pub locked: bool,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct CloudCreditProjectionRecord {
    pub status: CloudCreditStatus,
    pub expiry_epoch: Option<i64>,
    pub used_percent: f64,
}

#[uniffi::export]
pub fn usage_cloud_credit_projection(input: CloudCreditFactsInput, now: i64) -> CloudCreditProjectionRecord {
    let projection = usage::cloud_credit_projection(
        CloudCreditFacts {
            limit: input.limit,
            remaining: input.remaining,
            expires_at: input.expires_at,
            locked: input.locked,
        },
        now,
    );
    CloudCreditProjectionRecord {
        status: projection.status.into(),
        expiry_epoch: projection.expiry_epoch,
        used_percent: projection.used_percent,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/usage/cloud/tests.rs"]
mod tests;
