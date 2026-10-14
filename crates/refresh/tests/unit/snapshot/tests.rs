use super::*;

#[test]
fn fetched_snapshots_follow_account_and_freshness_policy() {
    for (previous_account, fetched_account, previous_fetched_at, fetched_at, complete, expected) in [
        (None, None, None, Some(100.0), true, (false, true, true)),
        (None, Some("a"), None, Some(100.0), true, (true, true, true)),
        (Some("a"), None, Some(100.0), Some(90.0), false, (false, false, false)),
        (
            Some("a"),
            Some("a"),
            Some(100.0),
            Some(100.0),
            false,
            (false, true, false),
        ),
        (
            Some("a"),
            Some("a"),
            Some(100.0),
            Some(101.0),
            false,
            (false, true, false),
        ),
        (
            Some("a"),
            Some("a"),
            Some(100.0),
            Some(90.0),
            false,
            (false, false, false),
        ),
        (
            Some("a"),
            Some("b"),
            Some(100.0),
            Some(90.0),
            false,
            (true, true, false),
        ),
        (Some("a"), Some("a"), Some(100.0), None, false, (false, false, false)),
        (Some("a"), Some("a"), None, None, false, (false, true, false)),
        (Some("a"), Some("a"), Some(100.0), Some(90.0), true, (false, true, true)),
    ] {
        let decision = snapshot_transition(SnapshotTransitionInput {
            previous_account,
            fetched_account,
            previous_fetched_at,
            fetched_at,
            complete,
        });
        assert_eq!(
            (
                decision.account_changed,
                decision.replace_snapshot,
                decision.record_history
            ),
            expected,
        );
    }
}
