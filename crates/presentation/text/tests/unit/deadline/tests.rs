//! Checks the public behavior of the adjacent module.

use super::*;

#[test]
fn age_wakes_at_the_next_displayed_unit() {
    for (name, target, expected) in [
        ("seconds", 970.0, 1030.0),
        ("new", 1000.0, 1060.0),
        ("future", 1010.0, 1070.0),
        ("hours", -6200.0, 4600.0),
        ("days", -171_800.0, 87_400.0),
    ] {
        assert_eq!(
            next_update(DeadlineKind::Age, Some(target), 1000.0),
            Some(expected),
            "{name}"
        );
    }
}

#[test]
fn reset_wakes_after_the_countdown_changes() {
    for (name, target, expected) in [
        ("seconds", 1030.0, 1030.0),
        ("minutes", 4601.0, 1001.0f64.next_up()),
        ("hours", 90_975.0, 4575.0f64.next_up()),
    ] {
        assert_eq!(
            next_update(DeadlineKind::Reset, Some(target), 1000.0),
            Some(expected),
            "{name}"
        );
    }
}

#[test]
fn absent_and_expired_deadlines_do_not_schedule_updates() {
    for (name, kind, target) in [
        ("missing age", DeadlineKind::Age, None),
        ("missing reset", DeadlineKind::Reset, None),
        ("due reset", DeadlineKind::Reset, Some(1000.0)),
        ("past reset", DeadlineKind::Reset, Some(999.0)),
    ] {
        assert_eq!(next_update(kind, target, 1000.0), None, "{name}");
    }
}
