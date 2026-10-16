use super::*;

#[test]
fn provider_override_records_explicit_enable_and_disable() {
    let enabled = vec!["codex".into()];
    let configured = vec!["codex".into()];
    let chosen = set_provider_override(&enabled, &configured, "claude", true).unwrap();
    assert_eq!(chosen.enabled, ["claude", "codex"]);
    assert_eq!(chosen.configured, ["claude", "codex"]);
    let disabled = set_provider_override(&chosen.enabled, &chosen.configured, "claude", false).unwrap();
    assert_eq!(disabled.enabled, ["codex"]);
    assert_eq!(disabled.configured, ["claude", "codex"]);
}

#[test]
fn provider_override_rejects_unknown_provider() {
    assert_eq!(set_provider_override(&[], &[], "unknown", true), None);
}

#[test]
fn provider_interval_step_changes_only_the_selected_provider() {
    let intervals = BTreeMap::from([("claude".into(), 3), ("codex".into(), 7)]);
    assert_eq!(
        step_provider_interval(&intervals, "claude", -1),
        Some(BTreeMap::from([("claude".into(), 2), ("codex".into(), 7)]))
    );
    assert_eq!(step_provider_interval(&intervals, "unknown", 1), None);
}

#[test]
fn provider_interval_step_uses_shared_bounds_and_defaults() {
    assert_eq!(
        step_provider_interval(&BTreeMap::new(), "claude", -100).unwrap()["claude"],
        2
    );
    assert_eq!(
        step_provider_interval(&BTreeMap::from([("claude".into(), 30)]), "claude", 1).unwrap()["claude"],
        30
    );
}

#[test]
fn preferences_enable_new_providers_without_overriding_existing_choices() {
    let known = vec!["claude".into(), "codex".into()];
    let enabled = vec!["codex".into()];
    let ordered = vec!["codex".into(), "claude".into()];
    let migrated = migrate_provider_preferences(Some(&known), Some(&enabled), None, Some(&ordered));
    assert_eq!(migrated.added, ["gemini", "antigravity", "cursor", "copilot"]);
    assert_eq!(
        migrated.enabled,
        ["codex", "gemini", "antigravity", "cursor", "copilot"]
    );
    assert_eq!(migrated.configured, IDS);
    assert_eq!(
        migrated.order,
        ["codex", "claude", "gemini", "antigravity", "cursor", "copilot"]
    );
}

#[test]
fn preferences_preserve_explicit_overrides_and_order() {
    let known = IDS.iter().map(|id| (*id).into()).collect::<Vec<_>>();
    let enabled = vec!["codex".into()];
    let configured = vec!["codex".into()];
    let ordered = vec!["codex".into(), "claude".into()];
    let migrated = migrate_provider_preferences(Some(&known), Some(&enabled), Some(&configured), Some(&ordered));
    assert!(migrated.added.is_empty());
    assert_eq!(migrated.enabled, enabled);
    assert_eq!(migrated.configured, configured);
    assert_eq!(migrated.order, ordered);
}

#[test]
fn preferences_initialize_all_providers_for_first_launch() {
    let migrated = migrate_provider_preferences(None, None, None, None);
    assert!(migrated.added.is_empty());
    assert_eq!(migrated.enabled, IDS);
    assert!(migrated.configured.is_empty());
    assert_eq!(migrated.order, IDS);
}

#[test]
fn preferences_do_not_duplicate_previously_enabled_new_providers() {
    let known = vec!["claude".into()];
    let enabled = vec!["codex".into()];
    let ordered = vec!["codex".into()];
    let migrated = migrate_provider_preferences(Some(&known), Some(&enabled), None, Some(&ordered));
    assert_eq!(migrated.enabled.iter().filter(|id| id.as_str() == "codex").count(), 1);
    assert_eq!(migrated.order.iter().filter(|id| id.as_str() == "codex").count(), 1);
}

#[test]
fn platform_setting_limits_share_the_provider_policy() {
    for (value, decimals, workdays) in [(-1, 0, 4), (5, 2, 5), (9, 2, 7)] {
        assert_eq!(normalized_percent_decimals(value), decimals);
        assert_eq!(normalized_pace_workdays(value), workdays);
    }
    assert_eq!(normalized_polling_seconds("claude", -1), Some(120));
    assert_eq!(normalized_polling_seconds("codex", 300), Some(300));
    assert_eq!(normalized_polling_seconds("codex", i64::MAX), Some(1800));
    assert_eq!(normalized_polling_seconds("unknown", 300), None);
}
