use super::{HistoryNavigation, HistoryRange, NavigationState};

fn navigation() -> NavigationState {
    NavigationState::new(HistoryNavigation {
        range: HistoryRange::Week,
        follow_now: true,
        anchor_end: None,
        custom_start: 200.0,
        custom_end: 500.0,
        return_to_live_at: None,
        daily_utc: false,
        time_zone: "UTC".into(),
        fallback_offset: 0,
    })
}

#[test]
fn setters_update_the_navigation_snapshot() {
    let mut navigation = navigation();
    navigation.set_follow_now(false);
    navigation.set_custom_start(300.0);
    navigation.set_custom_end(600.0);

    assert!(!navigation.follow_now());
    assert_eq!(navigation.custom_start(), 300.0);
    assert_eq!(navigation.custom_end(), 600.0);
    assert_eq!(navigation.anchor_end(), None);
    assert_eq!(navigation.snapshot().custom_end, 600.0);
}

#[test]
fn custom_bound_selection_keeps_a_minute_between_dates() {
    let mut navigation = navigation();
    navigation.select_custom_start(1_000.0, 990.0);
    assert_eq!(navigation.snapshot().custom_start, 940.0);
    navigation.select_custom_end(1_000.0, 920.0);
    assert_eq!(navigation.snapshot().custom_end, 1_000.0);
}

#[test]
fn pause_and_reset_change_following_behavior() {
    let mut navigation = navigation();
    navigation.pause(1_000.0);
    assert!(!navigation.follow_now());
    assert_eq!(navigation.anchor_end(), Some(1_000.0));
    navigation.reset(1_100.0);
    assert!(navigation.follow_now());
    assert_eq!(navigation.anchor_end(), None);
}
