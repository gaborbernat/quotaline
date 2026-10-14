//! Quota reset chart boundary cases.

use super::*;

fn point(at: f64, value: f64, reset_at: Option<f64>) -> WindowChartPoint {
    WindowChartPoint {
        at,
        value,
        reset_at,
        is_reset: false,
        carried: false,
        segment: 0,
        stack_base: 0.0,
    }
}

#[test]
fn empty_and_single_series_keep_their_points() {
    assert!(insert_reset_zeros(&[]).is_empty());
    let single = point(100.0, 80.0, Some(120.0));
    assert_eq!(insert_reset_zeros(std::slice::from_ref(&single)), vec![single]);
}

#[test]
fn changed_reset_inserts_the_exact_boundary() {
    let points = [point(100.0, 80.0, Some(120.0)), point(180.0, 20.0, Some(3600.0))];
    let rendered = insert_reset_zeros(&points);
    assert_eq!(
        rendered.iter().map(|point| point.at).collect::<Vec<_>>(),
        [100.0, 120.0, 180.0]
    );
    assert_eq!(rendered[1].value, 0.0);
    assert_eq!(rendered[1].reset_at, Some(120.0));
    assert!(rendered[1].is_reset);
}

#[test]
fn changed_reset_with_lower_reading_inserts_a_cliff() {
    for old in [90.0, 180.0] {
        let rendered = insert_reset_zeros(&[point(100.0, 80.0, Some(old)), point(180.0, 20.0, Some(3600.0))]);
        assert_eq!(rendered[1].at, 179.0);
        assert_eq!(rendered[1].reset_at, Some(180.0));
        assert!(rendered[1].is_reset);
    }
}

#[test]
fn unchanged_or_missing_reset_does_not_insert_a_cliff() {
    for points in [
        [point(100.0, 80.0, None), point(180.0, 20.0, Some(3600.0))],
        [point(100.0, 80.0, Some(120.0)), point(180.0, 20.0, None)],
        [point(100.0, 80.0, Some(120.0)), point(180.0, 20.0, Some(120.0))],
        [point(100.0, 80.0, Some(3600.0)), point(180.0, 20.0, Some(120.0))],
        [point(100.0, 80.0, Some(90.0)), point(180.0, 90.0, Some(3600.0))],
    ] {
        assert_eq!(insert_reset_zeros(&points), points);
    }
}

fn samples(values: &[f64]) -> Vec<WindowChartPoint> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| point(index as f64 * 300.0, *value, None))
        .collect()
}

fn downsample_indices(points: &[WindowChartPoint], limit: usize, cadence: Option<f64>) -> Vec<usize> {
    let indexed = points.iter().cloned().enumerate().collect::<Vec<_>>();
    downsample(&indexed, limit, cadence, |(_, point)| {
        (point.at, point.value, point.is_reset)
    })
    .into_iter()
    .map(|(index, _)| index)
    .collect()
}

#[test]
fn downsample_preserves_small_series_and_empty_budget() {
    let points = samples(&[1.0, 2.0, 3.0]);
    assert!(downsample_indices(&points, 0, None).is_empty());
    assert_eq!(downsample_indices(&[], 3, None), Vec::<usize>::new());
    assert_eq!(downsample_indices(&points, 3, None), [0, 1, 2]);
    assert_eq!(downsample_indices(&points, 4, None), [0, 1, 2]);
}

#[test]
fn downsample_keeps_reset_and_gap_boundaries() {
    let mut points = samples(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    points[3].is_reset = true;
    points[6].at = 2500.0;
    for point in points.iter_mut().skip(7) {
        point.at += 1000.0;
    }
    let selected = downsample_indices(&points, 7, Some(300.0));
    assert!(selected.contains(&3));
    assert!(selected.contains(&5));
    assert!(selected.contains(&6));
    assert_eq!(selected.len(), 7);
    assert_eq!(downsample_indices(&points, 1, Some(300.0)), [0]);
    assert_eq!(downsample_indices(&points, 2, Some(300.0)), [0, 9]);
}

#[test]
fn downsample_uses_last_slot_for_largest_remaining_value() {
    let points = samples(&[1.0, 2.0, -90.0, 4.0, 5.0, 6.0, 7.0]);
    assert_eq!(downsample_indices(&points, 3, None), [0, 2, 6]);
}

#[test]
fn downsample_buckets_preserve_extrema_and_deduplicate_timestamps() {
    let points = samples(&[0.0, 9.0, 1.0, 8.0, 3.0, 4.0, 7.0, 2.0, 0.0]);
    let selected = downsample_indices(&points, 6, None);
    assert!(selected.contains(&1));
    assert!(selected.contains(&2));
    assert!(selected.contains(&7));
    assert!(selected.len() <= 6);

    let flat = samples(&[0.0, 1.0, 1.0, 1.0, 0.0]);
    assert_eq!(downsample_indices(&flat, 4, None), [0, 1, 4]);

    let mut repeated = samples(&[0.0, 5.0, 1.0, 4.0, 0.0]);
    repeated[2].at = repeated[1].at;
    assert_eq!(downsample_indices(&repeated, 4, None), [0, 2, 4]);
}

#[test]
fn downsample_spreads_many_protected_points() {
    let mut points = samples(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    for point in points.iter_mut().skip(2).take(6) {
        point.is_reset = true;
    }
    assert_eq!(downsample_indices(&points, 4, None), [0, 3, 6, 9]);
    assert_eq!(downsample_indices(&points, 8, None), [0, 2, 3, 4, 5, 6, 7, 9]);
}
