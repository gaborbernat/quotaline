use super::*;

fn entries() -> [StatusSelectionEntry<'static>; 5] {
    [
        StatusSelectionEntry {
            provider: "claude",
            used: 0.0,
            restricted: false,
        },
        StatusSelectionEntry {
            provider: "codex",
            used: 60.0,
            restricted: false,
        },
        StatusSelectionEntry {
            provider: "claude",
            used: 70.0,
            restricted: false,
        },
        StatusSelectionEntry {
            provider: "codex",
            used: 20.0,
            restricted: true,
        },
        StatusSelectionEntry {
            provider: "claude",
            used: 40.0,
            restricted: true,
        },
    ]
}

#[test]
fn all_keeps_input_order_and_can_hide_zero() {
    assert_eq!(
        select_status(&entries(), StatusSelectionMode::All, false, false).indices,
        vec![0, 1, 2, 3, 4]
    );
    assert_eq!(
        select_status(&entries(), StatusSelectionMode::All, true, false).indices,
        vec![1, 2, 3, 4]
    );
}

#[test]
fn usage_order_is_stable_for_equal_values() {
    let entries = [
        StatusSelectionEntry {
            provider: "claude",
            used: 40.0,
            restricted: false,
        },
        StatusSelectionEntry {
            provider: "codex",
            used: 40.0,
            restricted: false,
        },
        StatusSelectionEntry {
            provider: "gemini",
            used: 50.0,
            restricted: false,
        },
    ];
    assert_eq!(
        select_status(&entries, StatusSelectionMode::All, false, true).indices,
        vec![2, 0, 1]
    );
}

#[test]
fn worst_per_provider_prioritizes_restriction_then_usage() {
    assert_eq!(
        select_status(&entries(), StatusSelectionMode::WorstPerProvider, false, false).indices,
        vec![4, 3]
    );
}

#[test]
fn worst_per_provider_sorts_selected_windows_by_usage() {
    assert_eq!(
        select_status(&entries(), StatusSelectionMode::WorstPerProvider, false, true).indices,
        vec![4, 3]
    );
}

#[test]
fn worst_per_provider_keeps_restricted_window_over_higher_unrestricted_usage() {
    let entries = [
        StatusSelectionEntry {
            provider: "claude",
            used: 30.0,
            restricted: true,
        },
        StatusSelectionEntry {
            provider: "claude",
            used: 90.0,
            restricted: false,
        },
    ];
    assert_eq!(
        select_status(&entries, StatusSelectionMode::WorstPerProvider, false, false).indices,
        vec![0]
    );
}

#[test]
fn worst_per_provider_keeps_higher_usage_when_restrictions_match() {
    for restricted in [false, true] {
        let entries = [
            StatusSelectionEntry {
                provider: "claude",
                used: 70.0,
                restricted,
            },
            StatusSelectionEntry {
                provider: "claude",
                used: 40.0,
                restricted,
            },
        ];
        assert_eq!(
            select_status(&entries, StatusSelectionMode::WorstPerProvider, false, false).indices,
            vec![0]
        );
    }
}

#[test]
fn mini_bars_groups_windows_and_rolls_up_maximum() {
    let selection = select_status(&entries(), StatusSelectionMode::MiniBars, false, false);
    assert_eq!(selection.groups.len(), 2);
    assert_eq!(
        selection.groups[0],
        StatusSelectionGroup {
            provider: "claude".into(),
            indices: vec![0, 2, 4],
            used: 70.0,
            restricted: true,
        }
    );
    assert_eq!(
        selection.groups[1],
        StatusSelectionGroup {
            provider: "codex".into(),
            indices: vec![1, 3],
            used: 60.0,
            restricted: true,
        }
    );
}

#[test]
fn mini_bars_sorts_groups_by_maximum_usage_then_provider() {
    let entries = [
        StatusSelectionEntry {
            provider: "codex",
            used: 40.0,
            restricted: false,
        },
        StatusSelectionEntry {
            provider: "claude",
            used: 40.0,
            restricted: false,
        },
    ];
    assert_eq!(
        select_status(&entries, StatusSelectionMode::MiniBars, false, true)
            .groups
            .into_iter()
            .map(|group| group.provider)
            .collect::<Vec<_>>(),
        vec!["claude", "codex"]
    );
}

#[test]
fn icon_only_has_no_selected_windows_or_groups() {
    assert_eq!(
        select_status(&entries(), StatusSelectionMode::IconOnly, false, false),
        StatusSelection {
            indices: vec![],
            groups: vec![]
        }
    );
}

#[test]
fn empty_selection_has_no_groups() {
    assert_eq!(
        select_status(&[], StatusSelectionMode::MiniBars, true, true).groups,
        vec![]
    );
}
