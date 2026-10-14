//! Keeps each account's analytics apart when a provider switches accounts.

use super::*;

fn turns(value: f64) -> AnalyticsPoint {
    AnalyticsPoint {
        provider: "codex".into(),
        day: "2026-08-29".into(),
        metric: "turns".into(),
        series: "total".into(),
        value,
    }
}

#[test]
fn analytics_switch_to_the_account_they_belong_to() {
    let store = Store::open(None, 60).unwrap();
    let now = at("2026-08-29T12:00:00Z");
    store
        .record_analytics("codex", Some("first"), now, &[turns(3.0)], &[])
        .unwrap();
    store
        .record_analytics("codex", Some("second"), now, &[turns(9.0)], &[])
        .unwrap();
    assert_eq!(store.active_account("codex").unwrap().as_deref(), Some("second"));
    assert_eq!(
        store.analytics("codex", "2026-08-29", "2026-08-29").unwrap(),
        [turns(9.0)]
    );
}

#[test]
fn analytics_activate_an_account_when_none_is_active() {
    let store = Store::open(None, 60).unwrap();
    let now = at("2026-08-29T12:00:00Z");
    store
        .record_analytics("codex", Some("first"), now, &[turns(3.0)], &[])
        .unwrap();
    store.activate_account("codex", "second").unwrap();
    assert!(store.analytics("codex", "2026-08-29", "2026-08-29").unwrap().is_empty());
    store.activate_account("codex", "first").unwrap();
    assert_eq!(
        store.analytics("codex", "2026-08-29", "2026-08-29").unwrap(),
        [turns(3.0)]
    );
}
