//! Selects, labels, orders, and lays out the quota windows the status bar and widgets show.

use quotaline_catalog::{CODES, IDS};
use quotaline_quota::restriction::{RestrictionWindow, projected_restriction_reset, restrictive_indices};
use quotaline_quota::*;
use quotaline_settings::*;
use quotaline_text::text::normalized_label;
use std::collections::BTreeMap;
use unicode_segmentation::UnicodeSegmentation;

#[cfg(test)]
use quotaline_demo::*;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

mod labels;
mod order;
mod selection;
mod settings_list;
mod status;
mod status_projection;
mod status_selection;
mod template;
mod widget;

pub use labels::{
    LabelWindow, SHORT_LABEL_LIMIT, default_short_label, derived_labels_for_windows, label_override,
    preview_label_for_windows, resolved_labels_for_windows, short_label_draft, valid_overrides_for_windows,
};
pub use order::OrderDraft;
pub use selection::{
    apply_window_changes, available_selected_window_keys, available_window_selection, default_window_ids,
    history_window_selection, selected_window_keys, toggle_provider_windows, toggle_window,
};
pub use settings_list::{
    SettingsListGroup, SettingsListRequest, SettingsListRow, SettingsListWindow, settings_list,
    settings_list_for_providers,
};
pub use status::{
    ShortLabelPreview, default_selected_windows, derived_short_labels, preview_short_label, short_label_conflicts,
    short_label_maps,
};
pub use status_projection::{
    StatusProjectedEntry, StatusProvider, StatusRestriction, StatusSelectedWindow, StatusWindow, status_entries,
};
pub use status_selection::{
    StatusSelection, StatusSelectionEntry, StatusSelectionGroup, StatusSelectionMode, select_status,
};
pub use template::{TemplateToken, parse_template};
pub use widget::{
    WidgetContent, WidgetKey, WidgetProjection, WidgetProvider, WidgetRow, WidgetWindow, widget_projection,
    widget_same_content, widget_should_publish,
};

#[cfg(test)]
#[path = "../tests/unit/model_labels/tests.rs"]
mod model_labels_tests;
#[cfg(test)]
#[path = "../tests/unit/model_legacy/tests.rs"]
mod model_legacy_tests;
#[cfg(test)]
#[path = "../tests/unit/model_selection/tests.rs"]
mod model_selection_tests;
