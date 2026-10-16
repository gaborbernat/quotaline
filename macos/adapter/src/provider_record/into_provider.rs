//! Turns a Swift-held provider record back into the shared provider value the Rust rules take.

use quotaline_quota as quota;

use super::{ProviderReadingRecord, ProviderRecord, ProviderWindowRecord};

impl ProviderWindowRecord {
    /// The reset as Unix seconds, for the status and widget rules that compare times numerically.
    pub(crate) fn reset_seconds(&self) -> Option<f64> {
        self.reset.as_deref().and_then(quotaline_text::time::parse_iso_epoch)
    }
}

impl From<ProviderWindowRecord> for quota::Window {
    fn from(record: ProviderWindowRecord) -> Self {
        let mut window = quota::Window::new(record.id, record.label, record.used, record.reset, record.group);
        window.active = record.active;
        window.reset_precision = if record.reset_by_day {
            quota::ResetPrecision::Day
        } else {
            quota::ResetPrecision::Instant
        };
        window.scope = record.scope;
        window.severity = record.severity;
        window.detail = record.detail;
        window.duration_seconds = record.duration_seconds;
        window
    }
}

impl From<ProviderRecord> for quota::Provider {
    fn from(record: ProviderRecord) -> Self {
        let mut provider = quota::Provider::empty(&record.id, "connected");
        provider.plan = record.plan;
        provider.tier = record.tier;
        provider.identity = record.identity;
        provider.organization = record.organization;
        provider.account_fingerprint = record.account_fingerprint;
        provider.subscription_active_until = record.subscription_active_until;
        provider.windows = record.windows.into_iter().map(Into::into).collect();
        provider.local_usage = record.local_usage;
        provider.spend = record.spend;
        provider.details = record.details;
        provider.credit_balance = record.credit_balance;
        provider.usage_credits = record.usage_credits.map(|credits| quota::UsageCredits {
            unlimited: credits.unlimited,
            has_credits: credits.has_credits,
            overage_limit_reached: credits.overage_limit_reached,
            local_messages: credits.local_messages.map(|range| (range.low, range.high)),
            cloud_messages: credits.cloud_messages.map(|range| (range.low, range.high)),
        });
        provider.reset_credits = record.reset_credits;
        provider.cloud_credits = record.cloud_credits;
        provider.reset_credit_expiries = record.reset_credit_expiries;
        provider.notices = record
            .notices
            .into_iter()
            .map(|notice| quota::Notice {
                kind: serde_json::from_value(serde_json::Value::String(notice.kind)).unwrap_or(quota::NoticeKind::Info),
                text: notice.text,
                window_id: notice.window_id,
                resets_at: notice.resets_at,
            })
            .collect();
        provider.warnings = record.warnings;
        provider
    }
}

impl From<ProviderReadingRecord> for quota::Provider {
    fn from(reading: ProviderReadingRecord) -> Self {
        let mut provider = Self::from(reading.provider);
        provider.updated = Some(reading.fetched_at as i64);
        provider
    }
}

#[cfg(test)]
#[path = "../../tests/unit/provider_record/into_provider/tests.rs"]
mod tests;
