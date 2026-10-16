//! Maps macOS window labels to the shared label policy.

use quotaline_status_bar::LabelWindow;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct LabelWindowInput {
    pub provider: String,
    pub id: String,
    pub label: String,
    pub scope: Option<String>,
}

impl From<LabelWindowInput> for LabelWindow {
    fn from(value: LabelWindowInput) -> Self {
        Self {
            provider: value.provider,
            id: value.id,
            label: value.label,
            scope: value.scope,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct LabelPreview {
    pub override_value: Option<String>,
    pub conflict_key: Option<String>,
}

#[uniffi::export]
pub fn short_label_limit() -> u32 {
    quotaline_status_bar::SHORT_LABEL_LIMIT as u32
}

#[uniffi::export]
pub fn short_label_draft(value: String) -> String {
    quotaline_status_bar::short_label_draft(&value)
}

#[uniffi::export]
pub fn short_label_override(value: String, default_label: String) -> Option<String> {
    quotaline_status_bar::label_override(&value, &default_label)
}

#[uniffi::export]
pub fn default_window_short_label(
    provider: String,
    id: String,
    label: String,
    scope: Option<String>,
) -> Option<String> {
    quotaline_status_bar::default_short_label(&provider, &id, &label, scope.as_deref())
}

fn core_windows(windows: Vec<LabelWindowInput>) -> Vec<LabelWindow> {
    windows.into_iter().map(Into::into).collect()
}

#[uniffi::export]
pub fn derived_window_labels(windows: Vec<LabelWindowInput>) -> HashMap<String, String> {
    quotaline_status_bar::derived_labels_for_windows(&core_windows(windows))
        .into_iter()
        .collect()
}

#[uniffi::export]
pub fn resolved_window_labels(
    windows: Vec<LabelWindowInput>,
    overrides: HashMap<String, String>,
) -> HashMap<String, String> {
    quotaline_status_bar::resolved_labels_for_windows(&core_windows(windows), &overrides.into_iter().collect())
        .into_iter()
        .collect()
}

#[uniffi::export]
pub fn preview_window_label(
    windows: Vec<LabelWindowInput>,
    overrides: HashMap<String, String>,
    key: String,
    draft: String,
) -> Option<LabelPreview> {
    let overrides: BTreeMap<_, _> = overrides.into_iter().collect();
    quotaline_status_bar::preview_label_for_windows(&core_windows(windows), &overrides, &key, &draft).map(|preview| {
        LabelPreview {
            override_value: preview.override_value,
            conflict_key: preview.conflict_key,
        }
    })
}

#[uniffi::export]
pub fn valid_window_label_overrides(
    windows: Vec<LabelWindowInput>,
    persisted: HashMap<String, String>,
    drafts: HashMap<String, String>,
) -> HashMap<String, String> {
    quotaline_status_bar::valid_overrides_for_windows(
        &core_windows(windows),
        &persisted.into_iter().collect(),
        &drafts.into_iter().collect(),
    )
    .into_iter()
    .collect()
}

#[cfg(test)]
#[path = "../tests/unit/labels/tests.rs"]
mod tests;
