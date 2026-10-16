//! Checks threshold edits keep the stored list valid.

use super::*;

#[test]
fn threshold_update_adds_and_removes_in_sorted_order() {
    assert_eq!(set_threshold([90, 75, 90], 50, true), [50, 75, 90]);
    assert_eq!(set_threshold([90, 75, 90], 90, false), [75]);
}

#[test]
fn threshold_update_normalizes_invalid_values() {
    assert_eq!(set_threshold([0, 75, 150], 101, true), [75]);
    assert!(set_threshold([0, 75, 150], 75, false).is_empty());
}
