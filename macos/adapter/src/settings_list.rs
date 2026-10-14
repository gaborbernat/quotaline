//! Exposes the shared settings window list to Swift.

use quotaline_status_bar::SettingsListRequest;
use quotaline_status_bar::SettingsListWindow;
use std::collections::BTreeMap;

use crate::provider_record::ProviderReadingRecord;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SettingsListRequestRecord {
    pub readings: Vec<ProviderReadingRecord>,
    /// When each `provider:window` key last changed in history.
    pub last_used_at: std::collections::HashMap<String, f64>,
    pub selected: Vec<String>,
    pub overrides: std::collections::HashMap<String, String>,
    pub provider_order: Vec<String>,
    pub model_order: Vec<String>,
    pub query: String,
    pub hide_unused: bool,
    pub revealed: Option<String>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SettingsListRowRecord {
    pub key: String,
    pub position: u32,
    pub default_label: String,
    pub label: String,
    pub selected: bool,
    pub can_toggle: bool,
    pub overridden: bool,
    pub last_used_at: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SettingsListGroupRecord {
    pub provider: String,
    pub rows: Vec<SettingsListRowRecord>,
    pub selected_count: u64,
    pub total_count: u64,
    pub can_toggle_all: bool,
}

#[uniffi::export]
pub fn settings_model_groups(request: SettingsListRequestRecord) -> Vec<SettingsListGroupRecord> {
    quotaline_status_bar::settings_list(SettingsListRequest {
        windows: request
            .readings
            .into_iter()
            .flat_map(|reading| {
                let provider = reading.provider.id;
                let fetched_at = reading.fetched_at;
                let last_used_at = &request.last_used_at;
                reading
                    .provider
                    .windows
                    .into_iter()
                    .map(move |window| SettingsListWindow {
                        last_used_at: last_used_at.get(&format!("{provider}:{}", window.id)).copied(),
                        provider: provider.clone(),
                        id: window.id,
                        label: window.label,
                        scope: window.scope,
                        used: window.used,
                        fetched_at,
                    })
            })
            .collect(),
        selected: request.selected,
        overrides: request.overrides.into_iter().collect::<BTreeMap<_, _>>(),
        provider_order: request.provider_order,
        model_order: request.model_order,
        query: request.query,
        hide_unused: request.hide_unused,
        revealed: request.revealed,
    })
    .into_iter()
    .map(|group| SettingsListGroupRecord {
        provider: group.provider,
        rows: group
            .rows
            .into_iter()
            .map(|row| SettingsListRowRecord {
                key: row.key,
                position: row.position as u32,
                default_label: row.default_label,
                label: row.label,
                selected: row.selected,
                can_toggle: row.can_toggle,
                overridden: row.overridden,
                last_used_at: row.last_used_at,
            })
            .collect(),
        selected_count: group.selected_count as u64,
        total_count: group.total_count as u64,
        can_toggle_all: group.can_toggle_all,
    })
    .collect()
}

#[cfg(test)]
#[path = "../tests/unit/settings_list/tests.rs"]
mod tests;
