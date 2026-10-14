//! Carries one provider reading from the shared Rust projections to Swift, the same value GNOME receives.

use quotaline_quota as quota;
use serde::{Deserialize, Serialize};

mod into_provider;
mod reading;
pub use reading::{ProviderReadingRecord, ReadingSource};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct ProviderWindowRecord {
    pub id: String,
    pub label: String,
    pub used: f64,
    pub active: bool,
    pub reset: Option<String>,
    pub reset_by_day: bool,
    pub group: String,
    pub scope: Option<String>,
    pub severity: Option<String>,
    pub detail: String,
    pub duration_seconds: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct ProviderMessageRange {
    pub low: i64,
    pub high: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct ProviderUsageCreditsRecord {
    pub unlimited: bool,
    pub has_credits: Option<bool>,
    pub overage_limit_reached: bool,
    pub local_messages: Option<ProviderMessageRange>,
    pub cloud_messages: Option<ProviderMessageRange>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct ProviderNoticeRecord {
    pub kind: String,
    pub text: String,
    pub window_id: Option<String>,
    pub resets_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct ProviderRecord {
    pub id: String,
    pub plan: String,
    pub tier: Option<String>,
    pub identity: String,
    pub organization: String,
    pub account_fingerprint: Option<String>,
    pub subscription_active_until: Option<String>,
    pub windows: Vec<ProviderWindowRecord>,
    pub local_usage: Option<quota::LocalUsage>,
    pub spend: Option<quota::SpendControl>,
    pub details: Vec<quota::Detail>,
    pub credit_balance: Option<f64>,
    pub usage_credits: Option<ProviderUsageCreditsRecord>,
    pub reset_credits: Option<quota::ResetCredits>,
    pub cloud_credits: Option<quota::CloudCredits>,
    pub reset_credit_expiries: Vec<quota::CreditExpiry>,
    pub notices: Vec<ProviderNoticeRecord>,
    pub warnings: Vec<String>,
}

fn range((low, high): (i64, i64)) -> ProviderMessageRange {
    ProviderMessageRange { low, high }
}

impl From<quota::Window> for ProviderWindowRecord {
    fn from(window: quota::Window) -> Self {
        Self {
            id: window.id,
            label: window.label,
            used: window.used,
            active: window.active,
            reset: window.reset,
            reset_by_day: window.reset_precision == quota::ResetPrecision::Day,
            group: window.group,
            scope: window.scope,
            severity: window.severity,
            detail: window.detail,
            duration_seconds: window.duration_seconds,
        }
    }
}

impl From<quota::Provider> for ProviderRecord {
    fn from(provider: quota::Provider) -> Self {
        Self {
            account_fingerprint: provider.account_fingerprint(),
            id: provider.id,
            plan: provider.plan,
            tier: provider.tier,
            identity: provider.identity,
            organization: provider.organization,
            subscription_active_until: provider.subscription_active_until,
            windows: provider.windows.into_iter().map(Into::into).collect(),
            local_usage: provider.local_usage,
            spend: provider.spend,
            details: provider.details,
            credit_balance: provider.credit_balance,
            usage_credits: provider.usage_credits.map(|credits| ProviderUsageCreditsRecord {
                unlimited: credits.unlimited,
                has_credits: credits.has_credits,
                overage_limit_reached: credits.overage_limit_reached,
                local_messages: credits.local_messages.map(range),
                cloud_messages: credits.cloud_messages.map(range),
            }),
            reset_credits: provider.reset_credits,
            cloud_credits: provider.cloud_credits,
            reset_credit_expiries: provider.reset_credit_expiries,
            notices: provider
                .notices
                .into_iter()
                .map(|notice| ProviderNoticeRecord {
                    kind: notice.kind.as_str().into(),
                    text: notice.text,
                    window_id: notice.window_id,
                    resets_at: notice.resets_at,
                })
                .collect(),
            warnings: provider.warnings,
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/provider_record/tests.rs"]
mod tests;
