//! Prepares daily analytics series for platform charts.

use std::collections::{BTreeMap, BTreeSet};

use super::utc_day::midnight;
use super::{Point, downsample, history_projection, point_limits};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AnalyticsChartPoint {
    pub at: f64,
    pub value: f64,
    pub segment: u32,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AnalyticsChartSeries {
    pub provider: String,
    pub series: String,
    pub points: Vec<AnalyticsChartPoint>,
    pub visible: bool,
    pub summary_value: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AnalyticsChart {
    pub series: Vec<AnalyticsChartSeries>,
    pub timeline: Vec<f64>,
    pub y_max: f64,
    pub summary_total: f64,
    pub data_point_count: u64,
}

fn series_points(days: BTreeMap<&str, f64>, bars: bool) -> Vec<AnalyticsChartPoint> {
    let mut dated = days
        .into_iter()
        .filter_map(|(day, value)| midnight(day).map(|at| (at, value)))
        .collect::<Vec<_>>();
    dated.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut previous = None;
    let mut segment = 0;
    dated
        .into_iter()
        .map(|(at, value)| {
            if !bars && previous.is_some_and(|previous| at - previous > 1.5 * 86_400.0) {
                segment += 1;
            }
            previous = Some(at);
            AnalyticsChartPoint { at, value, segment }
        })
        .collect()
}

fn budget(series: &mut [AnalyticsChartSeries]) {
    let limits = point_limits(
        &series.iter().map(|item| item.points.len()).collect::<Vec<_>>(),
        400,
        1_200,
    );
    for (item, limit) in series.iter_mut().zip(limits) {
        if item.points.len() > limit {
            item.points = downsample(&item.points, limit, None, |point| (point.at, point.value, false));
        }
    }
}

pub fn analytics_chart(
    rows: &[Point],
    metric: &str,
    hidden: &[(String, String)],
    stacked: bool,
) -> Option<AnalyticsChart> {
    let definition = super::metadata::definition(metric)?;
    let filtered = rows.iter().filter(|row| row.metric == metric).collect::<Vec<_>>();
    let projection = history_projection(
        &filtered
            .iter()
            .map(|row| super::HistoryReading {
                provider: row.provider.clone(),
                bucket: row.day.clone(),
                series: row.series.clone(),
                value: row.value,
            })
            .collect::<Vec<_>>(),
    );
    let chart_rows = filtered
        .iter()
        .filter(|row| {
            !definition.parallel
                || row.series != "total"
                || !projection
                    .detailed_buckets
                    .contains(&(row.provider.clone(), row.day.clone()))
        })
        .collect::<Vec<_>>();
    let mut grouped = BTreeMap::<(&str, &str), BTreeMap<&str, f64>>::new();
    for row in &chart_rows {
        *grouped
            .entry((&row.provider, &row.series))
            .or_default()
            .entry(&row.day)
            .or_default() += row.value;
    }
    let hidden = hidden
        .iter()
        .map(|(provider, series)| (provider.as_str(), series.as_str()))
        .collect::<BTreeSet<_>>();
    let mut series = grouped
        .into_iter()
        .filter_map(|((provider, name), days)| {
            let points = series_points(days, definition.mark == "bars");
            if points.is_empty() {
                return None;
            }
            let summary_value = if definition.summary == "sum" {
                Some(points.iter().map(|point| point.value).sum())
            } else {
                points.last().map(|point| point.value)
            };
            Some(AnalyticsChartSeries {
                provider: provider.into(),
                series: name.into(),
                points,
                visible: !hidden.contains(&(provider, name)),
                summary_value,
            })
        })
        .collect::<Vec<_>>();
    budget(&mut series);
    let visible = series.iter().filter(|item| item.visible).collect::<Vec<_>>();
    let values = if stacked && definition.stackable {
        let mut totals = BTreeMap::<i64, f64>::new();
        for point in visible.iter().flat_map(|item| &item.points) {
            *totals.entry(point.at as i64).or_default() += point.value;
        }
        totals.into_values().collect::<Vec<_>>()
    } else {
        visible
            .iter()
            .flat_map(|item| item.points.iter().map(|point| point.value))
            .collect()
    };
    let maximum = values.into_iter().max_by(f64::total_cmp).unwrap_or(0.0);
    let y_max = if definition.unit == "percentage" {
        100.0
    } else if maximum > 0.0 {
        maximum * 1.08
    } else {
        1.0
    };
    let timeline = visible
        .iter()
        .flat_map(|item| item.points.iter().map(|point| point.at as i64))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|at| at as f64)
        .collect();
    let summary_total = if definition.parallel {
        projection.canonical_total
    } else {
        visible.iter().filter_map(|item| item.summary_value).sum()
    };
    Some(AnalyticsChart {
        series,
        timeline,
        y_max,
        summary_total,
        data_point_count: chart_rows.len() as u64,
    })
}

#[cfg(test)]
#[path = "../tests/unit/series/tests.rs"]
mod tests;
