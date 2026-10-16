//! Advances availability and retry state after a provider fetch.

use super::{failure_backoff_with_minute, rate_limit_backoff_with_minute};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchOutcome {
    Success,
    Partial,
    NotAuthenticated,
    NetworkUnavailable,
    RateLimited,
    Failed,
}

impl FetchOutcome {
    fn availability(self) -> &'static str {
        match self {
            Self::Success => "current",
            Self::Partial => "stale",
            Self::NotAuthenticated => "authenticationRequired",
            Self::NetworkUnavailable => "networkUnavailable",
            Self::RateLimited => "rateLimited",
            Self::Failed => "unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutcomeTransitionInput {
    pub outcome: FetchOutcome,
    pub rate_limit_strikes: u32,
    pub failure_strikes: u32,
    pub retry_after: Option<f64>,
    pub retry_interval: f64,
    pub now: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutcomeTransition {
    pub availability: &'static str,
    pub rate_limit_strikes: u32,
    pub failure_strikes: u32,
    pub retry_not_before: Option<f64>,
}

pub(super) fn outcome_transition_with_minute(input: OutcomeTransitionInput, minute_seconds: f64) -> OutcomeTransition {
    let availability = input.outcome.availability();
    match input.outcome {
        FetchOutcome::Success => OutcomeTransition {
            availability,
            rate_limit_strikes: 0,
            failure_strikes: 0,
            retry_not_before: None,
        },
        FetchOutcome::RateLimited => {
            let strikes = input.rate_limit_strikes.saturating_add(1);
            OutcomeTransition {
                availability,
                rate_limit_strikes: strikes,
                failure_strikes: 0,
                retry_not_before: Some(
                    input.now
                        + rate_limit_backoff_with_minute(
                            strikes,
                            input.retry_after,
                            input.retry_interval,
                            minute_seconds,
                        ),
                ),
            }
        }
        _ => {
            let strikes = input.failure_strikes.saturating_add(1);
            OutcomeTransition {
                availability,
                rate_limit_strikes: 0,
                failure_strikes: strikes,
                retry_not_before: Some(
                    input.now + failure_backoff_with_minute(strikes, input.retry_interval, minute_seconds),
                ),
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/transition/tests.rs"]
mod tests;
