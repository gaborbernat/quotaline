//! Checks default quota selection through the macOS boundary.

use super::*;

#[test]
fn selection_changes_use_shared_policy() {
    assert_eq!(
        apply_window_selection(
            vec!["claude:session".into(), "codex:weekly".into()],
            vec![
                SelectionChangeRecord {
                    key: "claude:session".into(),
                    on: false
                },
                SelectionChangeRecord {
                    key: "codex:weekly".into(),
                    on: false
                },
            ],
        ),
        ["codex:weekly"]
    );
}

#[test]
fn selection_preserves_provider_and_window_order() {
    let windows = |ids: &[(&str, bool)]| {
        ids.iter()
            .map(|(id, is_active)| SelectionWindow {
                id: (*id).into(),
                is_active: *is_active,
            })
            .collect()
    };
    let selected = default_window_selection(vec![
        SelectionProvider {
            id: "antigravity".into(),
            windows: windows(&[("gemini:session", true), ("3p:session", true), ("gemini:weekly", true)]),
        },
        SelectionProvider {
            id: "claude".into(),
            windows: windows(&[("session", true), ("weekly", true), ("inactive", false)]),
        },
    ]);
    assert_eq!(
        selected,
        [
            SelectedWindow {
                provider: "antigravity".into(),
                window_id: "gemini:session".into(),
            },
            SelectedWindow {
                provider: "antigravity".into(),
                window_id: "3p:session".into(),
            },
            SelectedWindow {
                provider: "claude".into(),
                window_id: "session".into(),
            },
            SelectedWindow {
                provider: "claude".into(),
                window_id: "weekly".into(),
            },
        ]
    );
    assert!(default_window_selection(Vec::new()).is_empty());
}
