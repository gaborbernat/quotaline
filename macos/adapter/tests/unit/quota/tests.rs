use super::*;

#[test]
fn ffi_reset_groups_preserve_precision() {
    let windows = vec![
        ResetGroupInput {
            reset_at: Some(10.0),
            day_precision: false,
        },
        ResetGroupInput {
            reset_at: Some(10.0),
            day_precision: false,
        },
        ResetGroupInput {
            reset_at: Some(10.0),
            day_precision: true,
        },
    ];
    assert_eq!(
        reset_group_edges(windows),
        vec![
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
        ]
    );
}

#[test]
fn ffi_quota_reset_uses_shared_policy() {
    assert!(quota_has_reset(Some(10.0), Some(11.0), 80.0, 0.0));
    assert!(!quota_has_reset(Some(10.0), Some(10.0), 80.0, 0.0));
}
