//! Seeds demo previews from the shared demo generators.

use super::*;

#[test]
fn demo_usage_spans_the_demo_period_for_each_requested_provider() {
    let store = Store::open(None, 60).unwrap();
    let now = at("2026-10-12T14:00:00Z");
    store
        .seed_demo(&["claude".into(), "cursor".into()], now, false)
        .unwrap();
    let summaries = store.summaries().unwrap();
    assert!(summaries.iter().any(|row| row.key.starts_with("claude:")));
    assert!(summaries.iter().any(|row| row.key.starts_with("cursor:")));
    assert!(
        summaries
            .iter()
            .all(|row| row.key.starts_with("claude:") || row.key.starts_with("cursor:"))
    );
    let earliest = store.earliest_sample(None).unwrap().unwrap();
    assert_eq!(earliest, now - f64::from(DEMO_DAYS) * 86_400.0);
}

#[test]
fn demo_usage_stays_on_utc_half_hours() {
    let store = Store::open(None, 60).unwrap();
    let now = at("2026-10-12T14:10:00Z");
    store.seed_demo(&["claude".into()], now, false).unwrap();
    let rows = store.samples(query(0.0, now, None)).unwrap();
    assert!(rows.iter().all(|row| row.at % 1800.0 == 0.0));
    assert_eq!(rows.last().unwrap().at, at("2026-10-12T14:00:00Z"));
}

#[test]
fn demo_analytics_cover_the_retained_demo_days() {
    let store = Store::open(None, 60).unwrap();
    let now = at("2026-10-12T14:00:00Z");
    store.seed_demo(&["codex".into()], now, false).unwrap();
    let rows = store.analytics("codex", "2026-09-01", "2026-10-12").unwrap();
    assert_eq!(rows.first().unwrap().day, "2026-09-13");
    assert_eq!(rows.last().unwrap().day, "2026-10-12");
}

#[test]
fn demo_resets_are_stored_as_times() {
    let store = Store::open(None, 60).unwrap();
    let now = at("2026-10-12T14:00:00Z");
    store.seed_demo(&["claude".into()], now, false).unwrap();
    let rows = store.samples(query(now - 1800.0, now, None)).unwrap();
    assert!(
        rows.iter()
            .any(|row| row.reset_at.is_some_and(|reset| reset > now - 1800.0))
    );
}
