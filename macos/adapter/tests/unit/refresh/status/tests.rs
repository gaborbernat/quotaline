//! Checks the refresh status rules through the Swift boundary.

use super::*;

fn initial() -> ProviderStatusRecord {
    refresh::ProviderStatus::default().into()
}

fn decision(succeeded: bool) -> RefreshOutcomeDecision {
    RefreshOutcomeDecision {
        availability: if succeeded { "current" } else { "rateLimited" }.into(),
        retry_not_before: (!succeeded).then_some(300.0),
        last_error: (!succeeded).then(|| "HTTP 429. Next attempt 12:05.".into()),
        replace_snapshot: succeeded,
        record_history: succeeded,
        account_changed: false,
        activate_account: false,
        succeeded,
    }
}

#[test]
fn bridge_settles_a_rate_limited_result() {
    let status = provider_status_settle(initial(), decision(false), Vec::new(), false, true, 100.0);
    assert_eq!(status.availability, "rateLimited");
    assert_eq!(status.retry_not_before, Some(300.0));
    assert_eq!(status.last_error.as_deref(), Some("HTTP 429. Next attempt 12:05."));
    assert_eq!(status.last_attempt, Some(100.0));
}

#[test]
fn bridge_disables_and_keeps_the_last_success() {
    let succeeded = provider_status_settle(initial(), decision(true), Vec::new(), true, true, 100.0);
    let disabled = provider_status_keep_shown_when_disabled(
        ProviderStatusRecord {
            availability: "disabled".into(),
            ..initial()
        },
        succeeded,
    );
    assert_eq!(disabled.availability, "disabled");
    assert_eq!(disabled.last_success, Some(100.0));
    assert_eq!(disabled.last_analytics_attempt, None);
}
