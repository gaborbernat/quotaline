//! Allocates chart points across series.

pub fn point_limits(counts: &[usize], per_series: usize, total: usize) -> Vec<usize> {
    let capacities = counts.iter().map(|count| (*count).min(per_series)).collect::<Vec<_>>();
    let mut active = capacities
        .iter()
        .enumerate()
        .filter_map(|(index, capacity)| (*capacity > 0).then_some(index))
        .collect::<Vec<_>>();
    let mut remaining = total.max(active.len());
    let mut limits = vec![0; counts.len()];
    while remaining > 0 && !active.is_empty() {
        let share = (remaining / active.len()).max(1);
        let mut next = Vec::new();
        for index in active {
            if remaining == 0 {
                next.push(index);
                continue;
            }
            let grant = (capacities[index] - limits[index]).min(share).min(remaining);
            limits[index] += grant;
            remaining -= grant;
            if limits[index] < capacities[index] {
                next.push(index);
            }
        }
        active = next;
    }
    limits
}

#[cfg(test)]
#[path = "../tests/unit/budget/tests.rs"]
mod tests;
