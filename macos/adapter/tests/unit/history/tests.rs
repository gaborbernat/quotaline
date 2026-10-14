//! Checks the shared history policy through the macOS boundary.

use super::*;

#[test]
fn sample_policy_exposes_shared_limits_and_decision() {
    assert_eq!(
        history_sample_limits(),
        HistorySampleLimits {
            interval: 300.0,
            change_threshold: 5.0,
        }
    );
    let previous = HistorySamplePoint { at: 1000.0, used: 25.0 };
    assert!(!history_should_record(
        HistorySamplePoint { at: 1001.0, used: 27.0 },
        Some(previous),
        false
    ));
    assert!(history_should_record(
        HistorySamplePoint { at: 1001.0, used: 27.0 },
        Some(previous),
        true
    ));
    assert!(history_should_record(previous, None, false));
}
