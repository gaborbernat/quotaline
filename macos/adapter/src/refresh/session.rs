//! Exposes the shared refresh session to Swift: request coalescing, retry backoff, stagger, and attempt times.

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use quotaline_refresh as refresh;
use quotaline_refresh::Completion;
use quotaline_refresh::Cycle;
use quotaline_refresh::ProviderSchedule;
use quotaline_refresh::ScheduleInputs;

use super::{RefreshFetchOutcome, RefreshSkip};
use crate::RefreshIntent;
use crate::time::{Clock, ClockFormatter};

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct RefreshCycle {
    pub intent: RefreshIntent,
    pub generation: u64,
}

impl From<RefreshCycle> for Cycle {
    fn from(value: RefreshCycle) -> Self {
        Self {
            intent: value.intent.into(),
            generation: value.generation,
        }
    }
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct RefreshProviderSchedule {
    pub id: String,
    pub active: bool,
    pub minimum: f64,
    pub active_interval: f64,
    pub requested: f64,
    pub supports_analytics: bool,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct RefreshScheduleInputs {
    pub providers: Vec<RefreshProviderSchedule>,
    pub visible: bool,
    pub throttled: bool,
    pub analytics_interval: f64,
    pub maximum: f64,
    pub idle_threshold: f64,
    pub now: f64,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct RefreshPlannedFetch {
    pub provider: String,
    pub include_analytics: bool,
    pub retry_interval: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct RefreshSkipped {
    pub provider: String,
    pub skip: RefreshSkip,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct RefreshPlans {
    pub fetches: Vec<RefreshPlannedFetch>,
    pub skipped: Vec<RefreshSkipped>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct RefreshCompletion {
    pub generation: u64,
    pub provider: String,
    pub outcome: RefreshFetchOutcome,
    pub retry_after: Option<f64>,
    pub retry_interval: f64,
    pub included_analytics: bool,
    pub now: f64,
    /// The failure or partial-reading reason.
    pub message: Option<String>,
    pub readings: RefreshReadings,
}

/// Whose account the stored and fetched readings belong to, and when each was taken.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct RefreshReadings {
    pub previous_account: Option<String>,
    pub previous_fetched_at: Option<f64>,
    pub fetched_account: Option<String>,
    pub fetched_at: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct RefreshOutcomeDecision {
    pub availability: String,
    pub retry_not_before: Option<f64>,
    pub last_error: Option<String>,
    pub replace_snapshot: bool,
    pub record_history: bool,
    pub account_changed: bool,
    pub activate_account: bool,
    pub succeeded: bool,
}

#[derive(uniffi::Object)]
pub struct RefreshSession(Mutex<refresh::RefreshSession>);

#[uniffi::export]
impl RefreshSession {
    #[uniffi::constructor]
    pub fn new(now: f64) -> Self {
        Self(Mutex::new(refresh::RefreshSession::new(now)))
    }

    pub fn request(&self, intent: RefreshIntent) {
        self.0.lock().unwrap().request(intent.into());
    }

    pub fn has_pending(&self) -> bool {
        self.0.lock().unwrap().has_pending()
    }

    pub fn start(&self) -> Option<RefreshCycle> {
        self.0.lock().unwrap().start().map(|cycle| RefreshCycle {
            intent: cycle.intent.into(),
            generation: cycle.generation,
        })
    }

    pub fn finish(&self, cycle: RefreshCycle) {
        self.0.lock().unwrap().finish(&cycle.into());
    }

    pub fn cancel(&self) {
        self.0.lock().unwrap().cancel();
    }

    pub fn mark_visible(&self, now: f64) {
        self.0.lock().unwrap().mark_visible(now);
    }

    pub fn forget_analytics_attempts(&self) {
        self.0.lock().unwrap().forget_analytics_attempts();
    }

    pub fn retain_providers(&self, providers: Vec<String>) {
        self.0
            .lock()
            .unwrap()
            .retain(&providers.into_iter().collect::<BTreeSet<_>>());
    }

    pub fn plan(&self, cycle: RefreshCycle, inputs: RefreshScheduleInputs) -> RefreshPlans {
        let schedules = schedules(&inputs);
        let plans = self
            .0
            .lock()
            .unwrap()
            .plan(&cycle.into(), &schedule_inputs(&inputs, &schedules));
        RefreshPlans {
            fetches: plans
                .fetches
                .into_iter()
                .map(|fetch| RefreshPlannedFetch {
                    provider: fetch.provider,
                    include_analytics: fetch.include_analytics,
                    retry_interval: fetch.retry_interval,
                })
                .collect(),
            skipped: plans
                .skipped
                .into_iter()
                .map(|(provider, skip)| RefreshSkipped {
                    provider,
                    skip: skip.into(),
                })
                .collect(),
        }
    }

    /// Records a fetch result; returns `None` for a result from a cancelled cycle.
    pub fn complete(
        &self,
        completion: RefreshCompletion,
        clock: Arc<dyn ClockFormatter>,
    ) -> Option<RefreshOutcomeDecision> {
        let transition = self.0.lock().unwrap().complete(Completion {
            generation: completion.generation,
            provider: &completion.provider,
            outcome: completion.outcome.into(),
            retry_after: completion.retry_after,
            retry_interval: completion.retry_interval,
            included_analytics: completion.included_analytics,
            now: completion.now,
        })?;
        let readings = &completion.readings;
        let applied = refresh::apply(refresh::ApplyInput {
            outcome: completion.outcome.into(),
            message: completion.message.as_deref(),
            readings: refresh::Readings {
                previous_account: readings.previous_account.as_deref(),
                previous_fetched_at: readings.previous_fetched_at,
                fetched_account: readings.fetched_account.as_deref(),
                fetched_at: readings.fetched_at,
            },
            transition,
            now: completion.now,
            clock: &Clock(clock),
        });
        Some(RefreshOutcomeDecision {
            availability: applied.availability.into(),
            retry_not_before: applied.retry_not_before,
            last_error: applied.last_error,
            replace_snapshot: applied.replace_snapshot,
            record_history: applied.record_history,
            account_changed: applied.account_changed,
            activate_account: applied.activate_account,
            succeeded: applied.succeeded,
        })
    }

    pub fn next_deadline(&self, inputs: RefreshScheduleInputs) -> Option<f64> {
        let schedules = schedules(&inputs);
        self.0
            .lock()
            .unwrap()
            .next_deadline(&schedule_inputs(&inputs, &schedules))
    }
}

fn schedules(inputs: &RefreshScheduleInputs) -> Vec<ProviderSchedule<'_>> {
    inputs
        .providers
        .iter()
        .map(|schedule| ProviderSchedule {
            id: &schedule.id,
            active: schedule.active,
            minimum: schedule.minimum,
            active_interval: schedule.active_interval,
            requested: schedule.requested,
            supports_analytics: schedule.supports_analytics,
        })
        .collect()
}

fn schedule_inputs<'a>(inputs: &RefreshScheduleInputs, schedules: &'a [ProviderSchedule<'a>]) -> ScheduleInputs<'a> {
    ScheduleInputs {
        providers: schedules,
        visible: inputs.visible,
        throttled: inputs.throttled,
        analytics_interval: inputs.analytics_interval,
        maximum: inputs.maximum,
        idle_threshold: inputs.idle_threshold,
        now: inputs.now,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/refresh/session/tests.rs"]
mod tests;
