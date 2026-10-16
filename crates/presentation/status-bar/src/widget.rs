//! Selects widget rows and decides when changed or aged content needs publishing.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct WidgetWindow {
    pub id: String,
    pub label: String,
    pub used_percent: f64,
    pub resets_at: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WidgetProvider {
    pub id: String,
    pub name: String,
    pub windows: Vec<WidgetWindow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WidgetKey {
    pub provider: String,
    pub window_id: String,
}

// The macOS app already has a Swift type with this name.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[cfg_attr(feature = "uniffi", uniffi(name = "WidgetRowRecord"))]
pub struct WidgetRow {
    pub key: WidgetKey,
    pub provider_name: String,
    pub label: String,
    pub used_percent: f64,
    pub resets_at: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WidgetContent {
    pub rows: Vec<WidgetRow>,
    pub attention: bool,
    pub display: String,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WidgetProjection {
    pub rows: Vec<WidgetRow>,
    pub attention: bool,
}

pub fn widget_projection(
    providers: Vec<WidgetProvider>,
    selected: &[WidgetKey],
    authentication_required: bool,
) -> WidgetProjection {
    let providers: BTreeMap<_, _> = providers
        .into_iter()
        .map(|provider| (provider.id.clone(), provider))
        .collect();
    let rows = selected
        .iter()
        .filter_map(|key| {
            let provider = providers.get(&key.provider)?;
            let window = provider.windows.iter().find(|window| window.id == key.window_id)?;
            Some(WidgetRow {
                key: key.clone(),
                provider_name: provider.name.clone(),
                label: window.label.clone(),
                used_percent: window.used_percent,
                resets_at: window.resets_at,
            })
        })
        .collect();
    WidgetProjection {
        rows,
        attention: authentication_required,
    }
}

pub fn widget_same_content(current: &WidgetContent, previous: &WidgetContent) -> bool {
    current == previous
}

pub fn widget_should_publish(
    current: &WidgetContent,
    updated_at: f64,
    previous: Option<(&WidgetContent, f64)>,
    refresh_interval: f64,
) -> bool {
    previous.is_none_or(|(content, previous_at)| {
        !widget_same_content(current, content) || updated_at - previous_at >= refresh_interval
    })
}

#[cfg(test)]
#[path = "../tests/unit/widget/tests.rs"]
mod tests;
