//! Projects provider windows into status entries with shared labels and restrictions.

use std::collections::BTreeMap;

use super::{
    LabelWindow, RestrictionWindow, projected_restriction_reset, resolved_labels_for_windows, restrictive_indices,
};

pub struct StatusWindow {
    pub id: String,
    pub label: String,
    pub group: String,
    pub used: f64,
    pub active: bool,
    pub reset_at: Option<f64>,
    pub scope: Option<String>,
}

pub struct StatusProvider {
    pub id: String,
    pub windows: Vec<StatusWindow>,
}

pub struct StatusSelectedWindow {
    pub provider: String,
    pub window_id: String,
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct StatusRestriction {
    pub label: String,
    pub reset_at: Option<f64>,
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct StatusProjectedEntry {
    pub provider: String,
    pub provider_code: &'static str,
    pub provider_name: &'static str,
    pub window_id: String,
    pub window_label: String,
    pub window_scope: Option<String>,
    pub short_label: String,
    pub used: f64,
    pub reset_at: Option<f64>,
    pub active: bool,
    pub restricted: bool,
    pub restrictions: Vec<StatusRestriction>,
}

pub fn status_entries(
    providers: &[StatusProvider],
    selected: &[StatusSelectedWindow],
    overrides: &BTreeMap<String, String>,
    now: f64,
) -> Vec<StatusProjectedEntry> {
    let labels = resolved_labels_for_windows(
        &providers
            .iter()
            .flat_map(|provider| {
                provider.windows.iter().map(|window| LabelWindow {
                    provider: provider.id.clone(),
                    id: window.id.clone(),
                    label: window.label.clone(),
                    scope: window.scope.clone(),
                })
            })
            .collect::<Vec<_>>(),
        overrides,
    );
    selected
        .iter()
        .filter_map(|key| {
            let provider = providers.iter().find(|provider| provider.id == key.provider)?;
            let window = provider.windows.iter().find(|window| window.id == key.window_id)?;
            let short_label = labels.get(&format!("{}:{}", key.provider, key.window_id))?.clone();
            let definition = quotaline_catalog::definition(&provider.id)?;
            let candidates = provider.windows.iter().map(restriction_window).collect::<Vec<_>>();
            let indices = restrictive_indices(&provider.id, restriction_window(window), &candidates, now);
            let reset_at = indices
                .as_ref()
                .map(|indices| projected_restriction_reset(&candidates, indices))
                .unwrap_or(window.reset_at);
            let restrictions = indices
                .map(|indices| {
                    indices
                        .into_iter()
                        .map(|index| StatusRestriction {
                            label: candidates[index].label.into(),
                            reset_at: candidates[index].reset_at,
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let used = if !restrictions.is_empty() { 100.0 } else { window.used };
            let restricted = used >= 100.0
                && (window.active || !restrictions.is_empty())
                && reset_at.is_none_or(|reset| reset > now);
            Some(StatusProjectedEntry {
                provider: key.provider.clone(),
                provider_code: definition.code,
                provider_name: definition.name,
                window_id: key.window_id.clone(),
                window_label: window.label.clone(),
                window_scope: window.scope.clone(),
                short_label,
                used,
                reset_at,
                active: window.active,
                restricted,
                restrictions,
            })
        })
        .collect()
}

fn restriction_window(window: &StatusWindow) -> RestrictionWindow<'_> {
    RestrictionWindow {
        id: &window.id,
        label: &window.label,
        group: &window.group,
        used: window.used,
        active: window.active,
        reset_at: window.reset_at,
    }
}

#[cfg(test)]
#[path = "../tests/unit/status_projection/tests.rs"]
mod tests;
