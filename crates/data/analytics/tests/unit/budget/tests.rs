//! Series point budget cases.

use super::*;

#[test]
fn limits_cap_each_series_and_share_total() {
    assert_eq!(point_limits(&[500, 500, 500], 400, 1200), [400, 400, 400]);
    assert_eq!(point_limits(&[500, 10, 500], 400, 1200), [400, 10, 400]);
}

#[test]
fn limits_cover_empty_tiny_and_oversubscribed_budgets() {
    assert_eq!(point_limits(&[], 400, 1200), Vec::<usize>::new());
    assert_eq!(point_limits(&[0, 0], 400, 1200), [0, 0]);
    assert_eq!(point_limits(&[4, 4, 4], 4, 2), [1, 1, 1]);
    assert_eq!(point_limits(&[0, 10, 10, 0], 400, 3), [0, 2, 1, 0]);
    assert_eq!(point_limits(&[1, 10, 10], 400, 3), [1, 1, 1]);
    assert_eq!(point_limits(&[1, 10, 10], 400, 4), [1, 2, 1]);
}
