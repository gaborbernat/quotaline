//! Selects and groups projected status windows for platform renderers.

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusSelectionMode {
    All,
    WorstPerProvider,
    MiniBars,
    IconOnly,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusSelectionEntry<'a> {
    pub provider: &'a str,
    pub used: f64,
    pub restricted: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct StatusSelectionGroup {
    pub provider: String,
    pub indices: Vec<usize>,
    pub used: f64,
    pub restricted: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StatusSelection {
    pub indices: Vec<usize>,
    pub groups: Vec<StatusSelectionGroup>,
}

pub fn select_status(
    entries: &[StatusSelectionEntry<'_>],
    mode: StatusSelectionMode,
    hide_zero: bool,
    usage_order: bool,
) -> StatusSelection {
    let mut indices = if mode == StatusSelectionMode::IconOnly {
        Vec::new()
    } else {
        entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| (!hide_zero || entry.used > 0.0).then_some(index))
            .collect::<Vec<_>>()
    };
    if mode == StatusSelectionMode::WorstPerProvider {
        let mut positions: HashMap<&str, usize> = HashMap::new();
        let mut selected: Vec<usize> = Vec::new();
        for index in indices {
            let entry = entries[index];
            if let Some(&position) = positions.get(entry.provider) {
                let current = entries[selected[position]];
                if entry.restricted && !current.restricted
                    || entry.restricted == current.restricted && entry.used > current.used
                {
                    selected[position] = index;
                }
            } else {
                positions.insert(entry.provider, selected.len());
                selected.push(index);
            }
        }
        indices = selected;
    }
    if usage_order {
        indices.sort_by(|left, right| {
            entries[*right]
                .used
                .total_cmp(&entries[*left].used)
                .then_with(|| left.cmp(right))
        });
    }
    let mut groups = Vec::new();
    if mode == StatusSelectionMode::MiniBars {
        let mut positions = HashMap::new();
        for &index in &indices {
            let entry = entries[index];
            if let Some(&position) = positions.get(entry.provider) {
                let group: &mut StatusSelectionGroup = &mut groups[position];
                group.indices.push(index);
                group.used = group.used.max(entry.used);
                group.restricted |= entry.restricted;
            } else {
                positions.insert(entry.provider, groups.len());
                groups.push(StatusSelectionGroup {
                    provider: entry.provider.into(),
                    indices: vec![index],
                    used: entry.used,
                    restricted: entry.restricted,
                });
            }
        }
        if usage_order {
            groups.sort_by(|left, right| {
                right
                    .used
                    .total_cmp(&left.used)
                    .then_with(|| left.provider.cmp(&right.provider))
            });
        }
    }
    StatusSelection { indices, groups }
}

#[cfg(test)]
#[path = "../tests/unit/status_selection/tests.rs"]
mod tests;
