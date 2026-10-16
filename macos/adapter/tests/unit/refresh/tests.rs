use super::*;

#[test]
fn ffi_snapshot_transition_preserves_shared_decisions() {
    let decision = snapshot_transition(SnapshotTransitionInput {
        previous_account: Some("old".into()),
        fetched_account: Some("new".into()),
        previous_fetched_at: Some(100.0),
        fetched_at: Some(90.0),
        complete: false,
    });
    assert_eq!(
        decision,
        SnapshotTransition {
            account_changed: true,
            replace_snapshot: true,
            record_history: false,
        }
    );
}

#[test]
fn polling_interval_exposes_shared_policy() {
    assert_eq!(polling_interval(60.0, 120.0, 300.0, true), 120.0);
    assert_eq!(polling_interval(60.0, 120.0, 30.0, false), 60.0);
}
