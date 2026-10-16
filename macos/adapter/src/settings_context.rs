//! Exposes shared Settings visibility rules to Swift.

use quotaline_settings::context as settings_context;
use quotaline_settings::context::Options;
use quotaline_settings::context::Provider;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SettingsDiscoveryRecord {
    pub id: String,
    pub snapshot: bool,
    pub analytics: bool,
    pub credential_usable: bool,
    pub enabled_override: Option<bool>,
    pub show_all: bool,
    pub revealed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SettingsProviderStateRecord {
    pub id: String,
    pub discovered: bool,
    pub active: bool,
    pub visible: bool,
}

#[uniffi::export]
pub fn settings_provider_states(providers: Vec<SettingsDiscoveryRecord>) -> Vec<SettingsProviderStateRecord> {
    providers
        .into_iter()
        .map(|provider| {
            let facts = settings_context::Discovery {
                snapshot: provider.snapshot,
                analytics: provider.analytics,
                credential_usable: provider.credential_usable,
            };
            SettingsProviderStateRecord {
                id: provider.id,
                discovered: settings_context::discovered(facts),
                active: settings_context::active(facts, provider.enabled_override),
                visible: settings_context::visible(facts, provider.show_all, provider.revealed),
            }
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SettingsProviderRecord {
    pub id: String,
    pub long_window: bool,
    pub reset_credits: bool,
    pub local_auth: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SettingsOptionsRecord {
    pub analytics_interval_changed: bool,
    pub workdays_changed: bool,
    pub token_refresh_enabled: bool,
    pub expiring_credits_enabled: bool,
    pub sandboxed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SettingsVisibilityRecord {
    pub analytics: bool,
    pub pace: bool,
    pub token_refresh: bool,
    pub expiring_credits: bool,
}

#[uniffi::export]
pub fn settings_visibility(
    providers: Vec<SettingsProviderRecord>,
    options: SettingsOptionsRecord,
) -> SettingsVisibilityRecord {
    let providers = providers
        .into_iter()
        .map(|provider| Provider {
            id: provider.id,
            long_window: provider.long_window,
            reset_credits: provider.reset_credits,
            local_auth: provider.local_auth,
        })
        .collect::<Vec<_>>();
    let visible = settings_context::visibility(
        &providers,
        Options {
            analytics_interval_changed: options.analytics_interval_changed,
            workdays_changed: options.workdays_changed,
            token_refresh_enabled: options.token_refresh_enabled,
            expiring_credits_enabled: options.expiring_credits_enabled,
            sandboxed: options.sandboxed,
        },
    );
    SettingsVisibilityRecord {
        analytics: visible.analytics,
        pace: visible.pace,
        token_refresh: visible.token_refresh,
        expiring_credits: visible.expiring_credits,
    }
}

#[cfg(test)]
#[path = "../tests/unit/settings_context/tests.rs"]
mod tests;
