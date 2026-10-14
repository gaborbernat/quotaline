//! Checks the public behavior of the adjacent module.

use super::*;

fn intent(
    reason: RefreshReason,
    usage: RefreshPolicy,
    analytics: RefreshPolicy,
    providers: Option<&[&str]>,
) -> RefreshIntent {
    RefreshIntent {
        reason,
        usage,
        analytics,
        providers: providers.map(|ids| ids.iter().map(|id| (*id).into()).collect()),
    }
}

#[test]
fn merging_selects_each_stronger_priority() {
    let left = intent(RefreshReason::Export, RefreshPolicy::Skip, RefreshPolicy::Force, None);
    let right = intent(
        RefreshReason::Scheduled,
        RefreshPolicy::IfDue,
        RefreshPolicy::Skip,
        Some(&["claude"]),
    );
    assert_eq!(
        left.merged(right),
        intent(RefreshReason::Export, RefreshPolicy::IfDue, RefreshPolicy::Force, None)
    );
}

#[test]
fn merging_unions_explicit_provider_scopes() {
    assert_eq!(
        intent(
            RefreshReason::PopoverOpened,
            RefreshPolicy::IfDue,
            RefreshPolicy::Skip,
            Some(&["claude"]),
        )
        .merged(intent(
            RefreshReason::UserInitiated,
            RefreshPolicy::Force,
            RefreshPolicy::IfDue,
            Some(&["codex", "claude"]),
        )),
        intent(
            RefreshReason::UserInitiated,
            RefreshPolicy::Force,
            RefreshPolicy::IfDue,
            Some(&["claude", "codex"]),
        )
    );
}

#[test]
fn merging_unscopes_when_either_request_is_unscoped() {
    for (name, left, right) in [
        ("left", None, Some(&["claude"][..])),
        ("right", Some(&["claude"][..]), None),
    ] {
        assert_eq!(
            intent(RefreshReason::Scheduled, RefreshPolicy::Skip, RefreshPolicy::Skip, left)
                .merged(intent(
                    RefreshReason::Scheduled,
                    RefreshPolicy::Skip,
                    RefreshPolicy::Skip,
                    right
                ))
                .providers,
            None,
            "{name}"
        );
    }
}

#[test]
fn coverage_requires_both_policies() {
    let requested = intent(RefreshReason::Export, RefreshPolicy::IfDue, RefreshPolicy::IfDue, None);
    for (name, actual, expected) in [
        (
            "both",
            intent(
                RefreshReason::Scheduled,
                RefreshPolicy::Force,
                RefreshPolicy::Force,
                None,
            ),
            true,
        ),
        (
            "usage",
            intent(
                RefreshReason::Scheduled,
                RefreshPolicy::Skip,
                RefreshPolicy::Force,
                None,
            ),
            false,
        ),
        (
            "analytics",
            intent(
                RefreshReason::Scheduled,
                RefreshPolicy::Force,
                RefreshPolicy::Skip,
                None,
            ),
            false,
        ),
    ] {
        assert_eq!(actual.covers(&requested), expected, "{name}");
    }
}

#[test]
fn coverage_respects_provider_scope() {
    let all = intent(
        RefreshReason::Scheduled,
        RefreshPolicy::IfDue,
        RefreshPolicy::IfDue,
        None,
    );
    let one = intent(
        RefreshReason::Scheduled,
        RefreshPolicy::IfDue,
        RefreshPolicy::IfDue,
        Some(&["claude"]),
    );
    let two = intent(
        RefreshReason::Scheduled,
        RefreshPolicy::IfDue,
        RefreshPolicy::IfDue,
        Some(&["claude", "codex"]),
    );
    let other = intent(
        RefreshReason::Scheduled,
        RefreshPolicy::IfDue,
        RefreshPolicy::IfDue,
        Some(&["codex"]),
    );
    for (name, actual, requested, expected) in [
        ("unscoped", &all, &two, true),
        ("scoped cannot cover unscoped", &one, &all, false),
        ("superset", &two, &one, true),
        ("disjoint", &one, &other, false),
    ] {
        assert_eq!(actual.covers(requested), expected, "{name}");
    }
}

#[test]
fn due_respects_policy_and_last_attempt() {
    for (name, policy, last, expected) in [
        ("skip", RefreshPolicy::Skip, None, false),
        ("force", RefreshPolicy::Force, Some(999.0), true),
        ("first", RefreshPolicy::IfDue, None, true),
        ("early", RefreshPolicy::IfDue, Some(701.0), false),
        ("slack", RefreshPolicy::IfDue, Some(700.0), true),
    ] {
        assert_eq!(due(policy, last, 301.0, 1000.0), expected, "{name}");
    }
}

#[test]
fn polling_interval_respects_activity_and_provider_floor() {
    for (name, active, requested, expected) in [
        ("active ceiling", true, 300.0, 120.0),
        ("active floor", true, 30.0, 60.0),
        ("inactive requested", false, 300.0, 300.0),
        ("inactive floor", false, 30.0, 60.0),
    ] {
        assert_eq!(polling_interval(60.0, 120.0, requested, active), expected, "{name}");
    }
}

fn plan_input() -> RefreshPlanInput {
    RefreshPlanInput {
        reason: RefreshReason::Scheduled,
        usage: RefreshPolicy::IfDue,
        analytics: RefreshPolicy::IfDue,
        active: true,
        targeted: false,
        rate_limited: false,
        retry_not_before: None,
        last_attempt: None,
        last_analytics_attempt: None,
        usage_interval: 300.0,
        analytics_interval: 600.0,
        supports_analytics: true,
        now: 1000.0,
    }
}

#[test]
fn plan_requires_active_provider_except_targeted_probe() {
    let inactive = RefreshPlanInput {
        active: false,
        ..plan_input()
    };
    assert_eq!(plan(inactive), Err(RefreshSkip::Inactive));
    assert_eq!(
        plan(RefreshPlanInput {
            reason: RefreshReason::UserInitiated,
            ..inactive
        }),
        Err(RefreshSkip::Inactive)
    );
    assert_eq!(
        plan(RefreshPlanInput {
            reason: RefreshReason::UserInitiated,
            targeted: true,
            ..inactive
        }),
        Ok(true)
    );
}

#[test]
fn plan_respects_retry_block_and_manual_probe() {
    let blocked = RefreshPlanInput {
        retry_not_before: Some(1100.0),
        ..plan_input()
    };
    assert_eq!(plan(blocked), Err(RefreshSkip::RetryBackoff));
    assert_eq!(
        plan(RefreshPlanInput {
            reason: RefreshReason::UserInitiated,
            rate_limited: true,
            ..blocked
        }),
        Err(RefreshSkip::RetryBackoff)
    );
    assert_eq!(
        plan(RefreshPlanInput {
            reason: RefreshReason::UserInitiated,
            ..blocked
        }),
        Ok(true)
    );
    assert_eq!(
        plan(RefreshPlanInput {
            retry_not_before: Some(900.0),
            ..blocked
        }),
        Ok(true)
    );
}

#[test]
fn plan_decides_usage_and_analytics_independently() {
    let recent = RefreshPlanInput {
        last_attempt: Some(900.0),
        last_analytics_attempt: Some(900.0),
        ..plan_input()
    };
    for (name, input, expected) in [
        ("both due", plan_input(), Ok(true)),
        ("neither due", recent, Err(RefreshSkip::NoWork)),
        (
            "analytics not due",
            RefreshPlanInput {
                usage: RefreshPolicy::Skip,
                ..recent
            },
            Err(RefreshSkip::AnalyticsNotDue),
        ),
        (
            "analytics skipped",
            RefreshPlanInput {
                analytics: RefreshPolicy::Skip,
                ..plan_input()
            },
            Ok(false),
        ),
        (
            "usage skipped",
            RefreshPlanInput {
                usage: RefreshPolicy::Skip,
                ..plan_input()
            },
            Ok(true),
        ),
        (
            "all skipped",
            RefreshPlanInput {
                usage: RefreshPolicy::Skip,
                analytics: RefreshPolicy::Skip,
                ..plan_input()
            },
            Err(RefreshSkip::NoWork),
        ),
        (
            "analytics unsupported",
            RefreshPlanInput {
                usage: RefreshPolicy::Skip,
                supports_analytics: false,
                ..plan_input()
            },
            Err(RefreshSkip::NoWork),
        ),
        (
            "only usage due",
            RefreshPlanInput {
                last_analytics_attempt: Some(900.0),
                ..plan_input()
            },
            Ok(false),
        ),
        (
            "only analytics due",
            RefreshPlanInput {
                last_attempt: Some(900.0),
                ..plan_input()
            },
            Ok(true),
        ),
        (
            "retry forces usage",
            RefreshPlanInput {
                retry_not_before: Some(900.0),
                last_attempt: Some(900.0),
                last_analytics_attempt: Some(900.0),
                ..plan_input()
            },
            Ok(false),
        ),
        (
            "force overrides last attempt",
            RefreshPlanInput {
                usage: RefreshPolicy::Force,
                last_attempt: Some(900.0),
                ..plan_input()
            },
            Ok(true),
        ),
    ] {
        assert_eq!(plan(input), expected, "{name}");
    }
}

#[test]
fn next_deadline_uses_retry_or_earliest_poll() {
    let base = RefreshDeadlineInput {
        retry_not_before: None,
        last_attempt: Some(900.0),
        last_analytics_attempt: Some(900.0),
        usage_interval: 300.0,
        analytics_interval: 600.0,
        supports_analytics: true,
        now: 1000.0,
    };
    for (name, input, expected) in [
        ("usage first", base, 1200.0),
        (
            "analytics first",
            RefreshDeadlineInput {
                analytics_interval: 200.0,
                ..base
            },
            1100.0,
        ),
        (
            "no analytics",
            RefreshDeadlineInput {
                supports_analytics: false,
                ..base
            },
            1200.0,
        ),
        (
            "first usage",
            RefreshDeadlineInput {
                last_attempt: None,
                ..base
            },
            1000.0,
        ),
        (
            "first analytics",
            RefreshDeadlineInput {
                last_analytics_attempt: None,
                ..base
            },
            1000.0,
        ),
        (
            "future retry",
            RefreshDeadlineInput {
                retry_not_before: Some(1100.0),
                ..base
            },
            1100.0,
        ),
        (
            "past retry",
            RefreshDeadlineInput {
                retry_not_before: Some(900.0),
                ..base
            },
            1000.0,
        ),
    ] {
        assert_eq!(next_deadline(input), expected, "{name}");
    }
}

#[test]
fn staggered_interval_phases_only_pending_background_providers() {
    for (name, visible, count, pending, expected) in [
        ("phase", false, 3, true, 624.0),
        ("visible", true, 3, true, 600.0),
        ("single", false, 1, true, 600.0),
        ("settled", false, 3, false, 600.0),
    ] {
        assert_eq!(
            staggered_interval(600.0, 2, visible, count, pending),
            expected,
            "{name}"
        );
    }
}

#[test]
fn scheduled_usage_interval_combines_power_idle_and_ceiling() {
    for (name, throttle, visible, elapsed, maximum, expected) in [
        ("normal", false, false, 3599.0, 3600.0, 600.0),
        ("power", true, false, 3599.0, 3600.0, 1200.0),
        ("idle", false, false, 3600.0, 3600.0, 1800.0),
        ("visible", false, true, 3600.0, 3600.0, 600.0),
        ("ceiling", true, false, 3600.0, 3000.0, 3000.0),
    ] {
        assert_eq!(
            scheduled_usage_interval_with_idle_threshold(600.0, throttle, visible, elapsed, IDLE_THRESHOLD, maximum),
            expected,
            "{name}"
        );
    }
}

#[test]
fn scheduled_usage_interval_uses_supplied_idle_threshold() {
    assert_eq!(
        scheduled_usage_interval_with_idle_threshold(10.0, false, false, 2.0, 2.0, 30.0),
        30.0
    );
}

#[test]
fn rate_limit_backoff_honors_server_delay_and_local_ceiling() {
    for (name, strikes, retry_after, interval, expected) in [
        ("initial", 1, None, 60.0, 300.0),
        ("strike", 2, None, 60.0, 600.0),
        ("provider floor", 1, None, 400.0, 400.0),
        ("ceiling", 4, None, 60.0, 1800.0),
        ("server", 1, Some(2400.0), 60.0, 2400.0),
    ] {
        assert_eq!(rate_limit_backoff(strikes, retry_after, interval), expected, "{name}");
    }
}

#[test]
fn failure_backoff_uses_provider_floor_and_ceiling() {
    for (name, strikes, interval, expected) in [
        ("initial", 1, 30.0, 60.0),
        ("strike", 2, 30.0, 120.0),
        ("provider floor", 1, 300.0, 300.0),
        ("ceiling", 6, 60.0, 1800.0),
    ] {
        assert_eq!(failure_backoff_with_minute(strikes, interval, 60.0), expected, "{name}");
    }
}

#[test]
fn scaled_backoff_preserves_fast_gnome_clock() {
    assert_eq!(rate_limit_backoff_with_minute(2, Some(11.0), 1.0, 1.0), 11.0);
    assert_eq!(failure_backoff_with_minute(2, 1.0, 1.0), 2.0);
}
