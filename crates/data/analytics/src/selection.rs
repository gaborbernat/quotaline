//! Advances a history reading selection through keyboard and button events.

pub fn step_reading(count: usize, selected: Option<usize>, directions: &[i32]) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let mut current = selected.map(|index| index as i64);
    for &direction in directions {
        let base = current.unwrap_or(if direction > 0 { -1 } else { count as i64 });
        current = Some((base + i64::from(direction)).clamp(0, count as i64 - 1));
    }
    current.map(|index| index as usize)
}

#[cfg(test)]
#[path = "../tests/unit/selection/tests.rs"]
mod tests;
