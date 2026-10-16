use super::{SettingsListRequest, SettingsListWindow, settings_list};
use crate::{Provider, Settings, Window, settings_list_for_providers};
use std::collections::BTreeMap;

#[test]
fn provider_settings_list_uses_saved_selection_and_activity() {
    let mut provider = Provider::connected(
        "claude",
        "Pro".into(),
        "test".into(),
        String::new(),
        vec![
            Window::new("session", "Session", 0.0, None, "session"),
            Window::new("weekly", "Weekly", 50.0, None, "weekly"),
        ],
    );
    provider.updated = Some(100);
    let providers = BTreeMap::from([("claude".into(), provider)]);
    let settings = Settings {
        selected_windows: vec!["claude:session".into()],
        has_custom_selection: Some(true),
        ..Settings::default()
    };
    let activity = BTreeMap::from([("claude:session".into(), 90)]);

    let groups = settings_list_for_providers(&settings, &providers, &activity, "", None, true);
    assert_eq!(groups[0].rows[0].last_used_at, Some(90.0));
    assert!(groups[0].rows[0].selected);
    assert_eq!(groups[0].rows[1].last_used_at, Some(100.0));
}

#[test]
fn provider_settings_list_reveals_hidden_zero_usage_model() {
    let provider = Provider::connected(
        "claude",
        "Pro".into(),
        "test".into(),
        String::new(),
        vec![Window::new("session", "Session", 0.0, None, "session")],
    );
    let providers = BTreeMap::from([("claude".into(), provider)]);

    assert!(settings_list_for_providers(&Settings::default(), &providers, &BTreeMap::new(), "", None, true).is_empty());
    assert_eq!(
        settings_list_for_providers(
            &Settings::default(),
            &providers,
            &BTreeMap::new(),
            "missing",
            Some("claude:session"),
            true,
        )[0]
        .rows[0]
            .key,
        "claude:session"
    );
}

fn window(provider: &str, id: &str, used: f64) -> SettingsListWindow {
    SettingsListWindow {
        provider: provider.into(),
        id: id.into(),
        label: id.into(),
        scope: None,
        used,
        fetched_at: 100.0,
        last_used_at: None,
    }
}

fn request(windows: Vec<SettingsListWindow>) -> SettingsListRequest {
    SettingsListRequest {
        windows,
        selected: Vec::new(),
        overrides: Default::default(),
        provider_order: Vec::new(),
        model_order: Vec::new(),
        query: String::new(),
        hide_unused: false,
        revealed: None,
    }
}

#[test]
fn orders_providers_and_models_and_counts_hidden_selections() {
    let mut input = request(vec![
        window("claude", "session", 10.0),
        window("claude", "weekly", 0.0),
        window("codex", "session", 20.0),
    ]);
    input.provider_order = vec!["codex".into(), "claude".into()];
    input.model_order = vec!["claude:weekly".into(), "claude:session".into()];
    input.selected = vec!["claude:weekly".into()];
    input.hide_unused = true;

    let groups = settings_list(input);

    assert_eq!(
        groups.iter().map(|group| group.provider.as_str()).collect::<Vec<_>>(),
        ["codex", "claude"]
    );
    assert_eq!(
        groups[1].rows.iter().map(|row| row.key.as_str()).collect::<Vec<_>>(),
        ["claude:session"]
    );
    assert_eq!(groups[1].rows[0].position, 1);
    assert_eq!((groups[1].selected_count, groups[1].total_count), (1, 2));
}

#[test]
fn search_strips_accents_and_ignores_case() {
    let mut input = request(vec![window("claude", "weekly", 30.0)]);
    input.windows[0].label = "Éclair".into();
    input.query = " eClAiR ".into();

    assert_eq!(settings_list(input)[0].rows[0].key, "claude:weekly");
}

#[test]
fn revealed_window_bypasses_usage_and_search_filters() {
    let mut input = request(vec![window("claude", "weekly", 0.0), window("codex", "session", 50.0)]);
    input.query = "absent".into();
    input.hide_unused = true;
    assert!(settings_list(input.clone()).is_empty());
    input.revealed = Some("claude:weekly".into());

    assert_eq!(
        settings_list(input)
            .into_iter()
            .flat_map(|group| group.rows.into_iter().map(|row| row.key))
            .collect::<Vec<_>>(),
        ["claude:weekly"]
    );
}

#[test]
fn search_uses_full_unicode_case_folding() {
    let mut input = request(vec![window("claude", "weekly", 30.0)]);
    input.windows[0].label = "Straße".into();
    input.query = "STRASSE".into();

    assert_eq!(settings_list(input)[0].rows[0].key, "claude:weekly");
}

#[test]
fn search_matches_provider_id_and_resolved_label() {
    let mut input = request(vec![window("codex", "weekly", 0.0)]);
    input.overrides.insert("codex:weekly".into(), "ALIAS".into());
    for (case, query) in [
        ("resolved label", "alias"),
        ("provider", "codex"),
        ("window id", "weekly"),
    ] {
        input.query = query.into();
        let groups = settings_list(input.clone());
        let row = &groups[0].rows[0];
        assert_eq!(
            (row.key.as_str(), row.label.as_str(), row.overridden),
            ("codex:weekly", "ALIAS", true),
            "{case}"
        );
    }
}

#[test]
fn uses_last_observation_for_nonzero_window_and_explicit_history_for_zero_window() {
    let mut input = request(vec![window("claude", "session", 20.0), window("claude", "weekly", 0.0)]);
    input.windows[1].last_used_at = Some(20.0);
    input.hide_unused = true;
    input.selected = vec!["claude:session".into()];
    input.overrides.insert("claude:weekly".into(), String::new());
    let rows = settings_list(input)[0].rows.clone();
    assert_eq!(
        rows.iter().map(|row| row.last_used_at).collect::<Vec<_>>(),
        [Some(100.0), Some(20.0)]
    );
    assert!(rows[0].selected);
    assert!(!rows[1].selected);
    assert!(!rows[1].overridden);
}

#[test]
fn returns_no_groups_without_windows() {
    assert!(settings_list(request(Vec::new())).is_empty());
}

#[test]
fn selection_controls_reject_clearing_the_last_available_model() {
    let mut input = request(vec![window("claude", "session", 10.0), window("codex", "weekly", 10.0)]);
    input.selected = vec!["claude:session".into(), "removed:window".into()];
    let groups = settings_list(input);
    assert!(!groups[0].can_toggle_all);
    assert!(!groups[0].rows[0].can_toggle);
    assert!(groups[1].can_toggle_all);
    assert!(groups[1].rows[0].can_toggle);
}
