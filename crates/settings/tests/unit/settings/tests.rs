//! Checks settings behavior through the public API.

use super::*;

#[test]
fn provider_refresh_defaults_match_macos_policy() {
    assert_eq!(
        Settings::default().interval_minutes,
        BTreeMap::from([
            ("claude".into(), 5),
            ("codex".into(), 2),
            ("gemini".into(), 2),
            ("antigravity".into(), 2),
            ("cursor".into(), 5),
            ("copilot".into(), 5),
        ])
    );
}

#[test]
fn history_default_retention_is_two_months() {
    assert_eq!(Settings::default().retention_days, 60);
}

#[test]
fn analytics_default_interval_is_fifteen_minutes() {
    assert_eq!(Settings::default().analytics_interval_minutes, 15);
}

#[test]
fn default_status_template_stacks_the_percentage() {
    assert_eq!(Settings::default().template, "{provider} {window}\\n{pct}");
}

#[test]
fn history_retention_uses_shared_bounds() {
    for (requested, expected) in [(0, 7), (30, 30), (500, 365)] {
        let mut settings = Settings::default();
        settings.set("retention_days", serde_json::json!(requested)).unwrap();
        assert_eq!(settings.retention_days, expected);
    }
}

#[test]
fn analytics_interval_uses_shared_steps_and_bounds() {
    for (requested, expected) in [(0, 5), (17, 20), (30, 30), (500, 120)] {
        let mut settings = Settings::default();
        settings
            .set("analytics_interval_minutes", serde_json::json!(requested))
            .unwrap();
        assert_eq!(settings.analytics_interval_minutes, expected);
    }
}

#[test]
fn provider_refresh_intervals_respect_floors_and_ceiling() {
    let mut settings = Settings::default();
    settings
        .set(
            "interval_minutes",
            serde_json::json!({"claude":1,"codex":0,"cursor":40}),
        )
        .unwrap();
    assert_eq!(settings.interval_minutes["claude"], 2);
    assert_eq!(settings.interval_minutes["codex"], 1);
    assert_eq!(settings.interval_minutes["cursor"], 30);
}

#[test]
fn provider_refresh_intervals_fill_missing_providers_and_ignore_unknown_ids() {
    let mut settings = Settings::default();
    settings
        .set("interval_minutes", serde_json::json!({"claude":3,"unknown":4}))
        .unwrap();
    assert_eq!(settings.interval_minutes.len(), IDS.len());
    assert_eq!(settings.interval_minutes["claude"], 3);
    assert_eq!(settings.interval_minutes["gemini"], 2);
    assert!(!settings.interval_minutes.contains_key("unknown"));
}

#[test]
fn launch_demo_request_stays_out_of_stored_preferences() {
    let mut settings = Settings::default();
    settings.request_demo(true);
    assert!(settings.demo);
    assert!(!settings.stored().demo);
}

#[test]
fn explicit_demo_switch_overrides_a_launch_request() {
    let mut settings = Settings::default();
    settings.request_demo(true);
    settings.set("demo", serde_json::json!(false)).unwrap();
    settings.request_demo(true);
    assert_eq!((settings.demo, settings.demo_choice), (false, Some(false)));
}

#[test]
fn direct_demo_preference_writes_are_rejected() {
    let mut settings = Settings::default();
    assert!(settings.set("demo_choice", serde_json::json!(true)).is_err());
}

#[test]
fn provider_switches_fill_missing_providers_and_ignore_unknown_ids() {
    let mut settings = Settings::default();
    settings
        .set("providers", serde_json::json!({"claude":false,"unknown":true}))
        .unwrap();
    assert_eq!(settings.providers.len(), IDS.len());
    assert!(!settings.providers["claude"]);
    assert!(!settings.providers["gemini"]);
    assert!(!settings.providers.contains_key("unknown"));
}

#[test]
fn settings_reject_ambiguous_labels() {
    let mut settings = Settings::default();
    assert!(
        settings
            .set("short_labels", serde_json::json!({"claude:a":"A", "codex:b":"A"}))
            .is_err()
    );
}

#[test]
fn settings_keep_an_unchanged_short_label() {
    let mut settings = Settings::default();
    let labels = serde_json::json!({"claude:a":"CUSTOM"});
    settings.set("short_labels", labels.clone()).unwrap();
    settings.set("short_labels", labels).unwrap();
    assert_eq!(settings.short_labels["claude:a"], "CUSTOM");
}

#[test]
fn settings_count_graphemes_in_short_labels() {
    let mut settings = Settings::default();
    settings
        .set("short_labels", serde_json::json!({"claude:a":"A👨‍👩‍👧‍👦BCDE"}))
        .unwrap();
    assert_eq!(settings.short_labels["claude:a"], "A👨‍👩‍👧‍👦BCDE");
}

#[test]
fn settings_reject_more_than_six_short_label_graphemes() {
    let mut settings = Settings::default();
    assert!(
        settings
            .set("short_labels", serde_json::json!({"claude:a":"A👨‍👩‍👧‍👦BCDEF"}))
            .is_err()
    );
}

#[test]
fn settings_reject_case_and_diacritic_equivalent_short_labels() {
    let mut settings = Settings::default();
    assert!(
        settings
            .set("short_labels", serde_json::json!({"claude:a":"Écho", "codex:b":"echo"}))
            .is_err()
    );
}

#[test]
fn unknown_setting_is_rejected() {
    assert_eq!(
        Settings::default().set("missing", serde_json::json!(true)),
        Err("Unknown setting".into())
    );
}

#[test]
fn legacy_empty_selection_restores_default_mode() {
    let mut settings: Settings = serde_json::from_value(serde_json::json!({"selected_windows": []})).unwrap();
    settings.normalize();
    assert_eq!(settings.has_custom_selection, Some(false));
}

#[test]
fn legacy_explicit_selection_remains_custom() {
    let mut settings: Settings = serde_json::from_value(serde_json::json!({
        "selected_windows": ["claude:session"]
    }))
    .unwrap();
    settings.normalize();
    assert_eq!(settings.has_custom_selection, Some(true));
    assert_eq!(settings.selected_windows, ["claude:session"]);
}

#[test]
fn legacy_empty_selection_marker_migrates_to_an_empty_list() {
    let mut settings: Settings = serde_json::from_value(serde_json::json!({"selected_windows": ["-"]})).unwrap();
    settings.normalize();
    assert_eq!(settings.selected_windows, Vec::<String>::new());
    assert_eq!(settings.has_custom_selection, Some(true));
}

#[test]
fn explicit_selection_mode_survives_normalization() {
    let mut settings = Settings {
        selected_windows: vec!["claude:session".into()],
        ..Settings::default()
    };
    settings.normalize();
    assert_eq!(settings.has_custom_selection, Some(false));
}

#[test]
fn selection_mode_rejects_non_boolean_values() {
    let mut settings = Settings::default();
    assert_eq!(
        settings.set("has_custom_selection", serde_json::Value::Null),
        Err("Invalid selection mode".into())
    );
}

#[test]
fn default_history_range_is_today() {
    assert_eq!(Settings::default().history_days, 1);
}

#[test]
fn unsupported_panel_format_is_rejected() {
    assert_eq!(
        Settings::default().set("format", serde_json::json!("grid")),
        Err("Invalid format".into())
    );
}

#[test]
fn invalid_history_preferences_are_rejected() {
    for (key, value) in [
        ("last_tab", serde_json::json!("Charts")),
        ("history_days", serde_json::json!(2)),
        ("history_metric", serde_json::json!("missing")),
        ("history_rollup", serde_json::json!("week")),
    ] {
        assert!(
            Settings::default().set(key, value).is_err(),
            "{key} accepted an invalid choice"
        );
    }
}

#[test]
fn available_history_metric_is_accepted() {
    let mut settings = Settings::default();
    settings.set("history_metric", serde_json::json!("costUSD")).unwrap();
    assert_eq!(settings.history_metric, "costUSD");
}

#[test]
fn widget_size_accepts_only_the_three_supported_views() {
    let mut settings = Settings::default();
    for size in ["small", "medium", "large"] {
        settings.set("widget_size", serde_json::json!(size)).unwrap();
        assert_eq!(settings.widget_size, size);
    }
    assert!(settings.set("widget_size", serde_json::json!("huge")).is_err());
    assert_eq!(settings.widget_size, "large");
}

#[test]
fn provider_order_rejects_unknown_providers() {
    let mut settings = Settings::default();
    assert!(settings.set("provider_order", serde_json::json!(["unknown"])).is_err());
}

#[test]
fn provider_order_rejects_duplicate_providers() {
    let mut settings = Settings::default();
    assert!(
        settings
            .set("provider_order", serde_json::json!(["claude", "claude"]))
            .is_err()
    );
}

#[test]
fn setting_rejects_invalid_value_types_without_changing_state() {
    let mut settings = Settings::default();
    assert!(settings.set("decimals", serde_json::json!("two")).is_err());
    assert_eq!(settings.decimals, 0);
}

#[test]
fn model_order_rejects_duplicate_windows() {
    let mut settings = Settings::default();
    assert!(
        settings
            .set("model_order", serde_json::json!(["codex:weekly", "codex:weekly"]))
            .is_err()
    );
}

fn stored(document: serde_json::Value) -> Settings {
    let mut settings: Settings = serde_json::from_value(document).unwrap();
    settings.normalize();
    settings
}

#[test]
fn providers_added_after_a_document_was_written_start_enabled_and_last() {
    let settings = stored(serde_json::json!({
        "providers": {"claude": false, "codex": true},
        "provider_order": ["codex", "claude"],
        "known_providers": ["claude", "codex"]
    }));
    assert!(!settings.providers["claude"]);
    assert!(settings.providers["gemini"]);
    assert_eq!(settings.provider_order[..2], ["codex", "claude"]);
    assert_eq!(settings.provider_order.len(), IDS.len());
    assert_eq!(settings.known_providers.as_deref().unwrap().len(), IDS.len());
}

#[test]
fn added_providers_already_in_the_order_keep_their_place() {
    let settings = stored(serde_json::json!({
        "provider_order": ["gemini", "claude"],
        "known_providers": ["claude"]
    }));
    assert_eq!(settings.provider_order[..2], ["gemini", "claude"]);
    assert_eq!(settings.provider_order.iter().filter(|id| *id == "gemini").count(), 1);
}

#[test]
fn documents_without_known_providers_keep_missing_providers_off() {
    let settings = stored(serde_json::json!({"providers": {"claude": true}}));
    assert!(!settings.providers["gemini"]);
    assert!(settings.known_providers.is_some());
}

#[test]
fn replacing_the_document_normalizes_every_field() {
    let mut settings = Settings::default();
    let changed = Settings {
        retention_days: 1,
        decimals: 9,
        notify_thresholds: vec![100, 75, 75],
        ..Settings::default()
    };
    settings.replace(changed).unwrap();
    assert_eq!(settings.retention_days, 7);
    assert_eq!(settings.decimals, 2);
    assert_eq!(settings.notify_thresholds, [75, 100]);
}

#[test]
fn replacing_the_document_rejects_invalid_values_without_changing_state() {
    let mut settings = Settings::default();
    let changed = Settings {
        format: "unknown".into(),
        ..Settings::default()
    };
    assert_eq!(settings.replace(changed), Err("Invalid format".into()));
    assert_eq!(settings.format, "stacked");
}

#[test]
fn choosing_windows_marks_the_selection_as_custom() {
    let mut settings = Settings::default();
    settings
        .set("selected_windows", serde_json::json!(["claude:session"]))
        .unwrap();
    assert_eq!(settings.has_custom_selection, Some(true));
}

#[test]
fn clearing_the_custom_flag_returns_to_the_default_selection() {
    let mut settings = Settings::default();
    settings.set("selected_windows", serde_json::json!([])).unwrap();
    settings.set("has_custom_selection", serde_json::json!(false)).unwrap();
    assert_eq!(settings.has_custom_selection, Some(false));
}
