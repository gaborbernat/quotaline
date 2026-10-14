//! Checks the public behavior of the adjacent module.

use super::*;

fn window(used_percent: f64, duration_seconds: Option<f64>) -> LimitWindow {
    LimitWindow {
        used_percent,
        duration_seconds,
        reset_at: None,
        reset_after_seconds: None,
    }
}

fn group(kind: LimitKind, limit_reached: bool, primary: Option<LimitWindow>) -> LimitGroup {
    LimitGroup {
        kind,
        limit_reached,
        primary,
        secondary: None,
    }
}

#[test]
fn durations_keep_reference_ids_labels_and_groups() {
    let cases = [
        (None, "session", "5-hour", "session"),
        (Some(604_800.0), "weekly", "Weekly", "weekly"),
        (Some(2_592_000.0), "monthly", "Monthly", "monthly"),
        (Some(2_678_400.0), "monthly", "Monthly", "monthly"),
        (Some(86_400.0), "window-86400", "Daily", "other"),
        (Some(172_800.0), "window-172800", "2d", "other"),
        (Some(90_000.0), "window-90000", "25h", "other"),
        (Some(7200.0), "window-7200", "2h", "other"),
        (Some(900.0), "window-900", "15m", "other"),
    ];
    for (duration, id, label, kind) in cases {
        let result = windows(&[group(LimitKind::Usage, false, Some(window(25.0, duration)))], 100.0);
        assert_eq!((&*result[0].id, &*result[0].label, result[0].group), (id, label, kind));
        assert_eq!(result[0].duration_seconds, duration.unwrap_or(18_000.0));
    }
}

#[test]
fn groups_preserve_order_scope_and_both_windows() {
    let mut additional = group(
        LimitKind::Additional(" !Spark  Beta! ".into()),
        false,
        Some(window(5.0, None)),
    );
    additional.secondary = Some(window(10.0, Some(604_800.0)));
    let result = windows(
        &[
            group(LimitKind::Usage, false, None),
            group(LimitKind::CodeReview, false, Some(window(15.0, None))),
            additional,
        ],
        100.0,
    );
    assert_eq!(
        result.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        [
            "code_review:session",
            "additional:spark-beta:session",
            "additional:spark-beta:weekly",
        ]
    );
    assert_eq!(result[0].label, "Code review 5-hour");
    assert_eq!(result[1].label, " !Spark  Beta!  5-hour");
    assert_eq!(result[2].scope.as_deref(), Some(" !Spark  Beta! "));
    assert!(windows(&[], 100.0).is_empty());
}

#[test]
fn reset_and_severity_use_the_reported_limit() {
    let mut absolute = window(85.0, None);
    absolute.reset_at = Some(500.0);
    absolute.reset_after_seconds = Some(60.0);
    let mut relative = window(110.0, None);
    relative.reset_after_seconds = Some(60.0);
    let mut limited = group(LimitKind::CodeReview, true, Some(window(-5.0, None)));
    limited.secondary = Some(window(74.0, None));
    let result = windows(
        &[
            group(LimitKind::Usage, false, Some(absolute)),
            group(LimitKind::Usage, false, Some(relative)),
            limited,
        ],
        100.0,
    );
    assert_eq!(result[0].reset_at, Some(500.0));
    assert_eq!(result[0].severity, "warning");
    assert_eq!(result[1].reset_at, Some(160.0));
    assert_eq!(result[1].used_percent, 100.0);
    assert_eq!(result[1].severity, "critical");
    assert_eq!(result[2].used_percent, 0.0);
    assert_eq!(result[2].severity, "critical");
    assert_eq!(result[3].severity, "critical");
    assert_eq!(result[3].reset_at, None);
    assert_eq!(
        windows(&[group(LimitKind::Usage, false, Some(window(20.0, None)))], 100.0)[0].severity,
        "normal"
    );
}

#[test]
fn local_readings_replace_only_unreachable_or_unauthenticated_usage() {
    assert_eq!(
        [
            ProviderFailureKind::NotAuthenticated,
            ProviderFailureKind::NetworkUnavailable,
            ProviderFailureKind::RateLimited,
            ProviderFailureKind::Failed,
        ]
        .map(local_fallback),
        [true, true, false, false]
    );
}
