//! Decides how one fetch result changes a provider's stored state, the same way on both desktops.

use quotaline_text::time::{ClockFormat, ResetPrecision, reset_clock};

use crate::{FetchOutcome, OutcomeTransition, SnapshotTransitionInput, snapshot_transition};

/// What the stored state and the fetched reading say about whose account they belong to and when they were taken.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Readings<'a> {
    pub previous_account: Option<&'a str>,
    pub previous_fetched_at: Option<f64>,
    pub fetched_account: Option<&'a str>,
    pub fetched_at: Option<f64>,
}

#[derive(Clone, Copy)]
pub struct ApplyInput<'a> {
    pub outcome: FetchOutcome,
    /// The failure or partial-reading reason; ignored for a success.
    pub message: Option<&'a str>,
    pub readings: Readings<'a>,
    pub transition: OutcomeTransition,
    pub now: f64,
    pub clock: &'a dyn ClockFormat,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Applied {
    pub availability: &'static str,
    pub last_error: Option<String>,
    pub retry_not_before: Option<f64>,
    /// The fetched reading becomes the stored one.
    pub replace_snapshot: bool,
    pub record_history: bool,
    /// Analytics and pace notifications of the previous account no longer apply.
    pub account_changed: bool,
    /// A partial reading switched accounts, so history must follow the new account before it records.
    pub activate_account: bool,
    /// A full success clears sign-in notifications and stamps the last success.
    pub succeeded: bool,
}

pub fn apply(input: ApplyInput<'_>) -> Applied {
    let fetched = matches!(input.outcome, FetchOutcome::Success | FetchOutcome::Partial);
    let snapshot = snapshot_transition(SnapshotTransitionInput {
        previous_account: input.readings.previous_account,
        fetched_account: input.readings.fetched_account,
        previous_fetched_at: input.readings.previous_fetched_at,
        fetched_at: input.readings.fetched_at,
        complete: input.outcome == FetchOutcome::Success,
    });
    let account_changed = fetched && snapshot.account_changed;
    let message = input.message.filter(|_| input.outcome != FetchOutcome::Success);
    let last_error = match (input.outcome, input.transition.retry_not_before) {
        (FetchOutcome::RateLimited, Some(until)) => message.map(|reason| {
            let clock = reset_clock(until as i64, ResetPrecision::Instant, input.now as i64, input.clock);
            format!("{reason}. Next attempt {clock}.")
        }),
        _ => message.map(str::to_owned),
    };
    Applied {
        availability: input.transition.availability,
        last_error,
        retry_not_before: input.transition.retry_not_before,
        replace_snapshot: fetched && snapshot.replace_snapshot,
        record_history: fetched && snapshot.record_history,
        account_changed,
        activate_account: input.outcome == FetchOutcome::Partial && account_changed,
        succeeded: input.outcome == FetchOutcome::Success,
    }
}

#[cfg(test)]
#[path = "../tests/unit/apply/tests.rs"]
mod tests;
