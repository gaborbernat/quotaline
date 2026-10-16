use super::*;
use quotaline_analytics::{CoverageScope, CreditEvent, Point};

#[test]
fn ffi_merge_maps_points_events_and_coverage() {
    let previous = AnalyticsSnapshot {
        provider: "codex".into(),
        points: vec![
            Point {
                provider: "codex".into(),
                day: "2026-05-01".into(),
                metric: "turns".into(),
                series: "old".into(),
                value: 2.0,
            },
            Point {
                provider: "codex".into(),
                day: "2026-05-01".into(),
                metric: "credits".into(),
                series: "kept".into(),
                value: 5.0,
            },
        ],
        credit_events: vec![
            CreditEvent {
                id: "credit".into(),
                timestamp: 1_777_675_200.0,
                service: "Codex".into(),
                credits_used: 2.0,
            },
            CreditEvent {
                id: "retained".into(),
                timestamp: 1_777_675_100.0,
                service: "Codex".into(),
                credits_used: 1.0,
            },
        ],
        fetched_at: 1_777_675_200.0,
        account_fingerprint: Some("account".into()),
        covered_scopes: Vec::new(),
    };
    let newer = AnalyticsSnapshot {
        points: vec![Point {
            series: "new".into(),
            value: 3.0,
            ..previous.points[0].clone()
        }],
        credit_events: vec![CreditEvent {
            credits_used: 4.0,
            ..previous.credit_events[0].clone()
        }],
        covered_scopes: vec![CoverageScope {
            metrics: vec!["turns".into()],
            start_day: "2026-05-01".into(),
            end_day: "2026-05-01".into(),
        }],
        ..previous.clone()
    };
    assert_eq!(
        merge_provider_analytics(previous.clone(), newer.clone(), 7),
        AnalyticsSnapshot {
            points: vec![previous.points[1].clone(), newer.points[0].clone()],
            credit_events: vec![previous.credit_events[1].clone(), newer.credit_events[0].clone()],
            ..newer
        }
    );
}
