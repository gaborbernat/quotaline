//! Checks default quota selection through its public policy.

use super::*;
use crate::{Window, demo};

#[test]
fn history_selection_respects_custom_empty_and_default_choices() {
    let providers = BTreeMap::from([(
        "claude".into(),
        Provider::connected(
            "claude",
            "Pro".into(),
            "test".into(),
            String::new(),
            vec![Window::new("session", "Session", 12.0, None, "session")],
        ),
    )]);
    assert_eq!(
        history_window_selection(&Settings::default(), &providers),
        Some(BTreeSet::from(["claude:session".into()]))
    );
    assert_eq!(history_window_selection(&Settings::default(), &BTreeMap::new()), None);
    let settings = Settings {
        has_custom_selection: Some(true),
        ..Settings::default()
    };
    assert_eq!(history_window_selection(&settings, &providers), Some(BTreeSet::new()));
}

#[test]
fn available_state_selection_drops_removed_models() {
    let providers = demo();
    let settings = Settings {
        selected_windows: vec!["missing:window".into(), "claude:session".into()],
        has_custom_selection: Some(true),
        ..Settings::default()
    };

    assert_eq!(
        available_selected_window_keys(&settings, &providers),
        ["claude:session"]
    );
}

#[test]
fn available_selection_discards_removed_models_without_reordering() {
    assert_eq!(
        available_window_selection(
            &["claude:session".into(), "removed:window".into(), "codex:weekly".into()],
            &["codex:weekly".into(), "claude:session".into()],
        ),
        ["claude:session", "codex:weekly"]
    );
}

#[test]
fn preferred_windows_follow_provider_order() {
    assert_eq!(
        default_window_ids([
            ("monthly", true),
            ("session", true),
            ("weekly:fable", true),
            ("weekly", true),
            ("weekly:inactive", false),
        ]),
        ["session", "weekly:fable", "weekly"]
    );
}

#[test]
fn fallback_uses_first_two_active_windows() {
    assert_eq!(
        default_window_ids([
            ("inactive", false),
            ("model:a", true),
            ("model:b", true),
            ("model:c", true)
        ]),
        ["model:a", "model:b"]
    );
    assert!(default_window_ids([("inactive", false)]).is_empty());
}

#[test]
fn toggling_window_preserves_order_and_one_selection() {
    let selected = ["claude:session".into(), "codex:weekly".into()];
    assert_eq!(
        toggle_window(&selected, "claude:session"),
        Some(vec!["codex:weekly".into()])
    );
    assert_eq!(
        toggle_window(&selected, "gemini:daily"),
        Some(vec![
            "claude:session".into(),
            "codex:weekly".into(),
            "gemini:daily".into()
        ])
    );
    assert_eq!(toggle_window(&["claude:session".into()], "claude:session"), None);
}

#[test]
fn toggling_provider_windows_preserves_other_providers_and_one_selection() {
    let selected = ["claude:session".into(), "codex:weekly".into()];
    let claude = ["claude:session".into(), "claude:weekly".into()];
    assert_eq!(
        toggle_provider_windows(&selected, &claude),
        Some(vec![
            "claude:session".into(),
            "codex:weekly".into(),
            "claude:weekly".into()
        ])
    );
    assert_eq!(
        toggle_provider_windows(
            &["claude:session".into(), "claude:weekly".into(), "codex:weekly".into()],
            &claude
        ),
        Some(vec!["codex:weekly".into()])
    );
    assert_eq!(
        toggle_provider_windows(&["claude:session".into()], &["claude:session".into()]),
        None
    );
}

#[test]
fn applying_changes_preserves_order_and_rejects_last_removal() {
    let selected = vec!["claude:session".into(), "codex:weekly".into()];
    assert_eq!(
        apply_window_changes(
            &selected,
            [
                ("claude:session", false),
                ("codex:weekly", false),
                ("gemini:daily", true),
                ("gemini:daily", true),
            ],
        ),
        ["codex:weekly", "gemini:daily"]
    );
    assert_eq!(apply_window_changes(&selected, [("claude:session", true)]), selected);
}
