use super::*;

#[test]
fn cached_window_chart_queries_and_invalidates_real_history() {
    let store = RustHistoryStore::new(None, 60).unwrap();
    let now = at("2025-01-01T12:00:00Z");
    store.seed(vec![sample(now - 600.0, 20.0)]).unwrap();
    let request = WindowChartRequest {
        keys: vec!["claude:session".into()],
        visible: vec!["claude:session".into()],
        start: now - 900.0,
        end: now,
        now,
        rollup: WindowRollup::Minute,
        stacked: false,
        time_zone: "UTC".into(),
        fallback_offset: 0,
        includes_end: true,
    };
    let first = store.cached_window_chart(request.clone()).unwrap();
    store.seed(vec![sample(now - 60.0, 80.0)]).unwrap();
    assert_eq!(store.cached_window_chart(request.clone()).unwrap(), first);
    store.clear_chart_cache();
    assert_ne!(store.cached_window_chart(request).unwrap(), first);
}

#[test]
fn cached_analytics_chart_projects_without_returning_raw_rows() {
    let store = RustHistoryStore::new(None, 60).unwrap();
    let now = at("2025-01-01T12:00:00Z");
    store
        .record_analytics(
            "codex".into(),
            None,
            now,
            vec![Point {
                provider: "codex".into(),
                day: "2025-01-01".into(),
                metric: "turns".into(),
                series: "terminal".into(),
                value: 3.0,
            }],
            vec![],
        )
        .unwrap();
    for (hidden, visible) in [
        ("malformed", true),
        ("analytics:codex", true),
        ("analytics:codex:terminal", false),
    ] {
        let result = store
            .cached_analytics_chart(StoredAnalyticsChartQuery {
                metric: "turns".into(),
                providers: vec!["codex".into()],
                start_day: "2025-01-01".into(),
                before_day: "2025-01-03".into(),
                hidden: vec![hidden.into()],
                stacked: false,
            })
            .unwrap()
            .unwrap();
        assert_eq!(
            (
                result.series[0].provider.as_str(),
                result.series[0].visible,
                result.summary_total
            ),
            ("codex", visible, 3.0)
        );
    }
}

#[test]
fn cached_charts_translate_database_errors() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("history.sqlite3");
    let store = RustHistoryStore::new(Some(path.display().to_string()), 60).unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("DROP TABLE samples; DROP TABLE analytics").unwrap();
    let now = at("2025-01-01T12:00:00Z");
    assert!(matches!(
        store.cached_window_chart(WindowChartRequest {
            keys: vec!["claude:session".into()],
            visible: vec!["claude:session".into()],
            start: now - 900.0,
            end: now,
            now,
            rollup: WindowRollup::Minute,
            stacked: false,
            time_zone: "UTC".into(),
            fallback_offset: 0,
            includes_end: true,
        }),
        Err(HistoryStoreError::Database { .. })
    ));
    assert!(matches!(
        store.cached_analytics_chart(StoredAnalyticsChartQuery {
            metric: "turns".into(),
            providers: vec!["codex".into()],
            start_day: "2025-01-01".into(),
            before_day: "2025-01-03".into(),
            hidden: vec![],
            stacked: false,
        }),
        Err(HistoryStoreError::Database { .. })
    ));
}
