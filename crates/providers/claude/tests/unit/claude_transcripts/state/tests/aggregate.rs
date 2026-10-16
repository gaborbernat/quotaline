//! Checks retained totals, duplicates, project reattribution, and pruning.

use super::*;

#[test]
fn ingest_creates_daily_models_projects_and_recent_usage() {
    let mut state = TranscriptState::default();
    let mut message = record("one", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    message.project = Some("directory:-work-repo".into());
    assert!(state.ingest(&message, "fallback"));
    assert!(!state.ingest(&message, "fallback"));
    assert_eq!(state.message_count(), 1);
    let analytics = state.analytics().unwrap();
    assert_eq!(analytics.first_day, "2026-10-09");
    assert_eq!(analytics.last_day, "2026-10-09");
    assert_eq!(
        analytics
            .points
            .iter()
            .find(|point| point.metric == "messages")
            .unwrap()
            .value,
        1.0
    );
    assert_eq!(
        analytics
            .points
            .iter()
            .find(|point| point.metric == "projectMessages")
            .unwrap()
            .value,
        1.0
    );
    assert_eq!(state.recent.values().next().unwrap().tokens, 5);
}

#[test]
fn transcript_snapshot_covers_project_metrics_only_after_a_complete_scan() {
    let mut state = TranscriptState::default();
    state.ingest(&record("one", "2026-10-09T12:00:00Z", "claude-sonnet-4-6"), "project");
    let complete = state.analytics().unwrap().snapshot(12.0, true);
    assert_eq!(complete.provider, "claude");
    assert_eq!(complete.fetched_at, 12.0);
    assert_eq!(complete.points.len(), state.analytics().unwrap().points.len());
    assert!(complete.credit_events.is_empty());
    assert_eq!(complete.account_fingerprint, None);
    assert_eq!(complete.covered_scopes.len(), 1);
    assert_eq!(complete.covered_scopes[0].metrics, ["projectCost", "projectMessages"]);
    assert_eq!(complete.covered_scopes[0].start_day, "2026-10-09");
    assert_eq!(complete.covered_scopes[0].end_day, "2026-10-09");
    assert!(
        state
            .analytics()
            .unwrap()
            .snapshot(12.0, false)
            .covered_scopes
            .is_empty()
    );
}

#[test]
fn duplicate_record_can_reassign_a_legacy_project_without_recounting_usage() {
    let mut state = TranscriptState::default();
    let mut message = record("one", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    let day = message.day.clone();
    state
        .seen_by_day
        .entry(day.clone())
        .or_default()
        .insert(message.id.clone());
    state.days.entry(day.clone()).or_default().projects.insert(
        "legacy:repo".into(),
        ProjectAggregate {
            cost: message.cost.unwrap(),
            messages: 1,
            priced: true,
        },
    );
    message.project = Some("directory:-work-repo".into());
    assert!(state.ingest(&message, "fallback"));
    assert_eq!(state.message_count(), 0);
    assert!(!state.days[&day].projects.contains_key("legacy:repo"));
    assert_eq!(state.days[&day].projects["directory:-work-repo"].messages, 1);
}

#[test]
fn ingest_tracks_unpriced_models_and_fallback_project() {
    let mut state = TranscriptState::default();
    let message = record("one", "2026-10-09T12:00:00Z", "claude-unknown-9");
    assert!(state.ingest(&message, "fallback"));
    assert_eq!(state.unpriced_models(), ["claude-unknown-9"]);
    assert_eq!(state.days["2026-10-09"].projects["fallback"].cost, 0.0);
    let legacy = state.analytics().unwrap();
    assert_eq!(
        legacy
            .points
            .iter()
            .find(|point| point.metric == "costUSD")
            .unwrap()
            .value,
        0.0
    );
    assert_eq!(
        legacy
            .points
            .iter()
            .find(|point| point.metric == "projectCost")
            .unwrap()
            .value,
        0.0
    );
    let analytics = state.analytics_without_unpriced_costs().unwrap();
    let metrics = analytics
        .points
        .iter()
        .map(|point| point.metric.as_str())
        .collect::<Vec<_>>();
    assert!(!metrics.contains(&"costUSD"));
    assert!(!metrics.contains(&"projectCost"));
}

#[test]
fn known_zero_cost_keeps_cost_series_visible() {
    let mut state = TranscriptState::default();
    let mut message = record("one", "2026-10-09T12:00:00Z", "claude-unknown-9");
    message.cost = Some(0.0);
    state.ingest(&message, "fallback");
    let points = state.analytics().unwrap().points;
    assert!(
        points
            .iter()
            .any(|point| point.metric == "costUSD" && point.value == 0.0)
    );
    assert!(
        points
            .iter()
            .any(|point| point.metric == "projectCost" && point.value == 0.0)
    );
}

#[test]
fn priced_project_remains_in_analytics_without_unpriced_costs() {
    let mut state = TranscriptState::default();
    let message = record("one", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    let cost = message.cost.unwrap();
    state.ingest(&message, "fallback");
    let points = state.analytics_without_unpriced_costs().unwrap().points;
    assert_eq!(
        points.iter().find(|point| point.metric == "projectCost").unwrap().value,
        cost
    );
}

#[test]
fn prune_removes_expired_days_and_compacts_old_recent_events() {
    let mut state = TranscriptState::default();
    let old = record("old", "2026-01-01T12:00:00Z", "claude-sonnet-4-6");
    let current = record("new", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    state.ingest(&old, "old");
    state.ingest(&current, "new");
    let now = current.timestamp as f64;
    assert!(state.prune(now, 60));
    assert_eq!(state.message_count(), 1);
    assert_eq!(state.recent.len(), 1);
    assert!(!state.prune(now, 60));
}

#[test]
fn empty_state_has_no_analytics() {
    assert_eq!(TranscriptState::default().analytics(), None);
}

#[test]
fn reattribution_keeps_remaining_legacy_project_messages() {
    let mut message = record("one", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    let checkpoint = serde_json::json!({
        "offsets": {}, "seenByDay": {"2026-10-09": [message.id]}, "projectSeenByDay": {},
        "days": {"2026-10-09": {"models": {}, "messages": 2, "sessions": [], "toolCalls": 0,
            "projects": {"legacy:repo": {"messages": 2, "cost": 1.0}}}},
        "recent": {}, "projectIdentityVersion": 2
    });
    let mut state = TranscriptState::from_checkpoint(checkpoint.to_string().as_bytes()).unwrap();
    message.project = Some("directory:-work-repo".into());
    assert!(state.ingest(&message, "fallback"));
    assert_eq!(state.message_count(), 2);
    let points = state.analytics().unwrap().points;
    let projects = points
        .into_iter()
        .filter(|point| point.metric == "projectMessages")
        .map(|point| (point.series, point.value))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        projects,
        BTreeMap::from([("directory:-work-repo".into(), 1.0), ("legacy:repo".into(), 1.0)])
    );
}

#[test]
fn empty_project_and_one_hour_cache_write_are_checkpointed() {
    let mut state = TranscriptState::default();
    let mut message = record("one", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    message.cache_write = 5;
    message.cache_write_one_hour = Some(3);
    assert!(state.ingest(&message, ""));
    let checkpoint: serde_json::Value = serde_json::from_slice(&state.checkpoint().unwrap()).unwrap();
    assert_eq!(
        checkpoint["days"]["2026-10-09"]["models"]["claude-sonnet-4-6"]["cacheWriteOneHour"],
        3
    );
    assert_eq!(checkpoint["days"]["2026-10-09"]["projects"][""]["messages"], 1);
}

#[test]
fn pruning_compacts_legacy_recent_events_without_changing_their_totals() {
    let at = 978_307_200.0;
    let checkpoint = serde_json::json!({
        "offsets": {}, "seenByDay": {},
        "days": {"2001-01-01": {"models": {}, "messages": 1, "sessions": [], "toolCalls": 0}},
        "recent": {"978307200": {"timestamp": 0.0, "tokens": 17, "cost": 2.5, "messages": 1}},
        "projectIdentityVersion": 2
    });
    let mut state = TranscriptState::from_checkpoint(checkpoint.to_string().as_bytes()).unwrap();
    assert!(state.prune(at, 60));
    assert_eq!(state.local_usage(at - 1.0, at - 1.0, at).unwrap().today_tokens, 17);
    assert!(!state.prune(at, 60));
}

#[test]
fn pruning_can_expire_recent_usage_while_retaining_daily_history() {
    let mut state = TranscriptState::default();
    let message = record("one", "2026-10-09T12:00:00Z", "claude-sonnet-4-6");
    assert!(state.ingest(&message, "repo"));
    let now = message.timestamp as f64 + 27.0 * 3_600.0;
    assert!(state.prune(now, 60));
    assert_eq!(state.message_count(), 1);
    assert_eq!(
        state
            .local_usage(now - 24.0 * 3_600.0, now - 24.0 * 3_600.0, now)
            .unwrap()
            .today_tokens,
        0
    );
}

#[test]
fn pruning_merges_two_old_recent_buckets_for_one_minute() {
    let at = 978_307_200.0;
    let checkpoint = serde_json::json!({
        "offsets": {}, "seenByDay": {},
        "days": {"2001-01-01": {"models": {}, "messages": 2, "sessions": [], "toolCalls": 0}},
        "recent": {
            "first": {"timestamp": 0.0, "tokens": 3, "cost": 0.5, "messages": 1},
            "second": {"timestamp": 30.0, "tokens": 4, "cost": 0.75, "messages": 1}
        },
        "projectIdentityVersion": 2
    });
    let mut state = TranscriptState::from_checkpoint(checkpoint.to_string().as_bytes()).unwrap();
    assert!(state.prune(at + 30.0, 60));
    let usage = state.local_usage(at - 1.0, at - 1.0, at + 30.0).unwrap();
    assert_eq!(usage.window_tokens, 7);
    assert_eq!(usage.today_cost, 1.25);
}
