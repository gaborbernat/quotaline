use super::*;

fn effects(previous: &Settings, changed: &Settings, key: &str) -> SettingsUpdateEffects {
    SettingsUpdateEffects::between(previous, changed, key)
}

#[test]
fn retention_reduction_prunes_only_on_retention_edit() {
    let previous = Settings::default();
    let changed = Settings {
        retention_days: previous.retention_days - 1,
        ..previous.clone()
    };
    assert_eq!(
        effects(&previous, &changed, "retention_days").prune_days,
        Some(changed.retention_days)
    );
    assert_eq!(effects(&previous, &changed, "format").prune_days, None);
    assert_eq!(effects(&changed, &previous, "retention_days").prune_days, None);
}

#[test]
fn token_refresh_and_demo_request_full_refresh() {
    let previous = Settings::default();
    let changed = Settings {
        allow_token_refresh: true,
        demo: true,
        ..previous.clone()
    };
    assert!(effects(&previous, &changed, "allow_token_refresh").refresh_all);
    assert!(effects(&previous, &changed, "demo").refresh_all);
    assert!(effects(&previous, &changed, "demo").reset_demo);
    assert!(!effects(&changed, &previous, "allow_token_refresh").refresh_all);
    assert!(!effects(&previous, &changed, "format").refresh_all);
}

#[test]
fn provider_changes_refresh_only_newly_enabled_provider() {
    let mut previous = Settings::default();
    previous.providers.insert("claude".into(), false);
    let changed = Settings::default();
    assert_eq!(effects(&previous, &changed, "providers").newly_enabled, ["claude"]);
    assert_eq!(effects(&changed, &changed, "provider:codex").newly_enabled, ["codex"]);
    assert!(effects(&changed, &changed, "format").newly_enabled.is_empty());
    assert!(effects(&changed, &previous, "provider:claude").newly_enabled.is_empty());
}

#[test]
fn history_setting_effects_are_separate() {
    let settings = Settings::default();
    assert!(effects(&settings, &settings, "history_hidden_keys").sync_history_visibility);
    for key in ["history_days", "history_metric", "history_rollup", "history_utc"] {
        assert!(effects(&settings, &settings, key).reset_history_session, "{key}");
    }
    assert!(!effects(&settings, &settings, "format").reset_history_session);
}
