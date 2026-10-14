//! Maps refresh decisions from the shared core into UniFFI records.

use quotaline_refresh as refresh;
use quotaline_refresh::FetchOutcome as CoreFetchOutcome;

mod rediscovery;
mod session;
mod setup_observation;
mod status;
pub use session::{
    RefreshCompletion, RefreshCycle, RefreshOutcomeDecision, RefreshPlannedFetch, RefreshPlans,
    RefreshProviderSchedule, RefreshReadings, RefreshScheduleInputs, RefreshSession, RefreshSkipped,
};
pub use setup_observation::{
    SetupObservationInput, SetupObservationResult, SetupRestoredAvailability, setup_observation,
};
pub use status::{ProviderStatusRecord, provider_status_keep_shown_when_disabled, provider_status_settle};

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SnapshotTransitionInput {
    pub previous_account: Option<String>,
    pub fetched_account: Option<String>,
    pub previous_fetched_at: Option<f64>,
    pub fetched_at: Option<f64>,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SnapshotTransition {
    pub account_changed: bool,
    pub replace_snapshot: bool,
    pub record_history: bool,
}

#[uniffi::export]
pub fn snapshot_transition(input: SnapshotTransitionInput) -> SnapshotTransition {
    let decision = refresh::snapshot_transition(refresh::SnapshotTransitionInput {
        previous_account: input.previous_account.as_deref(),
        fetched_account: input.fetched_account.as_deref(),
        previous_fetched_at: input.previous_fetched_at,
        fetched_at: input.fetched_at,
        complete: input.complete,
    });
    SnapshotTransition {
        account_changed: decision.account_changed,
        replace_snapshot: decision.replace_snapshot,
        record_history: decision.record_history,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum RefreshFetchOutcome {
    Success,
    Partial,
    NotAuthenticated,
    NetworkUnavailable,
    RateLimited,
    Failed,
}

impl From<RefreshFetchOutcome> for CoreFetchOutcome {
    fn from(value: RefreshFetchOutcome) -> Self {
        match value {
            RefreshFetchOutcome::Success => Self::Success,
            RefreshFetchOutcome::Partial => Self::Partial,
            RefreshFetchOutcome::NotAuthenticated => Self::NotAuthenticated,
            RefreshFetchOutcome::NetworkUnavailable => Self::NetworkUnavailable,
            RefreshFetchOutcome::RateLimited => Self::RateLimited,
            RefreshFetchOutcome::Failed => Self::Failed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum RefreshSkip {
    Inactive,
    RetryBackoff,
    AnalyticsNotDue,
    NoWork,
}

impl From<refresh::RefreshSkip> for RefreshSkip {
    fn from(value: refresh::RefreshSkip) -> Self {
        match value {
            refresh::RefreshSkip::Inactive => Self::Inactive,
            refresh::RefreshSkip::RetryBackoff => Self::RetryBackoff,
            refresh::RefreshSkip::AnalyticsNotDue => Self::AnalyticsNotDue,
            refresh::RefreshSkip::NoWork => Self::NoWork,
        }
    }
}

#[uniffi::export]
pub fn polling_interval(minimum: f64, active_interval: f64, requested: f64, active: bool) -> f64 {
    refresh::polling_interval(minimum, active_interval, requested, active)
}

#[cfg(test)]
#[path = "../tests/unit/refresh/tests.rs"]
mod tests;
