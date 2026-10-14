//! Exercises the complete window projection across resets, gaps, and budgets.

use super::*;

fn request() -> WindowChartRequest {
    WindowChartRequest {
        keys: vec!["claude:session".into()],
        visible: vec!["claude:session".into()],
        start: 0.0,
        end: 2_100.0,
        now: 2_100.0,
        rollup: WindowRollup::Minute,
        stacked: false,
        time_zone: "UTC".into(),
        fallback_offset: 0,
        includes_end: true,
    }
}

#[test]
fn empty_chart_has_no_series_or_timeline() {
    let chart = window_chart(&[], &request());
    assert!(chart.series.is_empty());
    assert!(chart.timeline.is_empty());
    assert_eq!(chart.y_max, 100.0);
    assert_eq!(chart.data_point_count, 0);
}

#[test]
fn chart_deduplicates_repeated_window_keys() {
    let mut request = request();
    request.keys.push("claude:session".into());

    let chart = window_chart(&[sample(0.0, 20.0, None)], &request);

    assert_eq!(chart.series.len(), 1);
    assert_eq!(chart.series[0].key, "claude:session");
}

#[test]
fn chart_inserts_reset_boundary_and_extends_fresh_value() {
    let samples = [
        sample(-600.0, 10.0, None),
        sample(0.0, 80.0, Some(300.0)),
        sample(1_200.0, 5.0, Some(18_300.0)),
    ];
    let chart = window_chart(&samples, &request());
    assert_eq!(chart.data_point_count, 2);
    assert_eq!(chart.series.len(), 1);
    assert_eq!(
        chart.series[0]
            .points
            .iter()
            .map(|point| point.value)
            .collect::<Vec<_>>(),
        [80.0, 0.0, 5.0, 5.0]
    );
    assert_eq!(chart.resets.len(), 1);
    assert_eq!(chart.resets[0].at, 300.0);
    assert_eq!(chart.resets[0].reset_at, 300.0);
    // The value carried to the period end draws the line but is not a reading to select.
    assert_eq!(chart.timeline, [0.0, 300.0, 1_200.0]);
}

#[test]
fn chart_orders_multiple_reset_events() {
    let samples = [
        sample(0.0, 90.0, Some(300.0)),
        sample(600.0, 5.0, Some(900.0)),
        sample(1_200.0, 3.0, Some(1_500.0)),
    ];
    let chart = window_chart(&samples, &request());
    assert_eq!(
        chart.resets.iter().map(|event| event.at).collect::<Vec<_>>(),
        [300.0, 900.0]
    );
}

#[test]
fn chart_uses_observed_cadence_to_connect_sparse_samples() {
    let samples = (0..5)
        .map(|index| sample(index as f64 * 1_800.0, index as f64 * 10.0, None))
        .collect::<Vec<_>>();
    let mut request = request();
    request.end = 7_200.0;
    request.now = 7_200.0;
    let chart = window_chart(&samples, &request);
    assert_eq!(chart.series[0].points.len(), 5);
    assert_eq!(
        chart.series[0]
            .points
            .iter()
            .map(|point| point.segment)
            .collect::<Vec<_>>(),
        [0, 0, 0, 0, 0]
    );
}

#[test]
fn chart_segments_stale_gaps_and_excludes_hidden_timeline() {
    let samples = [
        sample(0.0, 20.0, None),
        sample(300.0, 20.0, None),
        sample(600.0, 20.0, None),
        sample(1_800.0, 20.0, None),
        sample(2_100.0, 20.0, None),
    ];
    let chart = window_chart(&samples, &request());
    assert_eq!(
        chart.series[0]
            .points
            .iter()
            .map(|point| point.segment)
            .collect::<Vec<_>>(),
        [0, 0, 1, 1]
    );
    let mut hidden = request();
    hidden.visible.clear();
    let chart = window_chart(&samples, &hidden);
    assert!(chart.timeline.is_empty());
    assert_eq!(chart.y_max, 100.0);
}

#[test]
fn chart_clamps_future_end_and_obeys_open_interval() {
    let mut request = request();
    request.now = 300.0;
    request.includes_end = false;
    let chart = window_chart(&[sample(0.0, 1.0, None), sample(300.0, 2.0, None)], &request);
    assert_eq!(chart.data_point_count, 1);
    assert_eq!(chart.series[0].points.last().unwrap().value, 1.0);
}

#[test]
fn stacked_chart_tracks_combined_axis_and_reset_events() {
    let mut request = request();
    request.keys.push("codex:weekly".into());
    request.visible.push("codex:weekly".into());
    request.stacked = true;
    let mut second = sample(0.0, 80.0, None);
    second.key = "codex:weekly".into();
    let chart = window_chart(&[sample(0.0, 80.0, None), second], &request);
    assert_eq!(chart.series.len(), 2);
    assert_eq!(chart.y_max, 160.0);
    assert_eq!(chart.series[1].points[0].stack_base, 80.0);
}

#[test]
fn chart_shares_point_budget_across_many_windows() {
    let mut request = request();
    request.keys = (0..13).map(|index| format!("codex:{index}")).collect();
    request.visible = request.keys.clone();
    request.end = 299.0 * 60.0;
    request.now = request.end;
    let samples = request
        .keys
        .iter()
        .flat_map(|key| {
            (0..300).map(move |index| WindowSample {
                key: key.clone(),
                at: index as f64 * 60.0,
                used: (index % 100) as f64,
                reset_at: None,
            })
        })
        .collect::<Vec<_>>();
    let chart = window_chart(&samples, &request);
    assert_eq!(chart.series.len(), 13);
    assert_eq!(chart.data_point_count, samples.len() as u64);
    assert!(chart.series.iter().map(|line| line.points.len()).sum::<usize>() <= 1_200);
}
