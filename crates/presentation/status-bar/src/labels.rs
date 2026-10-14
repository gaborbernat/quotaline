//! Resolves compact window labels from the same inputs on every platform.

use std::collections::BTreeMap;
use unicode_segmentation::UnicodeSegmentation;

use super::status::ShortLabelPreview;
use super::status::{
    default_label, preview_from_defaults, resolve_short_labels, short_label_override, unique_short_labels,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelWindow {
    pub provider: String,
    pub id: String,
    pub label: String,
    pub scope: Option<String>,
}

pub const SHORT_LABEL_LIMIT: usize = 6;

pub fn short_label_draft(value: &str) -> String {
    value.graphemes(true).take(SHORT_LABEL_LIMIT).collect()
}

pub fn label_override(value: &str, default: &str) -> Option<String> {
    short_label_override(value, default)
}

pub fn default_short_label(provider: &str, id: &str, label: &str, scope: Option<&str>) -> Option<String> {
    let code = quotaline_catalog::definition(provider)?.code;
    Some(short_label_draft(&default_label(code, id, label, scope)))
}

pub fn derived_labels_for_windows(windows: &[LabelWindow]) -> BTreeMap<String, String> {
    let bases = windows
        .iter()
        .filter_map(|window| {
            let code = quotaline_catalog::definition(&window.provider)?.code;
            Some((
                format!("{}:{}", window.provider, window.id),
                default_label(code, &window.id, &window.label, window.scope.as_deref()),
            ))
        })
        .collect();
    unique_short_labels(bases)
}

pub fn resolved_labels_for_windows(
    windows: &[LabelWindow],
    overrides: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    resolve_short_labels(&derived_labels_for_windows(windows), overrides)
}

pub fn preview_label_for_windows(
    windows: &[LabelWindow],
    overrides: &BTreeMap<String, String>,
    key: &str,
    draft: &str,
) -> Option<ShortLabelPreview> {
    preview_from_defaults(&derived_labels_for_windows(windows), overrides, key, draft)
}

pub fn valid_overrides_for_windows(
    windows: &[LabelWindow],
    persisted: &BTreeMap<String, String>,
    drafts: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let defaults = derived_labels_for_windows(windows);
    let mut result = persisted.clone();
    for (key, draft) in drafts {
        if let Some(preview) = preview_from_defaults(&defaults, &result, key, draft)
            && preview.conflict_key.is_none()
        {
            if let Some(value) = preview.override_value {
                result.insert(key.clone(), value);
            } else {
                result.remove(key);
            }
        }
    }
    result
}

#[cfg(test)]
#[path = "../tests/unit/labels/tests.rs"]
mod tests;
