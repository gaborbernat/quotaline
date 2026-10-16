//! Checks how one fetch result and a switch-off change the shown refresh status.

use super::*;

fn input(availability: &str, succeeded: bool, warnings: Vec<String>, active: bool) -> SettleInput<'_> {
    SettleInput {
        availability,
        last_error: (!succeeded).then_some("HTTP 503"),
        retry_not_before: (!succeeded).then_some(200.0),
        succeeded,
        warnings,
        included_analytics: succeeded,
        active,
        now: 100.0,
    }
}

fn previous() -> ProviderStatus {
    ProviderStatus {
        availability: "current".into(),
        last_error: Some("old error".into()),
        warnings: vec!["old warning".into()],
        retry_not_before: Some(50.0),
        last_attempt: Some(10.0),
        last_analytics_attempt: Some(10.0),
        last_success: Some(10.0),
        refreshing: true,
    }
}

#[test]
fn status_starts_loading() {
    assert_eq!(ProviderStatus::default().availability, "loading");
}

#[test]
fn success_stamps_the_attempt_and_success() {
    let status = settle(&previous(), input("current", true, vec!["new warning".into()], true));
    assert_eq!(
        status,
        ProviderStatus {
            availability: "current".into(),
            last_error: None,
            warnings: vec!["new warning".into()],
            retry_not_before: None,
            last_attempt: Some(100.0),
            last_analytics_attempt: Some(100.0),
            last_success: Some(100.0),
            refreshing: false,
        }
    );
}

#[test]
fn failure_keeps_the_last_success_and_analytics_attempt() {
    let status = settle(&previous(), input("unavailable", false, Vec::new(), true));
    assert_eq!(
        (
            status.last_success,
            status.last_analytics_attempt,
            status.retry_not_before
        ),
        (Some(10.0), Some(10.0), Some(200.0))
    );
    assert_eq!(status.last_error.as_deref(), Some("HTTP 503"));
}

#[test]
fn inactive_result_disables_and_keeps_what_was_shown() {
    let status = settle(
        &previous(),
        input("unavailable", false, vec!["new warning".into()], false),
    );
    assert_eq!(status.availability, "disabled");
    assert_eq!(status.last_error.as_deref(), Some("old error"));
    assert_eq!(status.warnings, ["old warning"]);
    assert_eq!(status.last_attempt, Some(100.0));
}

#[test]
fn disabling_drops_pending_work() {
    let status = keep_shown_when_disabled(ProviderStatus::default(), &previous());
    assert_eq!(
        status,
        ProviderStatus {
            availability: "disabled".into(),
            last_error: Some("old error".into()),
            warnings: vec!["old warning".into()],
            retry_not_before: None,
            last_attempt: None,
            last_analytics_attempt: None,
            last_success: Some(10.0),
            refreshing: false,
        }
    );
}

#[test]
fn disabling_a_new_provider_shows_nothing() {
    let status = keep_shown_when_disabled(ProviderStatus::default(), &ProviderStatus::default());
    assert_eq!((status.last_error, status.warnings.len()), (None, 0));
}

#[test]
fn disabling_keeps_warnings_the_new_state_already_has() {
    let next = ProviderStatus {
        warnings: vec!["new warning".into()],
        ..ProviderStatus::default()
    };
    assert_eq!(keep_shown_when_disabled(next, &previous()).warnings, ["new warning"]);
}
