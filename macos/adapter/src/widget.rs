//! Exposes shared widget selection and publication policy to Swift.

use quotaline_setup::action::Availability;
use quotaline_status_bar::WidgetContent;
use quotaline_status_bar::WidgetKey;
use quotaline_status_bar::WidgetProjection;
use quotaline_status_bar::WidgetProvider;
use quotaline_status_bar::WidgetWindow;

use crate::provider_record::ProviderRecord;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct WidgetPreviousRecord {
    pub content: WidgetContent,
    pub updated_at: f64,
}

#[uniffi::export]
pub fn project_widget(
    providers: Vec<ProviderRecord>,
    selected: Vec<WidgetKey>,
    availability: Vec<Availability>,
) -> WidgetProjection {
    quotaline_status_bar::widget_projection(
        providers.into_iter().map(widget_provider).collect(),
        &selected,
        availability.contains(&Availability::AuthenticationRequired),
    )
}

#[uniffi::export]
pub fn widget_same_content(current: WidgetContent, previous: WidgetContent) -> bool {
    quotaline_status_bar::widget_same_content(&current, &previous)
}

#[uniffi::export]
pub fn widget_should_publish(
    current: WidgetContent,
    updated_at: f64,
    previous: Option<WidgetPreviousRecord>,
    refresh_interval: f64,
) -> bool {
    quotaline_status_bar::widget_should_publish(
        &current,
        updated_at,
        previous
            .as_ref()
            .map(|previous| (&previous.content, previous.updated_at)),
        refresh_interval,
    )
}

fn widget_provider(record: ProviderRecord) -> WidgetProvider {
    WidgetProvider {
        name: quotaline_quota::name(&record.id).into(),
        windows: record
            .windows
            .into_iter()
            .map(|window| WidgetWindow {
                resets_at: window.reset_seconds(),
                id: window.id,
                label: window.label,
                used_percent: window.used,
            })
            .collect(),
        id: record.id,
    }
}

#[cfg(test)]
#[path = "../tests/unit/widget/tests.rs"]
mod tests;
