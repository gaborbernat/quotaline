//! Checks that settings round-trip through the shared history session.

use super::*;

const NOW: f64 = 1_788_955_200.0;

#[test]
fn session_starts_from_the_saved_history_choices() {
    let settings = Settings {
        history_days: 7,
        history_rollup: "hour".into(),
        history_metric: "inputTokens".into(),
        history_utc: true,
        ..Settings::default()
    };
    let session = history_session_from_settings(&settings, "UTC".into(), 0, NOW);
    assert_eq!((session.metric(), session.utc()), ("inputTokens", true));
}

#[test]
fn sync_writes_changed_choices_back_to_settings() {
    let source = Settings {
        history_days: 30,
        history_rollup: "day".into(),
        ..Settings::default()
    };
    let session = history_session_from_settings(&source, "UTC".into(), 0, NOW);
    let mut settings = Settings::default();
    assert!(sync_history_settings(&session, &mut settings));
    assert_eq!((settings.history_days, settings.history_rollup.as_str()), (30, "day"));
    assert!(!sync_history_settings(&session, &mut settings));
}
