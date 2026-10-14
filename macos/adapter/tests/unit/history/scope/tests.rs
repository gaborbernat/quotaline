use super::*;

#[test]
fn scope_bridge_returns_input_positions() {
    let keys = vec![
        HistoryScopeKey {
            provider: "claude".into(),
            storage_key: "claude:a".into(),
        },
        HistoryScopeKey {
            provider: "codex".into(),
            storage_key: "codex:b".into(),
        },
    ];
    assert_eq!(history_scoped_indices(keys.clone(), vec!["claude".into()], None), [0]);
    assert!(history_scoped_indices(keys, vec!["codex".into()], Some(vec![])).is_empty());
}
