use super::{HistoryVisibility, VisibilityAction};

#[test]
fn visibility_actions_apply_typed_shell_events() {
    let mut visibility = HistoryVisibility::default();
    let action = |kind: &str| {
        serde_json::from_value::<VisibilityAction>(serde_json::json!({
            "kind": kind,
            "metric": "usage",
            "id": "claude:sonnet",
            "series": ["claude:sonnet", "codex:gpt"],
        }))
        .unwrap()
    };

    assert_eq!(
        action("toggle").apply(&mut visibility, "global"),
        (true, vec!["codex:gpt".into()])
    );
    assert_eq!(
        action("isolate_undo").apply(&mut visibility, "global"),
        (true, vec!["claude:sonnet".into()])
    );
    assert_eq!(
        action("isolate").apply(&mut visibility, "global"),
        (true, vec!["claude:sonnet".into(), "codex:gpt".into()])
    );
}

fn series() -> Vec<String> {
    ["claude:sonnet", "codex:gpt", "gemini:pro"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[test]
fn toggle_keeps_one_visible_series_and_ignores_unknown_ids() {
    let mut visibility = HistoryVisibility::default();
    assert!(!visibility.toggle("usage", "usage", "missing", &series()));
    assert!(visibility.toggle("usage", "usage", "claude:sonnet", &series()));
    assert!(visibility.toggle("usage", "usage", "codex:gpt", &series()));
    assert!(!visibility.toggle("usage", "usage", "gemini:pro", &series()));
    assert_eq!(visibility.shown("usage", &series()), ["gemini:pro"]);
    assert!(visibility.toggle("usage", "usage", "claude:sonnet", &series()));
    assert_eq!(visibility.hidden("usage"), ["codex:gpt"]);
}

#[test]
fn isolation_restores_prior_hidden_set_and_changes_target() {
    let mut visibility = HistoryVisibility::default();
    visibility.set_hidden("usage", ["gemini:pro".into()]);
    assert!(!visibility.isolate("usage", "usage", "missing", &series(), false));
    assert!(visibility.isolate("usage", "usage", "claude:sonnet", &series(), false));
    assert_eq!(visibility.shown("usage", &series()), ["claude:sonnet"]);
    assert!(visibility.isolate("usage", "usage", "codex:gpt", &series(), false));
    assert_eq!(visibility.shown("usage", &series()), ["codex:gpt"]);
    assert!(visibility.isolate("usage", "usage", "codex:gpt", &series(), false));
    assert_eq!(visibility.hidden("usage"), ["gemini:pro"]);
}

#[test]
fn isolation_can_undo_a_first_click_before_double_click() {
    let mut visibility = HistoryVisibility::default();
    assert!(visibility.toggle("usage", "usage", "claude:sonnet", &series()));
    assert!(visibility.isolate("usage", "usage", "claude:sonnet", &series(), true));
    assert_eq!(visibility.shown("usage", &series()), ["claude:sonnet"]);
    assert!(visibility.isolate("usage", "usage", "claude:sonnet", &series(), false));
    assert!(visibility.hidden("usage").is_empty());
}

#[test]
fn double_click_restores_an_earlier_isolation_before_switching_series() {
    let mut visibility = HistoryVisibility::default();
    assert!(visibility.isolate("usage", "usage", "claude:sonnet", &series(), false));
    assert!(visibility.toggle("usage", "usage", "codex:gpt", &series()));
    assert!(visibility.isolate("usage", "usage", "codex:gpt", &series(), true));
    assert_eq!(visibility.shown("usage", &series()), ["codex:gpt"]);
    assert!(visibility.isolate("usage", "usage", "codex:gpt", &series(), false));
    assert!(visibility.hidden("usage").is_empty());
}

#[test]
fn rejected_last_visible_click_cannot_undo_an_earlier_toggle() {
    let mut visibility = HistoryVisibility::default();
    visibility.set_hidden(
        "usage",
        ["claude:sonnet".into(), "codex:gpt".into(), "gemini:pro".into()],
    );
    assert!(visibility.toggle("usage", "usage", "claude:sonnet", &series()));
    assert!(!visibility.toggle("usage", "usage", "claude:sonnet", &series()));
    assert!(visibility.isolate("usage", "usage", "claude:sonnet", &series(), true));
    assert!(visibility.isolate("usage", "usage", "claude:sonnet", &series(), false));
    assert_eq!(visibility.shown("usage", &series()), ["claude:sonnet"]);
}

#[test]
fn external_hidden_changes_reset_isolation_only_when_values_change() {
    let mut visibility = HistoryVisibility::default();
    visibility.set_hidden("usage", ["gemini:pro".into()]);
    assert!(visibility.isolate("usage", "usage", "claude:sonnet", &series(), false));
    let isolated = visibility.hidden("usage");
    visibility.set_hidden("usage", isolated.clone());
    assert!(visibility.isolate("usage", "usage", "claude:sonnet", &series(), false));
    assert_eq!(visibility.hidden("usage"), ["gemini:pro"]);
    visibility.set_hidden("usage", ["codex:gpt".into()]);
    assert!(visibility.isolate("usage", "usage", "codex:gpt", &series(), false));
    assert_eq!(visibility.shown("usage", &series()), ["codex:gpt"]);
    visibility.clear();
    assert_eq!(visibility.shown("usage", &series()), series());
}

#[test]
fn repeating_an_empty_hidden_set_preserves_other_scope_isolation() {
    let mut visibility = HistoryVisibility::default();
    visibility.set_hidden("usage", []);
    assert!(visibility.isolate("analytics", "analytics", "claude:sonnet", &series(), false));
    visibility.set_hidden("usage", []);
    assert!(visibility.isolate("analytics", "analytics", "claude:sonnet", &series(), false));
    assert!(visibility.hidden("analytics").is_empty());
}

#[test]
fn scopes_keep_hidden_sets_separate() {
    let mut visibility = HistoryVisibility::default();
    visibility.set_hidden("usage", ["claude:sonnet".into()]);
    assert!(!visibility.visible("usage", "claude:sonnet"));
    assert!(visibility.visible("analytics", "claude:sonnet"));
    assert!(visibility.hidden("analytics").is_empty());
    assert!(visibility.isolate("analytics", "analytics", "gemini:pro", &series(), true));
    assert_eq!(visibility.shown("analytics", &series()), ["gemini:pro"]);
    assert_eq!(visibility.hidden("usage"), ["claude:sonnet"]);
}
