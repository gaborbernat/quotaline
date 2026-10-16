use super::*;
use crate::{RefreshPolicy, RefreshReason};

const NOW: f64 = 1_000_000.0;

fn schedule(id: &str) -> RefreshProviderSchedule {
    RefreshProviderSchedule {
        id: id.into(),
        active: true,
        minimum: 60.0,
        active_interval: 60.0,
        requested: 300.0,
        supports_analytics: true,
    }
}

fn inputs(providers: Vec<RefreshProviderSchedule>, now: f64) -> RefreshScheduleInputs {
    RefreshScheduleInputs {
        providers,
        visible: false,
        throttled: false,
        analytics_interval: 900.0,
        maximum: 1800.0,
        idle_threshold: 3600.0,
        now,
    }
}

fn intent(reason: RefreshReason, usage: RefreshPolicy, analytics: RefreshPolicy) -> RefreshIntent {
    RefreshIntent {
        reason,
        usage,
        analytics,
        providers: None,
    }
}

fn scheduled() -> RefreshIntent {
    intent(RefreshReason::Scheduled, RefreshPolicy::IfDue, RefreshPolicy::IfDue)
}

fn completion(cycle: &RefreshCycle, outcome: RefreshFetchOutcome) -> RefreshCompletion {
    RefreshCompletion {
        generation: cycle.generation,
        provider: "claude".into(),
        outcome,
        retry_after: None,
        retry_interval: 300.0,
        included_analytics: true,
        now: NOW,
        message: Some("HTTP 429".into()),
        readings: RefreshReadings {
            previous_account: None,
            previous_fetched_at: None,
            fetched_account: Some("account".into()),
            fetched_at: Some(NOW),
        },
    }
}

fn clock() -> Arc<dyn ClockFormatter> {
    Arc::new(crate::test_clock::Clock)
}

#[test]
fn ffi_session_plans_due_providers_with_analytics() {
    let session = RefreshSession::new(NOW);
    session.request(scheduled());
    let cycle = session.start().unwrap();
    assert_eq!(
        session.plan(cycle, inputs(vec![schedule("claude")], NOW)),
        RefreshPlans {
            fetches: vec![RefreshPlannedFetch {
                provider: "claude".into(),
                include_analytics: true,
                retry_interval: 300.0,
            }],
            skipped: Vec::new(),
        }
    );
}

#[test]
fn ffi_session_reports_each_skip_reason() {
    let session = RefreshSession::new(NOW);
    session.request(scheduled());
    let first = session.start().unwrap();
    session
        .complete(completion(&first, RefreshFetchOutcome::Failed), clock())
        .unwrap();
    session.finish(first);
    session.request(intent(
        RefreshReason::Scheduled,
        RefreshPolicy::IfDue,
        RefreshPolicy::Skip,
    ));
    let cycle = session.start().unwrap();
    let providers = vec![
        RefreshProviderSchedule {
            active: false,
            ..schedule("codex")
        },
        schedule("claude"),
    ];
    let mut plans = session.plan(cycle.clone(), inputs(providers, NOW + 10.0)).skipped;
    session.finish(cycle);
    session.request(intent(
        RefreshReason::UserInitiated,
        RefreshPolicy::Skip,
        RefreshPolicy::IfDue,
    ));
    let analytics = session.start().unwrap();
    plans.extend(
        session
            .plan(analytics.clone(), inputs(vec![schedule("claude")], NOW + 10.0))
            .skipped,
    );
    session.finish(analytics);
    session.request(intent(
        RefreshReason::UserInitiated,
        RefreshPolicy::Skip,
        RefreshPolicy::Skip,
    ));
    let idle = session.start().unwrap();
    plans.extend(session.plan(idle, inputs(vec![schedule("claude")], NOW + 10.0)).skipped);
    assert_eq!(
        plans.into_iter().map(|skipped| skipped.skip).collect::<Vec<_>>(),
        [
            RefreshSkip::Inactive,
            RefreshSkip::RetryBackoff,
            RefreshSkip::AnalyticsNotDue,
            RefreshSkip::NoWork,
        ]
    );
}

#[test]
fn ffi_session_maps_every_outcome() {
    for (outcome, availability) in [
        (RefreshFetchOutcome::Success, "current"),
        (RefreshFetchOutcome::Partial, "stale"),
        (RefreshFetchOutcome::NotAuthenticated, "authenticationRequired"),
        (RefreshFetchOutcome::NetworkUnavailable, "networkUnavailable"),
        (RefreshFetchOutcome::RateLimited, "rateLimited"),
        (RefreshFetchOutcome::Failed, "unavailable"),
    ] {
        let session = RefreshSession::new(NOW);
        session.request(scheduled());
        let cycle = session.start().unwrap();
        assert_eq!(
            session
                .complete(completion(&cycle, outcome), clock())
                .unwrap()
                .availability,
            availability,
            "{outcome:?}"
        );
    }
}

#[test]
fn ffi_session_drops_results_from_a_cancelled_cycle() {
    let session = RefreshSession::new(NOW);
    session.request(scheduled());
    let cycle = session.start().unwrap();
    session.request(intent(
        RefreshReason::UserInitiated,
        RefreshPolicy::Force,
        RefreshPolicy::Skip,
    ));
    session.cancel();
    assert!(!session.has_pending());
    assert_eq!(
        session.complete(completion(&cycle, RefreshFetchOutcome::Success), clock()),
        None
    );
}

#[test]
fn ffi_session_deadlines_follow_attempts_and_visibility() {
    let session = RefreshSession::new(NOW);
    assert_eq!(session.next_deadline(inputs(vec![schedule("claude")], NOW)), Some(NOW));
    session.request(scheduled());
    let cycle = session.start().unwrap();
    session
        .complete(completion(&cycle, RefreshFetchOutcome::Success), clock())
        .unwrap();
    session.finish(cycle);
    let later = NOW + 7200.0;
    assert_eq!(
        session.next_deadline(inputs(vec![schedule("claude")], later)),
        Some(NOW + 900.0)
    );
    session.mark_visible(later);
    assert_eq!(
        session.next_deadline(inputs(vec![schedule("claude")], later)),
        Some(NOW + 300.0)
    );
}

#[test]
fn ffi_session_forgets_analytics_and_removed_providers() {
    let session = RefreshSession::new(NOW);
    session.request(scheduled());
    let cycle = session.start().unwrap();
    session
        .complete(completion(&cycle, RefreshFetchOutcome::Failed), clock())
        .unwrap();
    session.finish(cycle);
    session.forget_analytics_attempts();
    session.retain_providers(vec!["codex".into()]);
    assert_eq!(session.next_deadline(inputs(vec![schedule("claude")], NOW)), Some(NOW));
}

#[test]
fn ffi_session_starts_a_cycle_for_the_requested_providers() {
    let session = RefreshSession::new(NOW);
    let request = RefreshIntent {
        providers: Some(vec!["claude".into()]),
        ..intent(RefreshReason::UserInitiated, RefreshPolicy::Force, RefreshPolicy::Skip)
    };
    session.request(request.clone());
    assert_eq!(session.start().unwrap().intent, request);
}

#[test]
fn ffi_session_returns_the_shared_result_decision() {
    let session = RefreshSession::new(NOW);
    session.request(scheduled());
    let cycle = session.start().unwrap();
    let success = session
        .complete(completion(&cycle, RefreshFetchOutcome::Success), clock())
        .unwrap();
    assert!(success.replace_snapshot);
    assert!(success.record_history);
    assert!(success.succeeded);
    assert!(success.account_changed);
    assert!(!success.activate_account);
    assert_eq!(success.last_error, None);
    let limited = session
        .complete(completion(&cycle, RefreshFetchOutcome::RateLimited), clock())
        .unwrap();
    assert!(limited.last_error.unwrap().starts_with("HTTP 429. Next attempt "));
}
