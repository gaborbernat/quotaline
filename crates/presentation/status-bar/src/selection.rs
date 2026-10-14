//! Manages quota-window selection shared by desktop frontends.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use super::{Provider, Settings, default_selected_windows};

pub fn selected_window_keys(settings: &Settings, providers: &BTreeMap<String, Provider>) -> Vec<String> {
    if settings.has_custom_selection == Some(true) {
        settings.selected_windows.clone()
    } else {
        default_selected_windows(providers)
    }
}

pub fn history_window_selection(
    settings: &Settings,
    providers: &BTreeMap<String, Provider>,
) -> Option<BTreeSet<String>> {
    let selected = selected_window_keys(settings, providers);
    (settings.has_custom_selection == Some(true) || !selected.is_empty()).then(|| selected.into_iter().collect())
}

pub fn available_selected_window_keys(settings: &Settings, providers: &BTreeMap<String, Provider>) -> Vec<String> {
    let available = providers
        .iter()
        .flat_map(|(id, provider)| provider.windows.iter().map(move |window| format!("{id}:{}", window.id)))
        .collect::<Vec<_>>();
    available_window_selection(&selected_window_keys(settings, providers), &available)
}

pub fn available_window_selection(selected: &[String], available: &[String]) -> Vec<String> {
    let available = available.iter().collect::<HashSet<_>>();
    selected.iter().filter(|key| available.contains(key)).cloned().collect()
}

pub fn default_window_ids<'a>(windows: impl IntoIterator<Item = (&'a str, bool)>) -> Vec<&'a str> {
    let active = windows
        .into_iter()
        .filter_map(|(id, active)| active.then_some(id))
        .collect::<Vec<_>>();
    let preferred = active
        .iter()
        .copied()
        .filter(|id| *id == "session" || *id == "weekly" || id.starts_with("weekly:"))
        .collect::<Vec<_>>();
    if preferred.is_empty() {
        active.into_iter().take(2).collect()
    } else {
        preferred
    }
}

pub fn toggle_window(selected: &[String], key: &str) -> Option<Vec<String>> {
    let mut next = selected.to_vec();
    if next.iter().any(|value| value == key) {
        next.retain(|value| value != key);
    } else {
        next.push(key.to_owned());
    }
    (!next.is_empty()).then_some(next)
}

pub fn toggle_provider_windows(selected: &[String], keys: &[String]) -> Option<Vec<String>> {
    let mut next = selected.to_vec();
    if keys.iter().all(|key| next.contains(key)) {
        next.retain(|key| !keys.contains(key));
    } else {
        for key in keys {
            if !next.contains(key) {
                next.push(key.clone());
            }
        }
    }
    (!next.is_empty()).then_some(next)
}

pub fn apply_window_changes<'a>(
    selected: &[String],
    changes: impl IntoIterator<Item = (&'a str, bool)>,
) -> Vec<String> {
    let mut next = selected.to_vec();
    for (key, on) in changes {
        if on {
            if !next.iter().any(|value| value == key) {
                next.push(key.into());
            }
        } else if next.len() > 1 {
            next.retain(|value| value != key);
        }
    }
    next
}

#[cfg(test)]
#[path = "../tests/unit/selection/tests.rs"]
mod tests;
