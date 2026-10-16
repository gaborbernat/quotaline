//! Checks the macOS settings boundary against shared defaults and limits.

use super::*;

#[test]
fn settings_defaults_match_the_desktop_contract() {
    assert_eq!(
        settings_defaults(),
        SettingsDefaults {
            analytics_minutes: 15,
            custom_template: "{provider} {window}\n{pct}".into(),
        }
    );
}

#[test]
fn provider_preferences_bridge_preserves_new_provider_migration() {
    let preferences = migrated_provider_preferences(
        Some(vec!["claude".into()]),
        Some(vec!["claude".into()]),
        None,
        Some(vec!["claude".into()]),
    );
    assert_eq!(preferences.added.len(), 5);
    assert_eq!(preferences.enabled.len(), 6);
    assert_eq!(preferences.configured.len(), 6);
    assert_eq!(preferences.order.len(), 6);
    assert_eq!(preferences.order[0], "claude");
}

#[test]
fn provider_override_bridge_uses_shared_state_transition() {
    assert_eq!(
        provider_override(vec!["codex".into()], vec![], "claude".into(), true),
        Some(ProviderOverrideRecord {
            enabled: vec!["claude".into(), "codex".into()],
            configured: vec!["claude".into()],
        })
    );
    assert_eq!(provider_override(vec![], vec![], "invalid".into(), false), None);
}

#[test]
fn polling_interval_bridge_uses_the_provider_limits() {
    assert_eq!(settings_polling_seconds("claude".into(), 5), Some(120));
    assert_eq!(settings_polling_seconds("invalid".into(), 5), None);
}
