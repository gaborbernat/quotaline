//! Exposes shared policy and analytics through the macOS UniFFI boundary.

pub mod access;
pub mod claude;
pub mod credentials;
pub mod demo;
pub mod export;
pub mod history;
pub mod http;
pub mod labels;
pub mod launch;
pub mod logging;
pub mod login;
pub mod metric;
pub mod notices;
pub mod notifications;
pub mod order;
pub mod pace;
pub mod persistence;
pub mod provider_record;
pub mod providers;
pub mod quota;
pub mod reading_cache;
pub mod refresh;
pub mod reviews;
pub mod selection;
pub mod settings;
pub mod settings_context;
pub mod settings_list;
pub mod setup;
pub mod spend;
pub mod status_layout;
pub mod storage;
pub mod template;
pub mod text;
pub mod time;
pub mod usage;
pub mod watermark;
pub mod widget;
pub use providers::{antigravity, codex_reset, codex_usage, copilot, cursor, gemini};
#[cfg(test)]
#[path = "../tests/support/http_transport.rs"]
mod mock_http;
#[cfg(test)]
#[path = "../tests/support/server_scanner.rs"]
mod mock_scanner;
#[cfg(test)]
#[path = "../tests/support/clock.rs"]
mod test_clock;
#[cfg(test)]
#[path = "../tests/support/records.rs"]
mod test_records;
use quotaline_analytics::project;
use quotaline_refresh as core_refresh;
use quotaline_refresh::{RefreshPolicy, RefreshReason};
use quotaline_setup::action;
use quotaline_setup::action::{Availability, ProviderAction, RecoveryIssue};
use quotaline_text::deadline;
use quotaline_text::deadline::DeadlineKind;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct CodexAnalyticsPoint {
    pub day: String,
    pub metric: String,
    pub series: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct RefreshIntent {
    pub reason: RefreshReason,
    pub usage: RefreshPolicy,
    pub analytics: RefreshPolicy,
    pub providers: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct RefreshLimits {
    pub rate_limit_backoff: f64,
    pub minimum_backoff: f64,
    pub maximum_backoff: f64,
    pub network_backoff: f64,
    pub idle_threshold: f64,
}

impl From<RefreshIntent> for core_refresh::RefreshIntent {
    fn from(value: RefreshIntent) -> Self {
        Self {
            reason: value.reason,
            usage: value.usage,
            analytics: value.analytics,
            providers: value.providers.map(|providers| providers.into_iter().collect()),
        }
    }
}

impl From<core_refresh::RefreshIntent> for RefreshIntent {
    fn from(value: core_refresh::RefreshIntent) -> Self {
        Self {
            reason: value.reason,
            usage: value.usage,
            analytics: value.analytics,
            providers: value.providers.map(|providers| providers.into_iter().collect()),
        }
    }
}

#[uniffi::export]
pub fn account_fingerprint(email: String, organization: String) -> String {
    quotaline_quota::account_fingerprint(&email, &organization)
}

#[uniffi::export]
pub fn provider_primary_action(availability: Availability, issue: Option<RecoveryIssue>) -> ProviderAction {
    action::primary(availability, issue)
}

#[uniffi::export]
pub fn anonymous_project(identity: String) -> String {
    project::anonymous(&identity)
}

#[uniffi::export]
pub fn project_labels(identities: Vec<String>, hide_identity: bool) -> std::collections::HashMap<String, String> {
    project::labels(&identities, hide_identity).into_iter().collect()
}

#[uniffi::export]
pub fn next_deadline_update(kind: DeadlineKind, target: Option<f64>, now: f64) -> Option<f64> {
    deadline::next_update(kind, target, now)
}

#[uniffi::export]
pub fn refresh_limits() -> RefreshLimits {
    RefreshLimits {
        rate_limit_backoff: core_refresh::RATE_LIMIT_BACKOFF,
        minimum_backoff: core_refresh::MINIMUM_BACKOFF,
        maximum_backoff: core_refresh::MAXIMUM_BACKOFF,
        network_backoff: core_refresh::NETWORK_BACKOFF,
        idle_threshold: core_refresh::IDLE_THRESHOLD,
    }
}

#[uniffi::export]
pub fn rate_limit_backoff(strikes: u32, retry_after: Option<f64>, retry_interval: f64) -> f64 {
    core_refresh::rate_limit_backoff(strikes, retry_after, retry_interval)
}

uniffi::setup_scaffolding!();

#[cfg(test)]
#[path = "../tests/unit/tests.rs"]
mod tests;
