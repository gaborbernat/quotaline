//! Checks shared Settings behavior through UniFFI records.

use super::*;

#[test]
fn provider_state_preserves_discovery_activation_and_row_visibility() {
    let states = settings_provider_states(vec![
        SettingsDiscoveryRecord {
            id: "claude".into(),
            snapshot: true,
            analytics: false,
            credential_usable: false,
            enabled_override: Some(false),
            show_all: false,
            revealed: false,
        },
        SettingsDiscoveryRecord {
            id: "gemini".into(),
            snapshot: false,
            analytics: false,
            credential_usable: false,
            enabled_override: Some(true),
            show_all: true,
            revealed: false,
        },
    ]);
    assert_eq!(
        states,
        vec![
            SettingsProviderStateRecord {
                id: "claude".into(),
                discovered: true,
                active: false,
                visible: true,
            },
            SettingsProviderStateRecord {
                id: "gemini".into(),
                discovered: false,
                active: false,
                visible: true,
            }
        ]
    );
}

#[test]
fn claude_settings_are_visible_from_provider_facts() {
    let visible = settings_visibility(
        vec![SettingsProviderRecord {
            id: "claude".into(),
            long_window: true,
            reset_credits: true,
            local_auth: false,
        }],
        SettingsOptionsRecord {
            analytics_interval_changed: false,
            workdays_changed: false,
            token_refresh_enabled: false,
            expiring_credits_enabled: false,
            sandboxed: false,
        },
    );
    assert_eq!(
        visible,
        SettingsVisibilityRecord {
            analytics: true,
            pace: true,
            token_refresh: true,
            expiring_credits: true,
        }
    );
}
