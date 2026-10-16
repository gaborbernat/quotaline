use super::*;
use crate::http::{HttpErrorRecord, NativeHttpCompletion, NativeHttpTransport};
use crate::mock_http::MockTransport;
use futures_util::FutureExt;
use quotaline_codex::request;
use std::sync::Arc;

fn credits(available: u64) -> ResetCredits {
    ResetCredits {
        available,
        applicable: available,
        total_earned: None,
        purchase_eligible: None,
        credits: None,
    }
}

async fn lookup(
    cache: &CodexResetCreditCache,
    now: f64,
    reported: Option<ResetCredits>,
    transport: Arc<dyn NativeHttpTransport>,
) -> Resolved {
    let request = fetch::Account {
        base: request::BASE_URL,
        token: "secret",
        account_id: Some("account-id"),
    };
    let lookup = Lookup {
        account: "account",
        reported,
        now,
        home: "",
    };
    cache.resolve(lookup, &NativeTransport(transport), &request).await
}

#[test]
fn complete_inline_credits_skip_supplemental_request() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().never();
    let cache = CodexResetCreditCache::new();
    let complete = ResetCredits {
        total_earned: Some(2),
        purchase_eligible: Some(false),
        ..credits(2)
    };
    let result = futures_executor::block_on(lookup(&cache, 100.0, Some(complete.clone()), Arc::new(transport)));
    assert_eq!(result, (Some(complete), None));
}

#[test]
fn supplemental_request_reuses_and_expires_success() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().times(2).returning(|request, completion| {
        assert_eq!(
            request.url,
            "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits"
        );
        assert_eq!(request.operation, "codex.reset-credits");
        assert_eq!(request.headers.get("Authorization"), Some(&"Bearer secret".into()));
        assert_eq!(request.headers.get("ChatGPT-Account-Id"), Some(&"account-id".into()));
        completion.succeed(br#"{"available_count":2,"applicable_available_count":2}"#.to_vec());
    });
    let cache = CodexResetCreditCache::new();
    let transport: Arc<dyn NativeHttpTransport> = Arc::new(transport);
    for now in [100.0, 101.0, 100.0 + codex_reset_credit_success_ttl()] {
        assert_eq!(
            futures_executor::block_on(lookup(&cache, now, None, transport.clone())).0,
            Some(credits(2))
        );
    }
}

#[test]
fn failed_request_reuses_stale_credits_until_retry() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().once().return_once(|_, completion| {
        completion.succeed(br#"{"available_count":2,"applicable_available_count":2}"#.to_vec());
    });
    transport.expect_start_get().once().return_once(|_, completion| {
        completion.fail(HttpErrorRecord::Network {
            detail: "offline".into(),
        });
    });
    let cache = CodexResetCreditCache::new();
    let transport: Arc<dyn NativeHttpTransport> = Arc::new(transport);
    assert_eq!(
        futures_executor::block_on(lookup(&cache, 0.0, None, transport.clone())).0,
        Some(credits(2))
    );
    for now in [codex_reset_credit_success_ttl(), codex_reset_credit_success_ttl() + 1.0] {
        assert_eq!(
            futures_executor::block_on(lookup(&cache, now, None, transport.clone())),
            (
                Some(credits(2)),
                Some("Reset credits unavailable: Network error: offline".into())
            )
        );
    }
}

#[test]
fn malformed_response_is_negative_cached() {
    let mut transport = MockTransport::new();
    transport
        .expect_start_get()
        .times(2)
        .returning(|_, completion| completion.succeed(b"{".to_vec()));
    let cache = CodexResetCreditCache::new();
    let transport: Arc<dyn NativeHttpTransport> = Arc::new(transport);
    for now in [100.0, 101.0, 100.0 + codex_reset_credit_failure_ttl()] {
        assert_eq!(
            futures_executor::block_on(lookup(&cache, now, None, transport.clone()))
                .1
                .as_deref(),
            Some("Reset credits unavailable: Unexpected response: codex.reset-credits: invalid payload")
        );
    }
}

#[test]
fn concurrent_requests_share_a_supplemental_fetch() {
    let completion = Arc::new(std::sync::Mutex::new(None));
    let mut transport = MockTransport::new();
    let slot = completion.clone();
    transport.expect_start_get().once().return_once(move |_, callback| {
        *slot.lock().unwrap() = Some(callback);
    });
    let cache = CodexResetCreditCache::new();
    let transport: Arc<dyn NativeHttpTransport> = Arc::new(transport);
    let mut first = Box::pin(lookup(&cache, 100.0, None, transport.clone()));
    let mut second = Box::pin(lookup(&cache, 100.0, None, transport));
    assert!(first.as_mut().now_or_never().is_none());
    assert!(second.as_mut().now_or_never().is_none());
    completion
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .succeed(br#"{"available_count":2,"applicable_available_count":2}"#.to_vec());
    assert_eq!(futures_executor::block_on(first).0, Some(credits(2)));
    assert_eq!(futures_executor::block_on(second).0, Some(credits(2)));
}

#[test]
fn clear_prevents_an_in_flight_result_from_repopulating_cache() {
    let completion: Arc<std::sync::Mutex<Option<Arc<NativeHttpCompletion>>>> = Arc::new(std::sync::Mutex::new(None));
    let mut transport = MockTransport::new();
    let slot = completion.clone();
    transport.expect_start_get().once().return_once(move |_, callback| {
        *slot.lock().unwrap() = Some(callback);
    });
    let cache = CodexResetCreditCache::new();
    let transport: Arc<dyn NativeHttpTransport> = Arc::new(transport);
    let mut first = Box::pin(lookup(&cache, 100.0, None, transport.clone()));
    let mut second = Box::pin(lookup(&cache, 100.0, None, transport));
    assert!(first.as_mut().now_or_never().is_none());
    assert!(second.as_mut().now_or_never().is_none());
    cache.clear();
    assert_eq!(futures_executor::block_on(second).0, None);
    completion
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .succeed(br#"{"available_count":2,"applicable_available_count":2}"#.to_vec());
    assert_eq!(futures_executor::block_on(first).0, Some(credits(2)));
}

#[test]
fn cancelled_leader_releases_waiters() {
    let completion = Arc::new(std::sync::Mutex::new(None));
    let mut transport = MockTransport::new();
    let slot = completion.clone();
    transport.expect_start_get().once().return_once(move |_, callback| {
        *slot.lock().unwrap() = Some(callback);
    });
    let cache = CodexResetCreditCache::new();
    let transport: Arc<dyn NativeHttpTransport> = Arc::new(transport);
    let mut first = Box::pin(lookup(&cache, 100.0, None, transport.clone()));
    let mut second = Box::pin(lookup(&cache, 100.0, None, transport));
    assert!(first.as_mut().now_or_never().is_none());
    assert!(second.as_mut().now_or_never().is_none());
    drop(first);
    assert_eq!(futures_executor::block_on(second).0, None);
}
