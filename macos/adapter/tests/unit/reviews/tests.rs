//! macOS review bridge cases.

use super::*;

#[test]
fn review_bridge_returns_shared_counts() {
    let result = review_counts(
        vec![ReviewRowRecord {
            day: "2025-10-13".into(),
            value: 3.0,
        }],
        1_760_354_400.0,
    );
    assert_eq!(result, Some(ReviewCountsRecord { today: 3, week: 3 }));
    assert_eq!(review_counts(Vec::new(), 1_760_354_400.0), None);
}

#[test]
fn review_bridge_rejects_unsupported_time() {
    assert_eq!(review_counts(Vec::new(), f64::INFINITY), None);
}
