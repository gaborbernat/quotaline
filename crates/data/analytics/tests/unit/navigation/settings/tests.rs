use super::*;

const NOW: f64 = 1_788_955_200.0;

fn action(value: &str) -> HistoryAction {
    serde_json::from_str(value).unwrap()
}

fn defaults() -> SavedHistory {
    SavedHistory {
        days: 1,
        rollup: "minute".into(),
        metric: "usage".into(),
        utc: false,
    }
}

fn session() -> HistorySession {
    from_saved(&defaults(), "America/Los_Angeles".into(), -7 * 3_600, NOW)
}

#[test]
fn saved_preferences_create_a_live_session_with_fallbacks() {
    let current = session();
    assert_eq!(current.range(), HistoryRange::Today);
    assert_eq!(current.rollup(), WindowRollup::Minute);
    assert_eq!(current.metric(), "usage");
    let mut stored = SavedHistory {
        days: 999,
        rollup: "invalid".into(),
        ..defaults()
    };
    assert_eq!(from_saved(&stored, "UTC".into(), 0, NOW).view(NOW).days, 1);
    stored = SavedHistory {
        days: 60,
        rollup: "day".into(),
        metric: "inputTokens".into(),
        utc: true,
    };
    let saved = from_saved(&stored, "UTC".into(), 0, NOW);
    assert_eq!(saved.range(), HistoryRange::TwoMonths);
    assert_eq!(saved.rollup(), WindowRollup::Day);
    assert_eq!(saved.metric(), "inputTokens");
    assert!(saved.utc());
}

#[test]
fn period_actions_choose_each_supported_range() {
    for (days, expected) in [
        (1, HistoryRange::Today),
        (7, HistoryRange::Week),
        (30, HistoryRange::Month),
        (60, HistoryRange::TwoMonths),
    ] {
        let mut session = session();
        action(&format!(r#"{{"kind":"period","days":{days}}}"#))
            .apply(&mut session, NOW)
            .unwrap();
        assert_eq!(session.range(), expected);
        assert!(!session.view(NOW).follow_now);
    }
    assert_eq!(
        action(r#"{"kind":"period","days":2}"#).apply(&mut session(), NOW),
        Err("Unknown history period")
    );
    let mut session = session();
    action(r#"{"kind":"period","days":7}"#)
        .apply(&mut session, NOW)
        .unwrap();
    action(r#"{"kind":"now"}"#).apply(&mut session, NOW).unwrap();
    assert!(session.view(NOW).follow_now);
    action(r#"{"kind":"load"}"#).apply(&mut session, NOW).unwrap();
    assert!(session.view(NOW).follow_now);
}

#[test]
fn rollup_metric_and_utc_actions_validate_their_inputs() {
    let mut session = session();
    for (name, expected) in [
        ("minute", WindowRollup::Minute),
        ("hour", WindowRollup::Hour),
        ("day", WindowRollup::Day),
    ] {
        action(&format!(r#"{{"kind":"rollup","rollup":"{name}"}}"#))
            .apply(&mut session, NOW)
            .unwrap();
        assert_eq!(session.rollup(), expected);
    }
    assert_eq!(
        action(r#"{"kind":"rollup","rollup":"week"}"#).apply(&mut session, NOW),
        Err("Unknown history rollup")
    );
    action(r#"{"kind":"metric","metric":"inputTokens"}"#)
        .apply(&mut session, NOW)
        .unwrap();
    assert_eq!(session.metric(), "inputTokens");
    action(r#"{"kind":"metric","metric":"usage"}"#)
        .apply(&mut session, NOW)
        .unwrap();
    assert_eq!(session.metric(), "usage");
    assert_eq!(
        action(r#"{"kind":"metric","metric":"unknown"}"#).apply(&mut session, NOW),
        Err("Unknown metric")
    );
    action(r#"{"kind":"utc","utc":true}"#).apply(&mut session, NOW).unwrap();
    assert!(session.utc());
    action(r#"{"kind":"utc","utc":false}"#)
        .apply(&mut session, NOW)
        .unwrap();
    assert!(!session.utc());
}

#[test]
fn custom_and_page_actions_validate_dates_and_keep_picker_values() {
    let mut session = session();
    for (raw, error) in [
        (
            r#"{"kind":"custom","from":"2026-9-1","to":"2026-09-02"}"#,
            "Enter dates as YYYY-MM-DD",
        ),
        (
            r#"{"kind":"custom","from":"2026-09/01","to":"2026-09-02"}"#,
            "Enter dates as YYYY-MM-DD",
        ),
        (
            r#"{"kind":"custom","from":"2026/09-01","to":"2026-09-02"}"#,
            "Enter dates as YYYY-MM-DD",
        ),
        (
            r#"{"kind":"custom","from":"2026-09-01","to":"2026-9-2"}"#,
            "Enter dates as YYYY-MM-DD",
        ),
        (
            r#"{"kind":"custom","from":"2026-09-31","to":"2026-10-02"}"#,
            "Choose a valid date range",
        ),
        (
            r#"{"kind":"custom","from":"2026-09-08","to":"2026-09-07"}"#,
            "Choose a valid date range",
        ),
    ] {
        assert_eq!(action(raw).apply(&mut session, NOW), Err(error));
    }
    action(r#"{"kind":"custom","from":"2026-09-06","to":"2026-09-07"}"#)
        .apply(&mut session, NOW)
        .unwrap();
    assert_eq!(session.view(NOW).days, 0);
    action(r#"{"kind":"page","forward":false}"#)
        .apply(&mut session, NOW)
        .unwrap();
    assert_eq!(session.view(NOW).custom_from.as_deref(), Some("2026-09-06"));
    action(r#"{"kind":"page","forward":true}"#)
        .apply(&mut session, NOW)
        .unwrap();
}

#[test]
fn sync_persists_only_changed_preferences() {
    let mut session = session();
    let mut saved = defaults();
    assert!(!sync_saved(&session, &mut saved));
    action(r#"{"kind":"period","days":7}"#)
        .apply(&mut session, NOW)
        .unwrap();
    action(r#"{"kind":"rollup","rollup":"hour"}"#)
        .apply(&mut session, NOW)
        .unwrap();
    action(r#"{"kind":"metric","metric":"inputTokens"}"#)
        .apply(&mut session, NOW)
        .unwrap();
    action(r#"{"kind":"utc","utc":true}"#).apply(&mut session, NOW).unwrap();
    assert!(sync_saved(&session, &mut saved));
    assert_eq!(saved.days, 7);
    assert_eq!(saved.rollup, "hour");
    assert_eq!(saved.metric, "inputTokens");
    assert!(saved.utc);
    assert!(!sync_saved(&session, &mut saved));
    action(r#"{"kind":"rollup","rollup":"day"}"#)
        .apply(&mut session, NOW)
        .unwrap();
    assert!(sync_saved(&session, &mut saved));
    assert_eq!(saved.rollup, "day");
    action(r#"{"kind":"custom","from":"2026-09-06","to":"2026-09-07"}"#)
        .apply(&mut session, NOW)
        .unwrap();
    assert!(!sync_saved(&session, &mut saved));
    assert_eq!(saved.days, 7);
}
