//! Applies exhausted quotas to selected provider windows.

#[derive(Clone, Copy)]
pub struct RestrictionWindow<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub group: &'a str,
    pub used: f64,
    pub active: bool,
    pub reset_at: Option<f64>,
}

pub fn restriction_applies(
    provider: &str,
    limit: RestrictionWindow<'_>,
    window: RestrictionWindow<'_>,
    now: f64,
) -> bool {
    if !limit.active || limit.used < 100.0 || limit.reset_at.is_some_and(|reset| reset <= now) {
        return false;
    }
    if limit.id == window.id {
        return true;
    }
    match provider {
        "claude" => limit.id == "session" || limit.id == "weekly",
        "codex" if limit.group != "other" && window.group != "other" => {
            limit.id.rsplit_once(':').map_or("", |(prefix, _)| prefix)
                == window.id.rsplit_once(':').map_or("", |(prefix, _)| prefix)
        }
        _ => false,
    }
}

pub fn restrictive_indices(
    provider: &str,
    window: RestrictionWindow<'_>,
    candidates: &[RestrictionWindow<'_>],
    now: f64,
) -> Option<Vec<usize>> {
    let indices = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| restriction_applies(provider, *candidate, window, now).then_some(index))
        .collect::<Vec<_>>();
    indices
        .iter()
        .any(|index| candidates[*index].id != window.id)
        .then_some(indices)
}

pub fn projected_restriction_reset(candidates: &[RestrictionWindow<'_>], indices: &[usize]) -> Option<f64> {
    indices
        .iter()
        .map(|index| candidates[*index].reset_at)
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .max_by(f64::total_cmp)
}

#[cfg(test)]
#[path = "../tests/unit/restriction/tests.rs"]
mod tests;
