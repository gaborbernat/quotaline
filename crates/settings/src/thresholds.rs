//! Keeps the stored notification thresholds to distinct percentages between 1 and 100.

pub const DEFAULT_THRESHOLDS: [u8; 3] = [75, 90, 100];

pub fn normalize_thresholds<T: Into<i32>>(thresholds: impl IntoIterator<Item = T>) -> Vec<u8> {
    let mut normalized = thresholds
        .into_iter()
        .map(Into::into)
        .filter(|threshold| (1..=100).contains(threshold))
        .map(|threshold| threshold as u8)
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized.dedup();
    normalized
}

pub fn set_threshold<T: Into<i32>>(thresholds: impl IntoIterator<Item = T>, target: i32, enabled: bool) -> Vec<u8> {
    let mut values = thresholds
        .into_iter()
        .map(Into::into)
        .filter(|threshold| *threshold != target || enabled)
        .collect::<Vec<_>>();
    if enabled {
        values.push(target);
    }
    normalize_thresholds(values)
}

#[cfg(test)]
#[path = "../tests/unit/thresholds/tests.rs"]
mod tests;
