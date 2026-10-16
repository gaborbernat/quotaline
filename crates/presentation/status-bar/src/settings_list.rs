//! Selects and orders the quota windows shown in provider settings.

use caseless::Caseless;
use std::collections::{BTreeMap, HashMap, HashSet};
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

use super::{
    LabelWindow, OrderDraft, Provider, Settings, available_window_selection, derived_labels_for_windows,
    label_override, resolved_labels_for_windows, selected_window_keys,
};

#[derive(Clone, Debug)]
pub struct SettingsListWindow {
    pub provider: String,
    pub id: String,
    pub label: String,
    pub scope: Option<String>,
    pub used: f64,
    pub fetched_at: f64,
    pub last_used_at: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct SettingsListRequest {
    pub windows: Vec<SettingsListWindow>,
    pub selected: Vec<String>,
    pub overrides: BTreeMap<String, String>,
    pub provider_order: Vec<String>,
    pub model_order: Vec<String>,
    pub query: String,
    pub hide_unused: bool,
    pub revealed: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SettingsListRow {
    pub key: String,
    pub position: usize,
    pub default_label: String,
    pub label: String,
    pub selected: bool,
    pub can_toggle: bool,
    pub overridden: bool,
    pub last_used_at: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SettingsListGroup {
    pub provider: String,
    pub rows: Vec<SettingsListRow>,
    pub selected_count: usize,
    pub total_count: usize,
    pub can_toggle_all: bool,
}

pub fn settings_list_for_providers(
    settings: &Settings,
    providers: &BTreeMap<String, Provider>,
    activity: &BTreeMap<String, i64>,
    query: &str,
    revealed: Option<&str>,
    hide_unused: bool,
) -> Vec<SettingsListGroup> {
    let windows = providers
        .iter()
        .flat_map(|(id, provider)| {
            provider.windows.iter().map(move |window| SettingsListWindow {
                provider: id.clone(),
                id: window.id.clone(),
                label: window.label.clone(),
                scope: window.scope.clone(),
                used: window.used,
                fetched_at: provider.updated.unwrap_or(0) as f64,
                last_used_at: activity.get(&format!("{id}:{}", window.id)).map(|at| *at as f64),
            })
        })
        .collect();
    settings_list(SettingsListRequest {
        windows,
        selected: selected_window_keys(settings, providers),
        overrides: settings.short_labels.clone(),
        provider_order: settings.provider_order.clone(),
        model_order: settings.model_order.clone(),
        query: query.into(),
        hide_unused,
        revealed: revealed.map(str::to_owned),
    })
}

pub fn settings_list(request: SettingsListRequest) -> Vec<SettingsListGroup> {
    let windows = request
        .windows
        .iter()
        .map(|window| LabelWindow {
            provider: window.provider.clone(),
            id: window.id.clone(),
            label: window.label.clone(),
            scope: window.scope.clone(),
        })
        .collect::<Vec<_>>();
    let defaults = derived_labels_for_windows(&windows);
    let labels = resolved_labels_for_windows(&windows, &request.overrides);
    let keys = request
        .windows
        .iter()
        .map(|window| format!("{}:{}", window.provider, window.id))
        .collect::<Vec<_>>();
    let available_providers = request
        .windows
        .iter()
        .map(|window| window.provider.clone())
        .collect::<Vec<_>>();
    let order = OrderDraft::new(
        &request.provider_order,
        &request.model_order,
        &available_providers,
        &keys,
    );
    let by_key = keys
        .iter()
        .zip(&request.windows)
        .map(|(key, window)| (key.as_str(), window))
        .collect::<HashMap<_, _>>();
    let selected = available_window_selection(&request.selected, &keys)
        .into_iter()
        .collect::<HashSet<_>>();
    let query = normalize(&request.query);
    order
        .providers
        .iter()
        .filter_map(|provider| {
            let provider_keys = order
                .models
                .iter()
                .filter(|key| by_key[key.as_str()].provider == *provider)
                .collect::<Vec<_>>();
            let rows = provider_keys
                .iter()
                .enumerate()
                .filter_map(|(position, key)| {
                    let window = by_key[key.as_str()];
                    let default_label = defaults.get(key.as_str())?;
                    let label = labels.get(key.as_str())?;
                    let last_used_at = window
                        .last_used_at
                        .or_else(|| (window.used > 0.0).then_some(window.fetched_at));
                    let matches = query.is_empty()
                        || [quotaline_catalog::name(provider), &window.label, &window.id, label]
                            .iter()
                            .any(|value| normalize(value).contains(&query));
                    if request.revealed.as_ref() != Some(key)
                        && ((request.hide_unused && window.used <= 0.0 && window.last_used_at.is_none()) || !matches)
                    {
                        return None;
                    }
                    Some(SettingsListRow {
                        key: (*key).clone(),
                        position,
                        default_label: default_label.clone(),
                        label: label.clone(),
                        selected: selected.contains(key.as_str()),
                        can_toggle: !selected.contains(key.as_str()) || selected.len() > 1,
                        overridden: request
                            .overrides
                            .get(key.as_str())
                            .and_then(|value| label_override(value, default_label))
                            .is_some_and(|value| value == *label),
                        last_used_at,
                    })
                })
                .collect::<Vec<_>>();
            let selected_count = provider_keys
                .iter()
                .filter(|key| selected.contains(key.as_str()))
                .count();
            (!rows.is_empty()).then(|| SettingsListGroup {
                provider: provider.clone(),
                rows,
                selected_count,
                total_count: provider_keys.len(),
                can_toggle_all: selected_count != provider_keys.len() || selected.len() != selected_count,
            })
        })
        .collect()
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .nfd()
        .filter(|character| !is_combining_mark(*character))
        .default_case_fold()
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/settings_list/tests.rs"]
mod tests;
