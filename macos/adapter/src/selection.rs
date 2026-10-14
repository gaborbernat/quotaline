//! Bridges shared quota selection into macOS status and settings models.

use quotaline_status_bar::apply_window_changes;
use quotaline_status_bar::default_window_ids;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SelectionWindow {
    pub id: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SelectionProvider {
    pub id: String,
    pub windows: Vec<SelectionWindow>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SelectedWindow {
    pub provider: String,
    pub window_id: String,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SelectionChangeRecord {
    pub key: String,
    pub on: bool,
}

#[uniffi::export]
pub fn apply_window_selection(selected: Vec<String>, changes: Vec<SelectionChangeRecord>) -> Vec<String> {
    apply_window_changes(&selected, changes.iter().map(|change| (change.key.as_str(), change.on)))
}

#[uniffi::export]
pub fn default_window_selection(providers: Vec<SelectionProvider>) -> Vec<SelectedWindow> {
    providers
        .into_iter()
        .flat_map(|provider| {
            default_window_ids(
                provider
                    .windows
                    .iter()
                    .map(|window| (window.id.as_str(), window.is_active)),
            )
            .into_iter()
            .map(|window_id| SelectedWindow {
                provider: provider.id.clone(),
                window_id: window_id.into(),
            })
            .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/selection/tests.rs"]
mod tests;
