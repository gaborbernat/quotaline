//! Checks the macOS analytics chart bridge with shared Rust data.

use super::*;

#[test]
fn chart_bridge_maps_series_and_missing_metrics() {
    let row = Point {
        provider: "codex".into(),
        day: "2026-08-29".into(),
        metric: "turns".into(),
        series: "total".into(),
        value: 3.0,
    };
    let chart = chart_analytics(vec![row], "turns".into(), vec![], false).unwrap();
    assert_eq!(chart.series[0].provider, "codex");
    assert_eq!(chart.series[0].points[0].value, 3.0);
    assert_eq!(chart.data_point_count, 1);
    assert_eq!(chart_analytics(vec![], "unknown".into(), vec![], false), None);
}

#[test]
fn daily_bucket_bridge_keeps_sorted_totals() {
    let points = vec![
        Point {
            provider: String::new(),
            day: "2026-08-02".into(),
            metric: "turns".into(),
            series: "b".into(),
            value: 0.0,
        },
        Point {
            provider: String::new(),
            day: "2026-08-01".into(),
            metric: "turns".into(),
            series: "a".into(),
            value: 2.0,
        },
        Point {
            provider: String::new(),
            day: "2026-08-01".into(),
            metric: "turns".into(),
            series: "a".into(),
            value: 3.0,
        },
    ];
    assert_eq!(
        chart_daily_buckets(points, "turns".into()),
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
}

#[test]
fn chart_bridge_respects_hidden_series() {
    let chart = chart_analytics(
        vec![Point {
            provider: "codex".into(),
            day: "2026-08-29".into(),
            metric: "turns".into(),
            series: "total".into(),
            value: 3.0,
        }],
        "turns".into(),
        vec![ChartSeriesKey {
            provider: "codex".into(),
            series: "total".into(),
        }],
        false,
    )
    .unwrap();
    assert!(!chart.series[0].visible);
}
