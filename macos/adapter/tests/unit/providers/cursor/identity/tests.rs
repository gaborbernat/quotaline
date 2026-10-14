use super::*;
use crate::mock_http::MockTransport;
use futures_util::FutureExt;

#[test]
fn identity_duration_bridge_exposes_shared_lifetimes() {
    assert_eq!(
        cursor_identity_durations(),
        CursorIdentityDurations {
            success: 3_600.0,
            failure: 300.0,
        }
    );
}

#[test]
fn fetcher_caches_profile_email_until_expiry() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().times(2).returning(|request, completion| {
        assert_eq!(request.url, super::super::request::PROFILE_URL);
        assert_eq!(request.operation, "cursor.me");
        assert_eq!(request.headers.get("Cookie"), Some(&"session=secret".into()));
        completion.succeed(br#"{"email":"account@example.com"}"#.to_vec());
    });
    let fetcher = CursorIdentityFetcher::new();
    let transport = Arc::new(transport);
    for now in [100.0, 101.0, 100.0 + identity::SUCCESS_TTL_SECONDS] {
        assert_eq!(
            futures_executor::block_on(fetcher.email("token".into(), "session=secret".into(), now, transport.clone(),)),
            Some("account@example.com".into())
        );
    }
}

#[test]
fn fetcher_caches_missing_email_for_failure_ttl() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().once().return_once(|_, completion| {
        completion.fail(crate::http::HttpErrorRecord::Network {
            detail: "offline".into(),
        });
    });
    let fetcher = CursorIdentityFetcher::new();
    let transport = Arc::new(transport);
    for now in [100.0, 101.0] {
        assert_eq!(
            futures_executor::block_on(fetcher.email("token".into(), "cookie".into(), now, transport.clone())),
            None
        );
    }
}

#[test]
fn fetcher_coalesces_concurrent_requests() {
    let completion = Arc::new(Mutex::new(None));
    let mut transport = MockTransport::new();
    let slot = completion.clone();
    transport.expect_start_get().once().return_once(move |_, callback| {
        *slot.lock().unwrap() = Some(callback);
    });
    let fetcher = CursorIdentityFetcher::new();
    let transport = Arc::new(transport);
    let mut first = Box::pin(fetcher.email("token".into(), "cookie".into(), 100.0, transport.clone()));
    let mut second = Box::pin(fetcher.email("token".into(), "cookie".into(), 100.0, transport));
    assert!(first.as_mut().now_or_never().is_none());
    assert!(second.as_mut().now_or_never().is_none());
    completion
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .succeed(br#"{"email":"shared@example.com"}"#.to_vec());
    assert_eq!(futures_executor::block_on(first), Some("shared@example.com".into()));
    assert_eq!(futures_executor::block_on(second), Some("shared@example.com".into()));
}

#[test]
fn cancelled_leader_releases_waiters() {
    let completion = Arc::new(Mutex::new(None));
    let mut transport = MockTransport::new();
    let slot = completion.clone();
    transport.expect_start_get().once().return_once(move |_, callback| {
        *slot.lock().unwrap() = Some(callback);
    });
    let fetcher = CursorIdentityFetcher::new();
    let transport = Arc::new(transport);
    let mut first = Box::pin(fetcher.email("token".into(), "cookie".into(), 100.0, transport.clone()));
    let mut second = Box::pin(fetcher.email("token".into(), "cookie".into(), 100.0, transport));
    assert!(first.as_mut().now_or_never().is_none());
    assert!(second.as_mut().now_or_never().is_none());
    drop(first);
    assert_eq!(futures_executor::block_on(second), None);
}

#[test]
fn newer_account_does_not_cache_older_result() {
    let completions = Arc::new(Mutex::new(HashMap::new()));
    let mut transport = MockTransport::new();
    let slots = completions.clone();
    transport
        .expect_start_get()
        .times(2)
        .returning(move |request, callback| {
            slots
                .lock()
                .unwrap()
                .insert(request.headers["Cookie"].clone(), callback);
        });
    let fetcher = CursorIdentityFetcher::new();
    let transport = Arc::new(transport);
    let mut older = Box::pin(fetcher.email("old".into(), "old-cookie".into(), 100.0, transport.clone()));
    let mut newer = Box::pin(fetcher.email("new".into(), "new-cookie".into(), 100.0, transport.clone()));
    assert!(older.as_mut().now_or_never().is_none());
    assert!(newer.as_mut().now_or_never().is_none());
    completions
        .lock()
        .unwrap()
        .remove("new-cookie")
        .unwrap()
        .succeed(br#"{"email":"new@example.com"}"#.to_vec());
    assert_eq!(futures_executor::block_on(newer), Some("new@example.com".into()));
    completions
        .lock()
        .unwrap()
        .remove("old-cookie")
        .unwrap()
        .succeed(br#"{"email":"old@example.com"}"#.to_vec());
    assert_eq!(futures_executor::block_on(older), Some("old@example.com".into()));
    assert_eq!(
        futures_executor::block_on(fetcher.email("new".into(), "new-cookie".into(), 101.0, transport.clone())),
        Some("new@example.com".into())
    );
}
