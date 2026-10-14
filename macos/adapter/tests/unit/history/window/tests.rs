//! Checks the macOS window chart boundary using serialized record types.

use super::*;
use quotaline_analytics::WindowRollup;

fn sample(at: f64, used: f64) -> WindowSample {
    WindowSample {
        key: "claude:session".into(),
        at,
        used,
        reset_at: None,
    }
}

#[test]
fn bridge_maps_all_calendar_rollups() {
    let samples = vec![sample(10.0, 1.0), sample(80.0, 2.0)];
    for (rollup, expected) in [
        (WindowRollup::Minute, 2),
        (WindowRollup::Hour, 1),
        (WindowRollup::Day, 1),
    ] {
        let chart = chart_window(
            samples.clone(),
            WindowChartRequest {
                keys: vec!["claude:session".into()],
                visible: vec!["claude:session".into()],
                start: 10.0,
                end: 80.0,
                now: 80.0,
                rollup,
                stacked: false,
                time_zone: "UTC".into(),
                fallback_offset: 0,
                includes_end: true,
            },
        );
        assert_eq!(chart.series[0].points.len(), expected);
    }
}

#[test]
fn bridge_preserves_chart_points_and_reset_events() {
    let mut first = sample(0.0, 80.0);
    first.reset_at = Some(300.0);
    let mut second = sample(1_200.0, 5.0);
    second.reset_at = Some(18_300.0);
    let chart = chart_window(
        vec![first, second],
        WindowChartRequest {
            keys: vec!["claude:session".into()],
            visible: vec!["claude:session".into()],
            start: 0.0,
            end: 1_200.0,
            now: 1_200.0,
            rollup: WindowRollup::Minute,
            stacked: false,
            time_zone: "UTC".into(),
            fallback_offset: 0,
            includes_end: true,
        },
    );
    assert_eq!(chart.series.len(), 1);
    assert_eq!(
        chart.series[0]
            .points
            .iter()
            .map(|point| point.value)
            .collect::<Vec<_>>(),
        [80.0, 0.0, 5.0]
    );
    assert_eq!(chart.series[0].summary_value, 5.0);
    assert_eq!(chart.resets[0].key, "claude:session");
    assert_eq!(chart.timeline, [0.0, 300.0, 1_200.0]);
    assert_eq!(chart.data_point_count, 2);
}

#[test]
fn nearest_bridge_uses_the_shared_date_lookup() {
    assert_eq!(chart_window_nearest(vec![0.0, 600.0], 500.0), Some(600.0));
    assert_eq!(chart_window_nearest(vec![], 500.0), None);
}
