use super::*;

fn point(day: &str, metric: &str, series: &str, value: f64) -> Point {
    Point {
        day: day.into(),
        provider: "codex".into(),
        metric: metric.into(),
        series: series.into(),
        value,
    }
}

fn snapshot(provider: &str, account: &str, points: Vec<Point>) -> AnalyticsSnapshot {
    AnalyticsSnapshot {
        provider: provider.into(),
        points,
        credit_events: Vec::new(),
        fetched_at: 1_777_675_200.0,
        account_fingerprint: Some(account.into()),
        covered_scopes: Vec::new(),
    }
}

fn event(id: &str, timestamp: f64, credits_used: f64) -> CreditEvent {
    CreditEvent {
        id: id.into(),
        timestamp,
        service: "Codex".into(),
        credits_used,
    }
}

#[test]
fn correction_removes_only_covered_metric_days() {
    let previous = snapshot(
        "codex",
        "account",
        vec![
            point("2026-04-30", "turns", "old", 1.0),
            point("2026-05-01", "turns", "old", 2.0),
            point("2026-05-01", "credits", "kept", 3.0),
        ],
    );
    let mut newer = snapshot("codex", "account", vec![point("2026-05-01", "turns", "new", 4.0)]);
    newer.covered_scopes = vec![CoverageScope {
        metrics: vec!["turns".into()],
        start_day: "2026-05-01".into(),
        end_day: "2026-05-01".into(),
    }];
    assert_eq!(
        merge_analytics(&previous, newer, 7).points,
        [
            point("2026-04-30", "turns", "old", 1.0),
            point("2026-05-01", "credits", "kept", 3.0),
            point("2026-05-01", "turns", "new", 4.0),
        ]
    );
}

#[test]
fn newer_points_replace_matching_keys() {
    let previous = snapshot("codex", "account", vec![point("2026-05-01", "turns", "cli", 2.0)]);
    let newer = snapshot("codex", "account", vec![point("2026-05-01", "turns", "cli", 4.0)]);
    assert_eq!(
        merge_analytics(&previous, newer, 1).points,
        [point("2026-05-01", "turns", "cli", 4.0)]
    );
}

#[test]
fn previous_points_do_not_cross_account_or_provider() {
    let mut previous = snapshot("codex", "old", vec![point("2026-05-01", "turns", "old", 1.0)]);
    previous.credit_events = vec![event("old", previous.fetched_at, 1.0)];
    for (provider, account) in [("codex", "new"), ("claude", "old")] {
        let newer = snapshot(provider, account, vec![point("2026-05-01", "turns", "new", 2.0)]);
        assert_eq!(
            merge_analytics(&previous, newer.clone(), 0),
            newer,
            "{provider}/{account}"
        );
    }
}

#[test]
fn retention_keeps_cutoff_and_today_and_rejects_future() {
    let cutoff = 1_777_075_200.0;
    let mut previous = snapshot(
        "codex",
        "account",
        vec![
            point("2026-04-24", "turns", "old", 1.0),
            point("2026-04-25", "turns", "kept", 2.0),
            point("2026-05-02", "turns", "future", 5.0),
        ],
    );
    previous.credit_events = vec![
        event("old", cutoff - 1.0, 1.0),
        event("retained", cutoff, 2.0),
        event("future", 1_777_675_200.0 + 86_400.0, 3.0),
    ];
    let mut newer = snapshot(
        "codex",
        "account",
        vec![
            point("2026-04-24", "turns", "old", 1.0),
            point("2026-05-01", "turns", "today", 3.0),
            point("2026-05-02", "turns", "future", 4.0),
        ],
    );
    newer.credit_events = vec![
        event("old", cutoff - 1.0, 1.0),
        event("cutoff", cutoff, 1.0),
        event("today", newer.fetched_at, 2.0),
        event("future", newer.fetched_at + 86_400.0, 3.0),
    ];
    let merged = merge_analytics(&previous, newer, 7);
    assert_eq!(
        merged.points,
        [
            point("2026-04-25", "turns", "kept", 2.0),
            point("2026-05-01", "turns", "today", 3.0)
        ]
    );
    assert_eq!(
        merged.credit_events,
        [
            event("cutoff", cutoff, 1.0),
            event("retained", cutoff, 2.0),
            event("today", 1_777_675_200.0, 2.0)
        ]
    );
}

#[test]
fn credit_events_replace_by_id_and_sort_by_time_then_id() {
    let at = 1_777_675_200.0;
    let mut previous = snapshot("codex", "account", Vec::new());
    previous.credit_events = vec![event("replaced", at - 60.0, 1.0), event("retained", at - 30.0, 2.0)];
    let mut newer = snapshot("codex", "account", Vec::new());
    newer.credit_events = vec![event("replaced", at, 3.0), event("a", at, 4.0)];
    assert_eq!(
        merge_analytics(&previous, newer, 1).credit_events,
        [
            event("retained", at - 30.0, 2.0),
            event("a", at, 4.0),
            event("replaced", at, 3.0)
        ]
    );
}
