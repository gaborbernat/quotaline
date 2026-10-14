use super::RustHistoryVisibility;

#[test]
fn swift_boundary_keeps_history_selection_in_rust() {
    let visibility = RustHistoryVisibility::new();
    let series = vec!["claude:sonnet".into(), "codex:gpt".into()];
    visibility.set_hidden("usage".into(), Vec::new());
    assert!(visibility.toggle("usage".into(), "claude:sonnet".into(), series.clone()));
    assert_eq!(visibility.hidden("usage".into()), ["claude:sonnet"]);
    assert!(!visibility.visible("usage".into(), "claude:sonnet".into()));
    assert!(visibility.isolate("usage".into(), "claude:sonnet".into(), series.clone()));
    assert_eq!(visibility.hidden("usage".into()), ["codex:gpt"]);
    assert!(visibility.isolate("usage".into(), "claude:sonnet".into(), series));
    assert_eq!(visibility.hidden("usage".into()), ["claude:sonnet"]);
    visibility.clear();
    assert!(visibility.hidden("usage".into()).is_empty());
}
