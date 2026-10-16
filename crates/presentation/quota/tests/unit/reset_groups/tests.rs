use super::*;

#[test]
fn adjacent_windows_share_only_a_known_matching_reset() {
    let windows = [
        ResetGroupInput {
            reset_at: None,
            precision: ResetPrecision::Instant,
        },
        ResetGroupInput {
            reset_at: None,
            precision: ResetPrecision::Instant,
        },
        ResetGroupInput {
            reset_at: Some(10.0),
            precision: ResetPrecision::Instant,
        },
        ResetGroupInput {
            reset_at: Some(10.0),
            precision: ResetPrecision::Instant,
        },
        ResetGroupInput {
            reset_at: Some(10.0),
            precision: ResetPrecision::Day,
        },
        ResetGroupInput {
            reset_at: Some(20.0),
            precision: ResetPrecision::Day,
        },
    ];
    assert_eq!(
        reset_group_edges(&windows),
        vec![
            ResetGroupEdge {
                with_previous: false,
                with_next: false
            },
            ResetGroupEdge {
                with_previous: false,
                with_next: false
            },
            ResetGroupEdge {
                with_previous: false,
                with_next: true
            },
            ResetGroupEdge {
                with_previous: true,
                with_next: false
            },
            ResetGroupEdge {
                with_previous: false,
                with_next: false
            },
            ResetGroupEdge {
                with_previous: false,
                with_next: false
            },
        ]
    );
}

#[test]
fn reset_group_edges_cover_empty_and_single_windows() {
    assert!(reset_group_edges(&[]).is_empty());
    assert_eq!(
        reset_group_edges(&[ResetGroupInput {
            reset_at: Some(10.0),
            precision: ResetPrecision::Instant
        }]),
        vec![ResetGroupEdge {
            with_previous: false,
            with_next: false
        }]
    );
}

#[test]
fn quota_reset_prefers_deadline_changes_when_both_deadlines_exist() {
    for (current, expected) in [(Some(11.0), true), (Some(10.0), false), (Some(9.0), false)] {
        assert_eq!(has_reset(Some(10.0), current, 50.0, 0.0), expected);
    }
}

#[test]
fn quota_reset_uses_usage_drop_without_comparable_deadlines() {
    for (previous, current, used, expected) in [
        (None, None, 48.9, true),
        (None, Some(10.0), 49.0, false),
        (Some(10.0), None, 49.1, false),
    ] {
        assert_eq!(has_reset(previous, current, 50.0, used), expected);
    }
}
