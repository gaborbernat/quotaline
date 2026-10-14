//! Bridges projected quota data to the macOS status renderer.

use std::collections::HashMap;

use crate::provider_record::ProviderRecord;
use quotaline_status_bar::StatusProvider as CoreProvider;
use quotaline_status_bar::StatusSelectedWindow as CoreSelectedWindow;
use quotaline_status_bar::StatusSelection;
use quotaline_status_bar::StatusSelectionEntry;
use quotaline_status_bar::StatusSelectionMode;
use quotaline_status_bar::StatusWindow as CoreWindow;
use quotaline_status_bar::select_status;
use quotaline_status_bar::status_entries;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct StatusSelectedWindowRecord {
    pub provider: String,
    pub window_id: String,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct StatusRestrictionRecord {
    pub label: String,
    pub reset_at: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct StatusProjectedEntryRecord {
    pub provider: String,
    pub provider_code: String,
    pub provider_name: String,
    pub window_id: String,
    pub window_label: String,
    pub window_scope: Option<String>,
    pub short_label: String,
    pub used: f64,
    pub reset_at: Option<f64>,
    pub active: bool,
    pub restricted: bool,
    pub restrictions: Vec<StatusRestrictionRecord>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct StatusSelectionGroupRecord {
    pub provider: String,
    pub indices: Vec<u32>,
    pub used: f64,
    pub restricted: bool,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct StatusSelectionRecord {
    pub indices: Vec<u32>,
    pub groups: Vec<StatusSelectionGroupRecord>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct StatusProjectionRecord {
    pub entries: Vec<StatusProjectedEntryRecord>,
    pub all_selection: StatusSelectionRecord,
    pub worst_selection: StatusSelectionRecord,
    pub mini_selection: StatusSelectionRecord,
}

#[uniffi::export]
pub fn project_status(
    providers: Vec<ProviderRecord>,
    selected: Vec<StatusSelectedWindowRecord>,
    overrides: HashMap<String, String>,
    now: f64,
    hide_zero: bool,
    usage_order: bool,
) -> StatusProjectionRecord {
    let source = providers
        .into_iter()
        .map(|provider| CoreProvider {
            id: provider.id,
            windows: provider
                .windows
                .into_iter()
                .map(|window| CoreWindow {
                    reset_at: window.reset_seconds(),
                    id: window.id,
                    label: window.label,
                    group: window.group,
                    used: window.used,
                    active: window.active,
                    scope: window.scope,
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let keys = selected
        .into_iter()
        .map(|key| CoreSelectedWindow {
            provider: key.provider,
            window_id: key.window_id,
        })
        .collect::<Vec<_>>();
    let projected = status_entries(&source, &keys, &overrides.into_iter().collect(), now);
    let selection_entries = projected
        .iter()
        .map(|entry| StatusSelectionEntry {
            provider: &entry.provider,
            used: entry.used,
            restricted: !entry.restrictions.is_empty(),
        })
        .collect::<Vec<_>>();
    let all_selection = selection_record(select_status(
        &selection_entries,
        StatusSelectionMode::All,
        hide_zero,
        usage_order,
    ));
    let worst_selection = selection_record(select_status(
        &selection_entries,
        StatusSelectionMode::WorstPerProvider,
        hide_zero,
        usage_order,
    ));
    let mini_selection = selection_record(select_status(
        &selection_entries,
        StatusSelectionMode::MiniBars,
        hide_zero,
        usage_order,
    ));
    let entries = projected
        .into_iter()
        .map(|entry| StatusProjectedEntryRecord {
            provider: entry.provider,
            provider_code: entry.provider_code.into(),
            provider_name: entry.provider_name.into(),
            window_id: entry.window_id,
            window_label: entry.window_label,
            window_scope: entry.window_scope,
            short_label: entry.short_label,
            used: entry.used,
            reset_at: entry.reset_at,
            active: entry.active,
            restricted: entry.restricted,
            restrictions: entry
                .restrictions
                .into_iter()
                .map(|restriction| StatusRestrictionRecord {
                    label: restriction.label,
                    reset_at: restriction.reset_at,
                })
                .collect(),
        })
        .collect();
    StatusProjectionRecord {
        entries,
        all_selection,
        worst_selection,
        mini_selection,
    }
}

fn selection_record(selected: StatusSelection) -> StatusSelectionRecord {
    StatusSelectionRecord {
        indices: selected.indices.into_iter().map(|index| index as u32).collect(),
        groups: selected
            .groups
            .into_iter()
            .map(|group| StatusSelectionGroupRecord {
                provider: group.provider,
                indices: group.indices.into_iter().map(|index| index as u32).collect(),
                used: group.used,
                restricted: group.restricted,
            })
            .collect(),
    }
}

#[cfg(test)]
#[path = "../tests/unit/status_layout/tests.rs"]
mod tests;
