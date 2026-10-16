//! Checks analytics writes and filtering through the Swift boundary.

use super::*;

#[test]
fn analytics_queries_map_points_and_scopes() {
    let store = RustHistoryStore::new(None, 60).unwrap();
    let now = at("2026-08-29T12:00:00Z");
    let point = Point {
        provider: "codex".into(),
        day: "2026-08-29".into(),
        metric: "turns".into(),
        series: "total".into(),
        value: 3.0,
    };
    assert_eq!(
        store
            .record_analytics(
                "codex".into(),
                None,
                now,
                vec![point.clone()],
                vec![CoverageScope {
                    start_day: "2026-08-29".into(),
                    end_day: "2026-08-29".into(),
                    metrics: vec!["costUSD".into()],
                }]
            )
            .unwrap(),
        1
    );
    assert_eq!(
        store
            .analytics("codex".into(), "2026-08-01".into(), "2026-08-31".into())
            .unwrap()
            .as_slice(),
        std::slice::from_ref(&point)
    );
    assert_eq!(
        store
            .analytics_rows(
                "turns".into(),
                vec!["codex".into()],
                "2026-08-01".into(),
                "2026-08-31".into(),
                true
            )
            .unwrap(),
        [point]
    );
    assert_eq!(
        store.earliest_analytics("turns".into(), vec!["codex".into()]).unwrap(),
        Some(at("2026-08-29T00:00:00Z"))
    );
    assert_eq!(store.stats().unwrap().analytics_count, 1);
}
