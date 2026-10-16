use super::*;

#[test]
fn scope_filter_preserves_input_order_for_active_selected_windows() {
    let keys = [
        ScopeKey {
            provider: "claude".into(),
            storage_key: "claude:a".into(),
        },
        ScopeKey {
            provider: "codex".into(),
            storage_key: "codex:b".into(),
        },
        ScopeKey {
            provider: "claude".into(),
            storage_key: "claude:c".into(),
        },
    ];
    for (case, active, selected, expected) in [
        (
            "all selected",
            vec!["claude".into(), "codex".into()],
            None,
            vec![0, 1, 2],
        ),
        ("provider", vec!["claude".into()], None, vec![0, 2]),
        ("window", vec!["claude".into()], Some(vec!["claude:c".into()]), vec![2]),
        ("empty selection", vec!["claude".into()], Some(vec![]), vec![]),
        ("no providers", vec![], None, vec![]),
    ] {
        assert_eq!(scoped_indices(&keys, &active, selected.as_deref()), expected, "{case}");
    }
}
