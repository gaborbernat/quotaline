//! Checks daily totals, stable order, and metric filtering.

use super::*;

#[test]
fn buckets_combine_duplicates_and_keep_zero_series() {
    let points = [
        Point {
            day: "2026-08-02".into(),
            provider: "codex".into(),
            metric: "turns".into(),
            series: "b".into(),
            value: 0.0,
        },
        Point {
            day: "2026-08-01".into(),
            provider: "codex".into(),
            metric: "turns".into(),
            series: "a".into(),
            value: 2.0,
        },
        Point {
            day: "2026-08-01".into(),
            provider: "claude".into(),
            metric: "turns".into(),
            series: "a".into(),
            value: 3.0,
        },
        Point {
            day: "2026-08-01".into(),
            provider: "codex".into(),
            metric: "threads".into(),
            series: "a".into(),
            value: 50.0,
        },
    ];
    assert_eq!(
        daily_buckets(&points, "turns"),
        [
            DailyBucket {
                day: "2026-08-01".into(),
                series: "a".into(),
                value: 5.0
            },
            DailyBucket {
                day: "2026-08-02".into(),
                series: "b".into(),
                value: 0.0
            },
        ]
    );
    assert!(daily_buckets(&points, "unknown").is_empty());
}
