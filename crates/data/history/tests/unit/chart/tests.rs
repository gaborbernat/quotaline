use super::*;

fn sample(at: f64, used: f64) -> history::SampleWrite {
    history::SampleWrite {
        sample: history::Sample {
            at,
            key: "claude:session".into(),
            used,
            reset_at: None,
        },
        label: "Session".into(),
    }
}

fn window_request(start: f64, end: f64, rollup: WindowRollup) -> WindowChartRequest {
    WindowChartRequest {
        keys: vec!["claude:session".into()],
        visible: vec!["claude:session".into()],
        start,
        end,
        now: end,
        rollup,
        stacked: false,
        time_zone: "UTC".into(),
        fallback_offset: 0,
        includes_end: true,
    }
}

#[test]
fn window_queries_keep_rows_until_cleared() {
    let store = Arc::new(Store::open(None, 60).unwrap());
    let at = 1_700_000_000.0;
    store.seed(&[sample(at, 20.0)]).unwrap();
    let session = HistoryChartSession::new(store.clone(), 24_000);
    let request = window_request(at - 60.0, at + 900.0, WindowRollup::Minute);
    let first = session.window(request.clone()).unwrap();
    store.seed(&[sample(at + 600.0, 80.0)]).unwrap();
    assert_eq!(session.window(request.clone()).unwrap(), first);
    session.clear();
    let refreshed = session.window(request).unwrap();
    assert_ne!(refreshed, first);
    assert_eq!(refreshed.series[0].summary_value, 80.0);
}

#[test]
fn window_cache_eviction_preserves_recent_queries() {
    let store = Arc::new(Store::open(None, 60).unwrap());
    let at = 1_700_000_000.0;
    store.seed(&[sample(at, 20.0)]).unwrap();
    let session = HistoryChartSession::new(store.clone(), 24_000);
    let first_request = window_request(at - 60.0, at + 900.0, WindowRollup::Minute);
    let first = session.window(first_request.clone()).unwrap();
    session
        .window(window_request(at - 60.0, at + 900.0, WindowRollup::Hour))
        .unwrap();
    session
        .window(window_request(at - 60.0, at + 900.0, WindowRollup::Day))
        .unwrap();
    store.seed(&[sample(at + 600.0, 80.0)]).unwrap();
    assert_ne!(session.window(first_request).unwrap(), first);
}

#[test]
fn window_cache_evicts_oversized_queries() {
    let store = Arc::new(Store::open(None, 60).unwrap());
    let at = 1_700_000_000.0;
    store.seed(&[sample(at, 20.0), sample(at + 600.0, 40.0)]).unwrap();
    let session = HistoryChartSession::new(store.clone(), 1);
    let request = window_request(at - 60.0, at + 1_800.0, WindowRollup::Minute);
    let first = session.window(request.clone()).unwrap();
    store.seed(&[sample(at + 1_200.0, 80.0)]).unwrap();
    assert_ne!(session.window(request).unwrap(), first);
}

#[test]
fn analytics_queries_keep_rows_until_cleared() {
    let store = Arc::new(Store::open(None, 60).unwrap());
    let point = history::AnalyticsPoint {
        provider: "codex".into(),
        day: "2025-01-01".into(),
        metric: "turns".into(),
        series: "terminal".into(),
        value: 3.0,
    };
    store
        .record_analytics("codex", None, 1_735_689_600.0, std::slice::from_ref(&point), &[])
        .unwrap();
    let session = HistoryChartSession::new(store.clone(), 24_000);
    let query = AnalyticsQuery {
        metric: "turns".into(),
        providers: vec!["codex".into()],
        start_day: "2025-01-01".into(),
        before_day: "2025-01-03".into(),
    };
    let first = session.analytics(query.clone(), &[], false).unwrap();
    store
        .record_analytics(
            "codex",
            None,
            1_735_689_600.0,
            &[history::AnalyticsPoint { value: 8.0, ..point }],
            &[],
        )
        .unwrap();
    assert_eq!(session.analytics(query.clone(), &[], false).unwrap(), first);
    session.clear();
    let refreshed = session.analytics(query, &[], false).unwrap();
    assert_ne!(refreshed, first);
    assert_eq!(refreshed.unwrap().summary_total, 8.0);
}
