//! Checks sample writes, queries, retention, and account changes at the bridge.

use super::*;

#[test]
fn sample_queries_map_rows_and_usage_dates() {
    let store = RustHistoryStore::new(None, 60).unwrap();
    let start = at("2026-08-29T12:00:00Z");
    store
        .seed(vec![sample(start, 10.0), sample(start + 60.0, 20.0)])
        .unwrap();
    assert_eq!(store.samples(query(start, start + 60.0)).unwrap().len(), 2);
    assert_eq!(
        store
            .samples(StoredSampleQuery {
                rollup: Some(3_600.0),
                ..query(start, start + 60.0)
            })
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        store
            .last_usage_dates(vec!["claude:session".into()], start, start + 60.0)
            .unwrap()[0]
            .at,
        start + 60.0
    );
    assert_eq!(store.summaries().unwrap()[0].last_percent, 20.0);
    assert_eq!(
        store.earliest_sample(Some(vec!["claude:session".into()])).unwrap(),
        Some(start)
    );
    let stats = store.stats().unwrap();
    assert_eq!(stats.sample_count, 2);
    assert_eq!(stats.oldest, Some(start));
    assert_eq!(stats.newest, Some(start + 60.0));
    store.interrupt();
}

#[test]
fn snapshots_and_account_switches_use_the_shared_store() {
    let store = RustHistoryStore::new(None, 60).unwrap();
    let now = at("2026-08-29T12:00:00Z");
    let mut reading = crate::test_records::reading(
        "claude",
        vec![crate::test_records::window("session", 10.0, Some(now + 3_600.0))],
        now,
    );
    reading.provider.account_fingerprint = Some("first".into());
    assert_eq!(store.record_reading(reading, now).unwrap(), 1);
    store.activate_account("claude".into(), "second".into()).unwrap();
    assert_eq!(store.stats().unwrap().sample_count, 0);
    store.activate_account("claude".into(), "first".into()).unwrap();
    assert_eq!(store.stats().unwrap().sample_count, 1);
    assert_eq!(store.clear().unwrap(), 1);
}

#[test]
fn retention_bridge_clamps_and_removes_expired_samples() {
    let store = RustHistoryStore::new(None, 60).unwrap();
    let old = at("2026-01-01T12:00:00Z");
    let now = at("2026-08-29T12:00:00Z");
    store.seed(vec![sample(old, 10.0)]).unwrap();
    store.set_retention_days(365);
    assert_eq!(store.retention_days(), 365);
    let result = store.apply_retention_days(7, now).unwrap();
    assert_eq!(result.samples, 1);
    assert_eq!(result.analytics, 0);
    assert_eq!(store.stats().unwrap().sample_count, 0);
}
