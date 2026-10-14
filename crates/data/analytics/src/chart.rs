//! Prepares reset boundaries and reduces history chart series.

use super::WindowChartPoint;

fn reset_zero(at: f64, reset_at: f64) -> WindowChartPoint {
    WindowChartPoint {
        at,
        value: 0.0,
        reset_at: Some(reset_at),
        is_reset: true,
        carried: false,
        segment: 0,
        stack_base: 0.0,
    }
}

pub fn insert_reset_zeros(points: &[WindowChartPoint]) -> Vec<WindowChartPoint> {
    let Some(first) = points.first() else {
        return Vec::new();
    };
    let mut rendered = Vec::with_capacity(points.len());
    rendered.push(first.clone());
    for pair in points.windows(2) {
        let previous = &pair[0];
        let current = &pair[1];
        if let (Some(old), Some(new)) = (previous.reset_at, current.reset_at)
            && new > old
        {
            if old > previous.at && old < current.at {
                rendered.push(reset_zero(old, old));
            } else if current.value < previous.value {
                rendered.push(reset_zero(current.at - 1.0, current.at));
            }
        }
        rendered.push(current.clone());
    }
    rendered
}

/// `sample` reads a point's time, value, and whether it marks a reset.
pub fn downsample<T: Clone>(
    points: &[T],
    limit: usize,
    cadence: Option<f64>,
    sample: impl Fn(&T) -> (f64, f64, bool),
) -> Vec<T> {
    let at = |index: usize| sample(&points[index]).0;
    let value = |index: usize| sample(&points[index]).1;
    let pick = |indices: Vec<usize>| indices.into_iter().map(|index| points[index].clone()).collect();
    if limit == 0 {
        return Vec::new();
    }
    if points.len() <= limit {
        return points.to_vec();
    }
    let mut protected = std::collections::BTreeSet::from([0, points.len() - 1]);
    protected.extend((0..points.len()).filter(|index| sample(&points[*index]).2));
    if let Some(cadence) = cadence {
        for index in 1..points.len() {
            if at(index) - at(index - 1) > cadence * 1.5 {
                protected.insert(index - 1);
                protected.insert(index);
            }
        }
    }
    let protected_indices: Vec<_> = protected.iter().copied().collect();
    let selected_protected = evenly_spaced(&protected_indices, limit.min(protected.len()));
    if selected_protected.len() == limit {
        return pick(selected_protected);
    }
    let budget = limit - selected_protected.len();
    if budget == 1 {
        let candidate = (0..points.len())
            .filter(|index| !protected.contains(index))
            .max_by(|left, right| value(*left).abs().total_cmp(&value(*right).abs()))
            .expect("a remaining slot requires an unprotected point");
        let mut selected = selected_protected;
        selected.push(candidate);
        selected.sort_unstable();
        return pick(selected);
    }
    let interior_count = points.len() - 2;
    let bucket_count = (budget / 2).max(1);
    let mut selected = Vec::with_capacity(limit);
    for bucket in 0..bucket_count {
        let lower = 1 + bucket * interior_count / bucket_count;
        let upper = 1 + ((bucket + 1) * interior_count / bucket_count).min(interior_count);
        let mut minimum = lower;
        let mut maximum = lower;
        for index in lower + 1..upper {
            if value(index) < value(minimum) {
                minimum = index;
            }
            if value(index) > value(maximum) {
                maximum = index;
            }
        }
        selected.push(minimum);
        if maximum != minimum {
            selected.push(maximum);
        }
    }
    selected.extend(selected_protected);
    selected.sort_by(|left, right| at(*left).total_cmp(&at(*right)));
    selected.dedup_by(|left, right| at(*left) == at(*right));
    pick(selected)
}

fn evenly_spaced(values: &[usize], limit: usize) -> Vec<usize> {
    if values.len() <= limit || limit <= 1 {
        return values.iter().take(limit).copied().collect();
    }
    (0..limit)
        .map(|index| values[(index as f64 * (values.len() - 1) as f64 / (limit - 1) as f64).round() as usize])
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/chart/tests.rs"]
mod tests;
