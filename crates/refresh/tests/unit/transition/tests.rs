use super::*;

#[test]
fn success_clears_retry_state() {
    assert_eq!(
        outcome_transition_with_minute(
            OutcomeTransitionInput {
                outcome: FetchOutcome::Success,
                rate_limit_strikes: 3,
                failure_strikes: 2,
                retry_after: None,
                retry_interval: 60.0,
                now: 1000.0,
            },
            60.0
        ),
        OutcomeTransition {
            availability: "current",
            rate_limit_strikes: 0,
            failure_strikes: 0,
            retry_not_before: None,
        }
    );
}

#[test]
fn rate_limit_uses_retry_after_and_clears_failure_strikes() {
    assert_eq!(
        outcome_transition_with_minute(
            OutcomeTransitionInput {
                outcome: FetchOutcome::RateLimited,
                rate_limit_strikes: 1,
                failure_strikes: 2,
                retry_after: Some(900.0),
                retry_interval: 60.0,
                now: 1000.0,
            },
            60.0
        ),
        OutcomeTransition {
            availability: "rateLimited",
            rate_limit_strikes: 2,
            failure_strikes: 0,
            retry_not_before: Some(1900.0),
        }
    );
}

#[test]
fn failures_share_backoff_but_keep_their_availability() {
    for (outcome, availability) in [
        (FetchOutcome::Partial, "stale"),
        (FetchOutcome::NotAuthenticated, "authenticationRequired"),
        (FetchOutcome::NetworkUnavailable, "networkUnavailable"),
        (FetchOutcome::Failed, "unavailable"),
    ] {
        assert_eq!(
            outcome_transition_with_minute(
                OutcomeTransitionInput {
                    outcome,
                    rate_limit_strikes: 2,
                    failure_strikes: 1,
                    retry_after: None,
                    retry_interval: 60.0,
                    now: 1000.0,
                },
                60.0
            ),
            OutcomeTransition {
                availability,
                rate_limit_strikes: 0,
                failure_strikes: 2,
                retry_not_before: Some(1120.0),
            },
            "{outcome:?}"
        );
    }
}
