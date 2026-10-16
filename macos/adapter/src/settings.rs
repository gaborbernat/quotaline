//! Exposes shared settings defaults, limits, and the stored settings document to the macOS preferences view.

mod document;
pub use document::{RustSettings, SettingsError, SettingsRecord};

use quotaline_settings::DEFAULT_ANALYTICS_INTERVAL_MINUTES;
use quotaline_settings::DEFAULT_STATUS_TEMPLATE;
use quotaline_settings::ProviderOverride;
use quotaline_settings::ProviderPreferences;
use quotaline_settings::migrate_provider_preferences;
use quotaline_settings::normalized_polling_seconds;
use quotaline_settings::set_provider_override;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProviderPreferencesRecord {
    pub added: Vec<String>,
    pub enabled: Vec<String>,
    pub configured: Vec<String>,
    pub order: Vec<String>,
}

impl From<ProviderPreferences> for ProviderPreferencesRecord {
    fn from(preferences: ProviderPreferences) -> Self {
        Self {
            added: preferences.added,
            enabled: preferences.enabled,
            configured: preferences.configured,
            order: preferences.order,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProviderOverrideRecord {
    pub enabled: Vec<String>,
    pub configured: Vec<String>,
}

impl From<ProviderOverride> for ProviderOverrideRecord {
    fn from(value: ProviderOverride) -> Self {
        Self {
            enabled: value.enabled,
            configured: value.configured,
        }
    }
}

#[uniffi::export]
pub fn provider_override(
    enabled: Vec<String>,
    configured: Vec<String>,
    provider: String,
    active: bool,
) -> Option<ProviderOverrideRecord> {
    set_provider_override(&enabled, &configured, &provider, active).map(Into::into)
}

#[uniffi::export]
pub fn migrated_provider_preferences(
    known: Option<Vec<String>>,
    enabled: Option<Vec<String>>,
    configured: Option<Vec<String>>,
    order: Option<Vec<String>>,
) -> ProviderPreferencesRecord {
    migrate_provider_preferences(
        known.as_deref(),
        enabled.as_deref(),
        configured.as_deref(),
        order.as_deref(),
    )
    .into()
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SettingsDefaults {
    pub analytics_minutes: u32,
    pub custom_template: String,
}

#[uniffi::export]
pub fn settings_defaults() -> SettingsDefaults {
    SettingsDefaults {
        analytics_minutes: DEFAULT_ANALYTICS_INTERVAL_MINUTES,
        custom_template: DEFAULT_STATUS_TEMPLATE.replace("\\n", "\n"),
    }
}

#[uniffi::export]
pub fn settings_polling_seconds(provider: String, requested: i64) -> Option<u32> {
    normalized_polling_seconds(&provider, requested)
}

#[cfg(test)]
#[path = "../tests/unit/settings/tests.rs"]
mod tests;
