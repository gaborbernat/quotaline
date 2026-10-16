//! Checks exact minute boundaries and legacy recent-event fallback.

use super::*;

#[test]
fn quota_window_uses_reset_when_present_and_now_otherwise() {
    let mut state = TranscriptState::default();
    let message = record("one", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    state.ingest(&message, "repo");
    let now = message.timestamp as f64 + 3_600.0;
    assert_eq!(
        state
            .local_usage_for_window(None, 3_600.0, 0.0, now)
            .unwrap()
            .window_tokens,
        5
    );
    assert_eq!(
        state
            .local_usage_for_window(Some(now + 1_800.0), 3_600.0, 0.0, now)
            .unwrap()
            .window_tokens,
        0
    );
}

#[test]
fn local_usage_sums_window_and_today_independently() {
    let mut state = TranscriptState::default();
    let first = record("one", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    let second = record("two", "2026-10-09T12:00:30Z", "claude-sonnet-4-6");
    state.ingest(&first, "repo");
    state.ingest(&second, "repo");
    let now = second.timestamp as f64;
    let usage = state.local_usage(now - 15.0, now - 40.0, now).unwrap();
    assert_eq!(usage.window_tokens, 5);
    assert_eq!(usage.today_tokens, 10);
    assert_eq!(usage.today_messages, 2);
    assert!(usage.cost_per_hour > 0.0);
}

#[test]
fn recent_usage_ignores_events_after_now() {
    let mut state = TranscriptState::default();
    let first = record("one", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    let second = record("two", "2026-10-09T12:00:30Z", "claude-sonnet-4-6");
    state.ingest(&first, "repo");
    state.ingest(&second, "repo");
    let usage = state
        .local_usage(
            first.timestamp as f64 - 1.0,
            first.timestamp as f64 - 1.0,
            first.timestamp as f64,
        )
        .unwrap();
    assert_eq!(usage.window_tokens, 5);
    assert_eq!(usage.today_messages, 1);
}

#[test]
fn legacy_checkpoint_recent_totals_use_the_recorded_timestamp() {
    let at = 978_307_200.0;
    let data = serde_json::json!({
        "offsets": {}, "seenByDay": {},
        "days": {"2001-01-01": {"models": {}, "messages": 1, "sessions": [], "toolCalls": 0}},
        "recent": {"978307200": {"timestamp": 0.0, "tokens": 17, "cost": 2.5, "messages": 1}}
    });
    let state = TranscriptState::from_checkpoint(data.to_string().as_bytes()).unwrap();
    let usage = state.local_usage(at - 1.0, at - 1.0, at).unwrap();
    assert_eq!(usage.window_tokens, 17);
    assert_eq!(usage.today_cost, 2.5);
}

#[test]
fn empty_state_has_no_local_usage() {
    assert_eq!(TranscriptState::default().local_usage(0.0, 0.0, 0.0), None);
}

#[test]
fn usage_skips_old_minutes_and_sums_multiple_current_minutes() {
    let mut state = TranscriptState::default();
    let old = record("old", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    let first = record("first", "2026-10-09T12:01:00Z", "claude-sonnet-4-6");
    let second = record("second", "2026-10-09T12:02:00Z", "claude-sonnet-4-6");
    for message in [&old, &first, &second] {
        assert!(state.ingest(message, "repo"));
    }
    let now = second.timestamp as f64;
    let usage = state
        .local_usage(old.timestamp as f64 + 45.0, first.timestamp as f64 + 30.0, now)
        .unwrap();
    assert_eq!(usage.window_tokens, 10);
    assert_eq!(usage.today_tokens, 5);
    assert_eq!(usage.today_messages, 1);
    assert!(usage.cost_per_hour > 0.0);
}

#[test]
fn usage_checks_each_event_when_minute_straddles_window_and_now() {
    let mut state = TranscriptState::default();
    for (id, second) in [("old", 0), ("first", 20), ("second", 25), ("future", 45)] {
        let message = record(id, &format!("2026-10-09T12:00:{second:02}Z"), "claude-sonnet-4-6");
        assert!(state.ingest(&message, "repo"));
    }
    let first = record("first", "2026-10-09T12:00:20Z", "claude-sonnet-4-6");
    let now = first.timestamp as f64 + 10.0;
    let usage = state.local_usage(now - 15.0, now - 15.0, now).unwrap();
    assert_eq!(usage.window_tokens, 10);
    assert_eq!(usage.today_messages, 2);
    assert!(usage.cost_per_hour > 0.0);
}

#[test]
fn usage_with_no_records_in_the_requested_window_has_no_rate() {
    let mut state = TranscriptState::default();
    let message = record("old", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    assert!(state.ingest(&message, "repo"));
    let now = message.timestamp as f64 + 60.0;
    let usage = state.local_usage(now + 1.0, now + 1.0, now).unwrap();
    assert_eq!(usage.window_tokens, 0);
    assert_eq!(usage.today_tokens, 0);
    assert_eq!(usage.cost_per_hour, 0.0);
}

#[test]
fn usage_does_not_include_future_minutes() {
    let mut state = TranscriptState::default();
    let message = record("future", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    assert!(state.ingest(&message, "repo"));
    let now = message.timestamp as f64 - 30.0;
    let usage = state.local_usage(now - 15.0, now - 15.0, now).unwrap();
    assert_eq!(usage.window_tokens, 0);
    assert_eq!(usage.today_tokens, 0);
    assert_eq!(usage.cost_per_hour, 0.0);
}
