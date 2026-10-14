//! macOS status projection bridge cases.

use super::*;
use crate::test_records as records;

fn window(id: &str, used: f64) -> crate::provider_record::ProviderWindowRecord {
    records::window(id, used, Some(200.0))
}

fn selected(provider: &str, window_id: &str) -> StatusSelectedWindowRecord {
    StatusSelectedWindowRecord {
        provider: provider.into(),
        window_id: window_id.into(),
    }
}

#[test]
fn bridge_exposes_projected_data_for_native_layout() {
    let entries = project_status(
        vec![
            records::provider("claude", vec![window("session", 20.0), window("weekly", 100.0)]),
            records::provider("codex", vec![window("session", 80.0)]),
        ],
        vec![selected("claude", "session"), selected("codex", "session")],
        HashMap::from([("claude:session".into(), "EDIT".into())]),
        100.0,
        false,
        false,
    )
    .entries;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].provider, "claude");
    assert_eq!(entries[0].provider_code, "CC");
    assert_eq!(entries[0].provider_name, "Claude");
    assert_eq!(entries[0].window_id, "session");
    assert_eq!(entries[0].window_label, "session");
    assert_eq!(entries[0].window_scope, None);
    assert_eq!(entries[0].short_label, "EDIT");
    assert_eq!(entries[0].used, 100.0);
    assert_eq!(entries[0].reset_at, Some(200.0));
    assert!(entries[0].active);
    assert!(entries[0].restricted);
    assert_eq!(
        entries[0].restrictions,
        [StatusRestrictionRecord {
            label: "weekly".into(),
            reset_at: Some(200.0)
        }]
    );
    assert_eq!(entries[1].provider, "codex");
    assert_eq!(entries[1].used, 80.0);
    assert!(entries[1].restrictions.is_empty());
}

#[test]
fn bridge_ignores_unknown_selections() {
    assert!(
        project_status(
            vec![],
            vec![selected("claude", "session")],
            HashMap::new(),
            100.0,
            false,
            false
        )
        .entries
        .is_empty()
    );
}

fn selection_projection(hide_zero: bool, usage_order: bool) -> StatusProjectionRecord {
    project_status(
        vec![
            records::provider("claude", vec![window("session", 0.0), window("weekly", 70.0)]),
            records::provider("codex", vec![window("session", 40.0)]),
        ],
        vec![
            selected("claude", "session"),
            selected("claude", "weekly"),
            selected("codex", "session"),
        ],
        HashMap::new(),
        100.0,
        hide_zero,
        usage_order,
    )
}

#[test]
fn selection_bridge_filters_zero_windows() {
    assert_eq!(selection_projection(true, false).all_selection.indices, vec![1, 2]);
}

#[test]
fn selection_bridge_sorts_windows_by_usage() {
    assert_eq!(selection_projection(false, true).all_selection.indices, vec![1, 2, 0]);
}

#[test]
fn selection_bridge_chooses_worst_per_provider() {
    assert_eq!(selection_projection(false, false).worst_selection.indices, vec![1, 2]);
}

#[test]
fn selection_bridge_groups_mini_bars() {
    assert_eq!(
        selection_projection(false, false).mini_selection.groups,
        vec![
            StatusSelectionGroupRecord {
                provider: "claude".into(),
                indices: vec![0, 1],
                used: 70.0,
                restricted: false,
            },
            StatusSelectionGroupRecord {
                provider: "codex".into(),
                indices: vec![2],
                used: 40.0,
                restricted: false,
            },
        ]
    );
}

#[test]
fn selection_bridge_prioritizes_blocking_quota() {
    assert_eq!(
        project_status(
            vec![records::provider(
                "claude",
                vec![window("session", 0.0), window("weekly", 100.0)]
            )],
            vec![selected("claude", "session"), selected("claude", "weekly")],
            HashMap::new(),
            100.0,
            false,
            false,
        )
        .worst_selection
        .indices,
        vec![0]
    );
}
