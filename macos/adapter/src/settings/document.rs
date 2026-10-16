//! Holds the shared settings document behind the macOS preferences, which store it in UserDefaults.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};

use quotaline_settings::Settings;

#[derive(Debug, PartialEq, thiserror::Error, uniffi::Error)]
pub enum SettingsError {
    #[error("{message}")]
    Invalid { message: String },
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct SettingsRecord {
    pub providers: HashMap<String, bool>,
    pub configured_providers: Vec<String>,
    pub provider_order: Vec<String>,
    pub model_order: Vec<String>,
    pub hide_unused_models: bool,
    pub show_all_providers: bool,
    pub allow_token_refresh: bool,
    pub interval_minutes: HashMap<String, u32>,
    pub format: String,
    pub order: String,
    pub show_as: String,
    pub decimals: u8,
    pub hide_zero: bool,
    pub fit_to_space: bool,
    pub hide_identity: bool,
    pub short_labels: HashMap<String, String>,
    pub selected_windows: Vec<String>,
    pub has_custom_selection: bool,
    pub template: String,
    pub retention_days: u32,
    pub analytics_interval_minutes: u32,
    pub write_usage_json: bool,
    pub last_tab: String,
    pub history_days: u32,
    pub history_metric: String,
    pub history_rollup: String,
    pub history_utc: bool,
    pub history_stacked: bool,
    pub history_hidden_keys: Vec<String>,
    pub notifications: bool,
    pub notify_thresholds: Vec<u8>,
    pub notify_resets: bool,
    pub notify_sign_in: bool,
    pub notify_expiring_credits: bool,
    pub notify_pace: bool,
    pub notify_sound: bool,
    pub detailed_logging: bool,
    pub first_run_card_dismissed: bool,
    pub workdays: u8,
    pub demo_choice: Option<bool>,
}

impl From<&Settings> for SettingsRecord {
    fn from(settings: &Settings) -> Self {
        Self {
            providers: settings.providers.clone().into_iter().collect(),
            configured_providers: settings.configured_providers.iter().cloned().collect(),
            provider_order: settings.provider_order.clone(),
            model_order: settings.model_order.clone(),
            hide_unused_models: settings.hide_unused_models,
            show_all_providers: settings.show_all_providers,
            allow_token_refresh: settings.allow_token_refresh,
            interval_minutes: settings.interval_minutes.clone().into_iter().collect(),
            format: settings.format.clone(),
            order: settings.order.clone(),
            show_as: settings.show_as.clone(),
            decimals: settings.decimals,
            hide_zero: settings.hide_zero,
            fit_to_space: settings.fit_to_space,
            hide_identity: settings.hide_identity,
            short_labels: settings.short_labels.clone().into_iter().collect(),
            selected_windows: settings.selected_windows.clone(),
            has_custom_selection: settings.has_custom_selection == Some(true),
            template: settings.template.clone(),
            retention_days: settings.retention_days,
            analytics_interval_minutes: settings.analytics_interval_minutes,
            write_usage_json: settings.write_usage_json,
            last_tab: settings.last_tab.clone(),
            history_days: settings.history_days,
            history_metric: settings.history_metric.clone(),
            history_rollup: settings.history_rollup.clone(),
            history_utc: settings.history_utc,
            history_stacked: settings.history_stacked,
            history_hidden_keys: settings.history_hidden_keys.clone(),
            notifications: settings.notifications,
            notify_thresholds: settings.notify_thresholds.clone(),
            notify_resets: settings.notify_resets,
            notify_sign_in: settings.notify_sign_in,
            notify_expiring_credits: settings.notify_expiring_credits,
            notify_pace: settings.notify_pace,
            notify_sound: settings.notify_sound,
            detailed_logging: settings.detailed_logging,
            first_run_card_dismissed: settings.first_run_card_dismissed,
            workdays: settings.workdays,
            demo_choice: settings.demo_choice,
        }
    }
}

impl SettingsRecord {
    // Fields that only GNOME uses, such as the widget size, keep the values already in the document.
    fn edit(self, mut settings: Settings) -> Settings {
        settings.providers = self.providers.into_iter().collect();
        settings.configured_providers = self.configured_providers.into_iter().collect();
        settings.provider_order = self.provider_order;
        settings.model_order = self.model_order;
        settings.hide_unused_models = self.hide_unused_models;
        settings.show_all_providers = self.show_all_providers;
        settings.allow_token_refresh = self.allow_token_refresh;
        settings.interval_minutes = self.interval_minutes.into_iter().collect();
        settings.format = self.format;
        settings.order = self.order;
        settings.show_as = self.show_as;
        settings.decimals = self.decimals;
        settings.hide_zero = self.hide_zero;
        settings.fit_to_space = self.fit_to_space;
        settings.hide_identity = self.hide_identity;
        settings.short_labels = self.short_labels.into_iter().collect();
        settings.selected_windows = self.selected_windows;
        settings.has_custom_selection = Some(self.has_custom_selection);
        settings.template = self.template;
        settings.retention_days = self.retention_days;
        settings.analytics_interval_minutes = self.analytics_interval_minutes;
        settings.write_usage_json = self.write_usage_json;
        settings.last_tab = self.last_tab;
        settings.history_days = self.history_days;
        settings.history_metric = self.history_metric;
        settings.history_rollup = self.history_rollup;
        settings.history_utc = self.history_utc;
        settings.history_stacked = self.history_stacked;
        settings.history_hidden_keys = self.history_hidden_keys;
        settings.notifications = self.notifications;
        settings.notify_thresholds = self.notify_thresholds;
        settings.notify_resets = self.notify_resets;
        settings.notify_sign_in = self.notify_sign_in;
        settings.notify_expiring_credits = self.notify_expiring_credits;
        settings.notify_pace = self.notify_pace;
        settings.notify_sound = self.notify_sound;
        settings.detailed_logging = self.detailed_logging;
        settings.first_run_card_dismissed = self.first_run_card_dismissed;
        settings.workdays = self.workdays;
        settings.demo_choice = self.demo_choice;
        settings
    }
}

#[derive(uniffi::Object)]
pub struct RustSettings(Mutex<Settings>);

impl RustSettings {
    fn settings(&self) -> std::sync::MutexGuard<'_, Settings> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[uniffi::export]
impl RustSettings {
    /// Opens a stored document; an unreadable one falls back to the defaults, as GNOME does.
    #[uniffi::constructor]
    pub fn new(document: Option<Vec<u8>>) -> Arc<Self> {
        let mut settings = document
            .and_then(|document| serde_json::from_slice::<Settings>(&document).ok())
            .unwrap_or_default();
        settings.normalize();
        Arc::new(Self(Mutex::new(settings)))
    }

    pub fn record(&self) -> SettingsRecord {
        SettingsRecord::from(&*self.settings())
    }

    pub fn update(&self, record: SettingsRecord) -> Result<SettingsRecord, SettingsError> {
        let mut settings = self.settings();
        let changed = record.edit(settings.clone());
        settings
            .replace(changed)
            .map_err(|message| SettingsError::Invalid { message })?;
        Ok(SettingsRecord::from(&*settings))
    }

    pub fn document(&self) -> Vec<u8> {
        serde_json::to_vec(&self.settings().stored()).expect("settings contain only JSON values")
    }

    /// Restores the defaults but keeps the open tab, so resetting does not move the popover.
    pub fn reset(&self) -> SettingsRecord {
        let mut settings = self.settings();
        let last_tab = settings.last_tab.clone();
        *settings = Settings {
            last_tab,
            ..Settings::default()
        };
        settings.normalize();
        SettingsRecord::from(&*settings)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/settings/document/tests.rs"]
mod tests;
