//! Projects cloud credit availability and meter state for both frontends.

use serde::Serialize;

use crate::CloudCredits;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloudCreditFacts {
    pub limit: f64,
    pub remaining: f64,
    pub expires_at: Option<i64>,
    pub locked: bool,
}

impl CloudCreditFacts {
    pub fn from_credits(credits: &CloudCredits) -> Self {
        Self {
            limit: credits.limit,
            remaining: credits.remaining,
            expires_at: credits
                .expires_at
                .as_deref()
                .and_then(quotaline_text::time::parse_iso_epoch)
                .map(|epoch| epoch as i64),
            locked: credits.locked_reason.is_some(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CloudCreditProjection {
    pub status: CloudCreditStatus,
    pub expiry_epoch: Option<i64>,
    pub used_percent: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudCreditStatus {
    Active,
    Locked,
    Expired,
}

pub fn cloud_credit_projection(facts: CloudCreditFacts, now: i64) -> CloudCreditProjection {
    let expired = facts.expires_at.is_some_and(|expiry| expiry <= now);
    let (status, expiry_epoch) = if facts.locked {
        (CloudCreditStatus::Locked, None)
    } else if expired {
        (CloudCreditStatus::Expired, None)
    } else {
        (CloudCreditStatus::Active, facts.expires_at)
    };
    CloudCreditProjection {
        status,
        expiry_epoch,
        used_percent: if facts.limit > 0.0 {
            (100.0 * (1.0 - facts.remaining / facts.limit)).clamp(0.0, 100.0)
        } else {
            0.0
        },
    }
}

#[cfg(test)]
#[path = "../../tests/unit/usage/cloud/tests.rs"]
mod tests;
