//! macOS spend bridge cases.

use super::*;

#[test]
fn spend_bridge_returns_utc_days() {
    let days = spend_days(1_760_354_400.0);
    assert_eq!(days.today, "2025-10-13");
    assert_eq!(days.yesterday, "2025-10-12");
    assert_eq!(days.start, "2025-09-14");
}

#[test]
fn spend_bridge_groups_report_and_summary() {
    let rows = vec![SpendRowRecord {
        day: "2025-10-13".into(),
        provider: "claude".into(),
        model: "opus".into(),
        value: 3.0,
    }];
    let summary = spend_summary(rows.clone(), 1_760_354_400.0);
    let report = spend_report(rows, 1_760_354_400.0);
    assert_eq!(summary.today, 3.0);
    assert_eq!(report.total, summary);
    assert_eq!(report.by_provider["claude"], summary);
}
