//! Exposes the shared refresh status rules to Swift's provider state.

use quotaline_refresh as refresh;

use super::RefreshOutcomeDecision;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ProviderStatusRecord {
    pub availability: String,
    pub last_error: Option<String>,
    pub warnings: Vec<String>,
    pub retry_not_before: Option<f64>,
    pub last_attempt: Option<f64>,
    pub last_analytics_attempt: Option<f64>,
    pub last_success: Option<f64>,
    pub refreshing: bool,
}

impl From<ProviderStatusRecord> for refresh::ProviderStatus {
    fn from(record: ProviderStatusRecord) -> Self {
        Self {
            availability: record.availability,
            last_error: record.last_error,
            warnings: record.warnings,
            retry_not_before: record.retry_not_before,
            last_attempt: record.last_attempt,
            last_analytics_attempt: record.last_analytics_attempt,
            last_success: record.last_success,
            refreshing: record.refreshing,
        }
    }
}

impl From<refresh::ProviderStatus> for ProviderStatusRecord {
    fn from(status: refresh::ProviderStatus) -> Self {
        Self {
            availability: status.availability,
            last_error: status.last_error,
            warnings: status.warnings,
            retry_not_before: status.retry_not_before,
            last_attempt: status.last_attempt,
            last_analytics_attempt: status.last_analytics_attempt,
            last_success: status.last_success,
            refreshing: status.refreshing,
        }
    }
}

#[uniffi::export]
pub fn provider_status_settle(
    previous: ProviderStatusRecord,
    decision: RefreshOutcomeDecision,
    warnings: Vec<String>,
    included_analytics: bool,
    active: bool,
    now: f64,
) -> ProviderStatusRecord {
    refresh::settle(
        &previous.into(),
        refresh::SettleInput {
            availability: &decision.availability,
            last_error: decision.last_error.as_deref(),
            retry_not_before: decision.retry_not_before,
            succeeded: decision.succeeded,
            warnings,
            included_analytics,
            active,
            now,
        },
    )
    .into()
}

#[uniffi::export]
pub fn provider_status_keep_shown_when_disabled(
    next: ProviderStatusRecord,
    previous: ProviderStatusRecord,
) -> ProviderStatusRecord {
    refresh::keep_shown_when_disabled(next.into(), &previous.into()).into()
}

#[cfg(test)]
#[path = "../../tests/unit/refresh/status/tests.rs"]
mod tests;
