//! Visible limit notice cases.

use super::*;

#[test]
fn full_visible_window_suppresses_only_its_limit_notice() {
    let full = BTreeSet::from(["session".into()]);
    assert!(!notice_visible(true, Some("session"), &full));
    assert!(notice_visible(true, Some("weekly"), &full));
    assert!(notice_visible(true, None, &full));
    assert!(notice_visible(false, Some("session"), &full));
}

#[test]
fn hidden_full_window_retains_its_limit_notice() {
    assert!(notice_visible(true, Some("session"), &BTreeSet::new()));
}
