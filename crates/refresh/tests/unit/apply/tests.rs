//! Checks how one fetch result changes a provider's stored state.

use super::*;
use quotaline_text::test_clock::Clock;

const NOW: f64 = 1_000.0;

fn transition(availability: &'static str, retry_not_before: Option<f64>) -> OutcomeTransition {
    OutcomeTransition {
        availability,
        rate_limit_strikes: 0,
        failure_strikes: 0,
        retry_not_before,
    }
}

fn input(outcome: FetchOutcome, message: Option<&'static str>, transition: OutcomeTransition) -> ApplyInput<'static> {
    ApplyInput {
        outcome,
        message,
        readings: Readings {
            previous_account: Some("old"),
            previous_fetched_at: Some(900.0),
            fetched_account: Some("old"),
            fetched_at: Some(NOW),
        },
        transition,
        now: NOW,
        clock: &Clock,
    }
}

#[test]
fn success_replaces_the_reading_records_history_and_clears_the_error() {
    let applied = apply(input(
        FetchOutcome::Success,
        Some("ignored"),
        transition("current", None),
    ));
    assert_eq!(
        (applied.availability, applied.last_error, applied.succeeded),
        ("current", None, true)
    );
    assert!(applied.replace_snapshot);
    assert!(applied.record_history);
    assert!(!applied.account_changed);
}

#[test]
fn partial_reading_keeps_its_reason_and_skips_history() {
    let applied = apply(input(
        FetchOutcome::Partial,
        Some("Usage API offline"),
        transition("stale", Some(1_060.0)),
    ));
    assert_eq!(applied.last_error.as_deref(), Some("Usage API offline"));
    assert!(applied.replace_snapshot);
    assert!(!applied.record_history);
    assert!(!applied.succeeded);
}

#[test]
fn partial_reading_from_another_account_activates_that_account() {
    let mut partial = input(FetchOutcome::Partial, Some("offline"), transition("stale", None));
    partial.readings.fetched_account = Some("new");
    let applied = apply(partial);
    assert!(applied.account_changed);
    assert!(applied.activate_account);
}

#[test]
fn failure_keeps_the_previous_reading() {
    let applied = apply(input(
        FetchOutcome::NetworkUnavailable,
        Some("Network error: offline"),
        transition("networkUnavailable", Some(1_060.0)),
    ));
    assert_eq!(applied.availability, "networkUnavailable");
    assert_eq!(applied.last_error.as_deref(), Some("Network error: offline"));
    assert!(!applied.replace_snapshot);
    assert!(!applied.record_history);
    assert!(!applied.account_changed);
}

#[test]
fn rate_limit_names_the_next_attempt() {
    let applied = apply(input(
        FetchOutcome::RateLimited,
        Some("HTTP 429"),
        transition("rateLimited", Some(1_300.0)),
    ));
    assert_eq!(applied.last_error.as_deref(), Some("HTTP 429. Next attempt time 1300."));
    assert_eq!(applied.retry_not_before, Some(1_300.0));
}
