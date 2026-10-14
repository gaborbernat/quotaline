//! Checks the macOS history navigation object through its public actions.

use super::*;

fn navigation(range: HistoryRange) -> Arc<RustHistoryNavigation> {
    RustHistoryNavigation::new(HistoryNavigation {
        range,
        follow_now: true,
        anchor_end: None,
        custom_start: 0.0,
        custom_end: 86_400.0,
        return_to_live_at: None,
        daily_utc: false,
        time_zone: "UTC".into(),
        fallback_offset: 0,
    })
}

#[test]
fn reading_step_bridge_uses_shared_selection() {
    assert_eq!(super::super::history_step_reading(3, None, 1), Some(0));
    assert_eq!(super::super::history_step_reading(3, Some(1), -1), Some(0));
    assert_eq!(super::super::history_step_reading(0, None, 1), None);
}

#[test]
fn navigation_state_owns_dates_and_follow_selection() {
    let state = navigation(HistoryRange::Today);
    assert!(state.follow_now());
    assert_eq!(state.custom_start(), 0.0);
    assert_eq!(state.custom_end(), 86_400.0);
    assert_eq!(state.anchor_end(), None);
    state.set_follow_now(false);
    state.set_custom_start(12.0);
    state.set_custom_end(22.0);
    assert_eq!(state.snapshot().custom_start, 12.0);
    assert_eq!(state.snapshot().custom_end, 22.0);
    assert!(!state.snapshot().follow_now);
}

#[test]
fn navigation_state_syncs_display_mode() {
    let state = navigation(HistoryRange::Today);
    state.sync_mode(HistoryRange::Week, true, "America/Los_Angeles".into(), -25_200);
    let current = state.snapshot();
    assert_eq!(current.range, HistoryRange::Week);
    assert!(current.daily_utc);
    assert_eq!(current.time_zone, "America/Los_Angeles");
    assert_eq!(current.fallback_offset, -25_200);
}

#[test]
fn fixed_ranges_page_from_shared_boundaries() {
    for (range, start, previous_end) in [
        (HistoryRange::Today, 172_800.0, None),
        (HistoryRange::Week, -432_000.0, Some(-432_000.0)),
        (HistoryRange::Month, -2_419_200.0, Some(-2_419_200.0)),
        (HistoryRange::TwoMonths, -5_011_200.0, Some(-5_011_200.0)),
    ] {
        let state = navigation(range);
        assert_eq!(state.viewport(172_800.0), HistoryViewport { start, end: 172_800.0 });
        state.page(172_800.0, false);
        let paged = state.snapshot();
        assert_eq!(paged.follow_now, previous_end.is_none());
        assert_eq!(paged.anchor_end, previous_end);
        assert_eq!(paged.return_to_live_at, previous_end.map(|_| 172_800.0));
    }
}

#[test]
fn custom_range_pages_and_returns_to_live() {
    let state = navigation(HistoryRange::Custom);
    assert_eq!(
        state.viewport(172_800.0),
        HistoryViewport {
            start: 0.0,
            end: 172_800.0
        }
    );
    state.page(172_800.0, false);
    assert_eq!(state.custom_start(), -172_800.0);
    assert_eq!(state.custom_end(), 0.0);
    assert_eq!(state.anchor_end(), Some(0.0));
    state.page(172_811.0, true);
    assert!(state.follow_now());
    assert_eq!(state.custom_end(), 172_811.0);
    assert_eq!(state.snapshot().return_to_live_at, None);
}

#[test]
fn selected_week_returns_to_live_after_paging_delay() {
    let state = navigation(HistoryRange::Week);
    state.select_period(Some(HistoryRange::Week), WindowRollup::Hour, 172_800.0);
    state.page(172_800.0, false);
    state.page(172_811.0, true);
    assert!(state.follow_now());
}

#[test]
fn period_and_rollup_actions_apply_shared_choices() {
    for (rollup, expected) in [
        (WindowRollup::Minute, WindowRollup::Minute),
        (WindowRollup::Hour, WindowRollup::Hour),
        (WindowRollup::Day, WindowRollup::Hour),
    ] {
        let state = navigation(HistoryRange::Week);
        assert_eq!(
            state.select_period(Some(HistoryRange::Today), rollup, 172_800.0),
            expected
        );
        assert_eq!(state.snapshot().range, HistoryRange::Today);
    }
    let state = navigation(HistoryRange::Week);
    assert_eq!(
        state.select_period(None, WindowRollup::Day, 172_800.0),
        WindowRollup::Day
    );
    assert!(state.follow_now());
    let state = navigation(HistoryRange::Today);
    assert_eq!(state.select_rollup(WindowRollup::Day), WindowRollup::Day);
    assert_eq!(state.snapshot().range, HistoryRange::Week);
}

#[test]
fn custom_date_pause_and_reset_actions_apply_shared_policy() {
    let state = navigation(HistoryRange::Week);
    state.select_custom_start(172_800.0, 200_000.0);
    assert_eq!(state.snapshot().range, HistoryRange::Custom);
    assert_eq!(state.custom_start(), 172_740.0);
    let state = navigation(HistoryRange::Week);
    state.select_custom_end(172_800.0, -500_000.0);
    assert_eq!(state.snapshot().range, HistoryRange::Custom);
    assert_eq!(state.custom_end(), state.custom_start() + 60.0);
    let state = navigation(HistoryRange::Week);
    state.pause(172_800.0);
    assert_eq!(state.anchor_end(), Some(172_800.0));
    assert!(!state.follow_now());
    state.reset(172_800.0);
    assert!(state.follow_now());
    assert_eq!(state.anchor_end(), None);
    assert_eq!(state.custom_start(), 86_400.0);
}
