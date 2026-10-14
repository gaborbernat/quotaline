//! Runs the shared window chart projection after calendar bucketing.

use super::*;

fn inferred_cadence(points: &[WindowChartPoint], minimum: f64) -> f64 {
    let mut intervals = points
        .windows(2)
        .map(|pair| pair[1].at - pair[0].at)
        .filter(|interval| *interval > 0.0)
        .collect::<Vec<_>>();
    intervals.sort_by(f64::total_cmp);
    intervals
        .get((intervals.len().saturating_sub(1)) / 4)
        .map_or(minimum, |interval| minimum.max(*interval))
}

fn sample(point: &WindowChartPoint) -> (f64, f64, bool) {
    (point.at, point.value, point.is_reset)
}

fn segmented(points: Vec<WindowChartPoint>, gaps: &[f64]) -> Vec<WindowChartPoint> {
    let mut segment = 0;
    points
        .into_iter()
        .map(|mut point| {
            if gaps.binary_search_by(|gap| gap.total_cmp(&point.at)).is_ok() {
                segment += 1;
            }
            point.segment = segment;
            point
        })
        .collect()
}

fn window_series(
    samples: &[&WindowSample],
    request: &WindowChartRequest,
    end: f64,
    key: &str,
    visible: bool,
) -> Option<WindowChartSeries> {
    let minimum = request.rollup.seconds().max(super::super::SAMPLE_INTERVAL);
    let raw = bucket_from(
        samples.iter().copied(),
        request.rollup,
        &request.time_zone,
        request.fallback_offset,
    );
    let cadence = inferred_cadence(&raw, minimum);
    let raw = insert_reset_zeros(&raw);
    let raw = window_clip(&raw, request.start, end, cadence, request.includes_end);
    let gaps = raw
        .windows(2)
        .filter_map(|pair| (pair[1].at - pair[0].at > cadence * 1.5).then_some(pair[1].at))
        .collect::<Vec<_>>();
    let raw = window_change_points(&raw, Some(cadence));
    let raw = window_extend_to(&raw, end, Some(cadence));
    let points = downsample(&raw, 400, Some(cadence), sample);
    let summary_value = points.last()?.value;
    Some(WindowChartSeries {
        key: key.into(),
        points: segmented(points, &gaps),
        visible,
        summary_value,
    })
}

fn budgeted(series: &mut [WindowChartSeries]) {
    let limits = point_limits(
        &series.iter().map(|line| line.points.len()).collect::<Vec<_>>(),
        400,
        1_200,
    );
    for (line, limit) in series.iter_mut().zip(limits) {
        if line.points.len() > limit {
            line.points = downsample(&line.points, limit, None, sample);
        }
    }
}

pub fn window_chart(samples: &[WindowSample], request: &WindowChartRequest) -> WindowChart {
    let end = request.end.min(request.now);
    let wanted = request.keys.iter().map(String::as_str).collect::<HashSet<_>>();
    let visible = request.visible.iter().map(String::as_str).collect::<HashSet<_>>();
    let mut grouped = BTreeMap::<&str, Vec<&WindowSample>>::new();
    let mut data_point_count = 0;
    for sample in samples.iter().filter(|sample| wanted.contains(sample.key.as_str())) {
        if sample.at >= request.start
            && (if request.includes_end {
                sample.at <= end
            } else {
                sample.at < end
            })
        {
            data_point_count += 1;
        }
        grouped.entry(&sample.key).or_default().push(sample);
    }
    let mut seen = HashSet::new();
    let lines = request
        .keys
        .iter()
        .filter(|key| seen.insert(key.as_str()))
        .filter_map(|key| {
            window_series(
                grouped.get(key.as_str()).map_or(&[], Vec::as_slice),
                request,
                end,
                key,
                visible.contains(key.as_str()),
            )
        })
        .collect::<Vec<_>>();
    let mut resets = lines
        .iter()
        .flat_map(|line| {
            line.points.iter().filter_map(|point| {
                (point.is_reset)
                    .then_some(point.reset_at)
                    .flatten()
                    .map(|reset_at| WindowResetEvent {
                        key: line.key.clone(),
                        at: point.at,
                        reset_at,
                    })
            })
        })
        .collect::<Vec<_>>();
    resets.sort_by(|left, right| left.at.total_cmp(&right.at));
    // Edge copies span the period but are not readings, so selection skips dates only they produce.
    let read = lines
        .iter()
        .flat_map(|line| {
            line.points
                .iter()
                .filter(|point| !point.carried)
                .map(|point| point.at.to_bits())
        })
        .collect::<HashSet<_>>();
    let mut series = if request.stacked { window_stack(&lines) } else { lines };
    budgeted(&mut series);
    let y_max = series
        .iter()
        .filter(|line| line.visible)
        .flat_map(|line| line.points.iter().map(|point| point.value + point.stack_base))
        .max_by(f64::total_cmp)
        .unwrap_or(0.0)
        .max(100.0);
    let mut timeline = series
        .iter()
        .filter(|line| line.visible)
        .flat_map(|line| line.points.iter().map(|point| point.at))
        .filter(|at| read.contains(&at.to_bits()))
        .collect::<Vec<_>>();
    timeline.sort_by(f64::total_cmp);
    timeline.dedup();
    WindowChart {
        series,
        timeline,
        resets,
        y_max,
        data_point_count,
    }
}
