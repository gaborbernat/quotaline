use super::*;
use crate::mock_http::MockTransport;

#[test]
fn bridge_reuses_and_expires_profile() {
    let profile = br#"{"account":{"email":"a@example.com"}}"#.to_vec();
    let mut transport = MockTransport::new();
    let response = profile.clone();
    transport
        .expect_start_get()
        .times(2)
        .returning(move |request, completion| {
            assert_eq!(request.url, request::PROFILE_URL);
            assert_eq!(request.operation, "claude.profile");
            assert_eq!(request.headers.get("Authorization"), Some(&"Bearer token".into()));
            completion.succeed(response.clone());
        });
    let cache = ClaudeProfileCache::new();
    let transport = Arc::new(transport);
    for now in [100.0, 101.0, 100.0 + claude_profile_cache_interval()] {
        assert_eq!(
            futures_executor::block_on(cache.fetch(
                "token".into(),
                "account".into(),
                now,
                String::new(),
                transport.clone(),
            )),
            ClaudeProfileResult {
                value: Some(profile.clone()),
                warning: None,
            }
        );
    }
}

#[test]
fn bridge_scopes_profile_to_credential() {
    let mut transport = MockTransport::new();
    transport
        .expect_start_get()
        .times(2)
        .returning(|_, completion| completion.succeed(br#"{"organization":{}}"#.to_vec()));
    let cache = ClaudeProfileCache::new();
    let transport = Arc::new(transport);
    for fingerprint in ["first", "second"] {
        assert_eq!(
            futures_executor::block_on(cache.fetch(
                "token".into(),
                fingerprint.into(),
                100.0,
                String::new(),
                transport.clone(),
            )),
            ClaudeProfileResult {
                value: Some(br#"{"organization":{}}"#.to_vec()),
                warning: None,
            }
        );
    }
}

#[test]
fn bridge_reuses_stale_profile_and_shortens_retry_after_failure() {
    let profile = br#"{"account":{"email":"a@example.com"}}"#.to_vec();
    let mut transport = MockTransport::new();
    let response = profile.clone();
    transport
        .expect_start_get()
        .once()
        .return_once(move |_, completion| completion.succeed(response));
    transport.expect_start_get().once().return_once(|_, completion| {
        completion.fail(crate::http::HttpErrorRecord::Response {
            status: 503,
            body: String::new(),
            retry_after: None,
        });
    });
    let cache = ClaudeProfileCache::new();
    let transport = Arc::new(transport);
    assert_eq!(
        futures_executor::block_on(cache.fetch(
            "token".into(),
            "account".into(),
            0.0,
            String::new(),
            transport.clone(),
        ))
        .value,
        Some(profile.clone())
    );
    for now in [claude_profile_cache_interval(), claude_profile_cache_interval() + 1.0] {
        assert_eq!(
            futures_executor::block_on(cache.fetch(
                "token".into(),
                "account".into(),
                now,
                String::new(),
                transport.clone(),
            )),
            ClaudeProfileResult {
                value: Some(profile.clone()),
                warning: Some("Profile unavailable: HTTP 503".into()),
            }
        );
    }
}

#[test]
fn bridge_rejects_unreadable_profile_with_shared_warning() {
    for payload in [br#"{}"#.as_slice(), br#"{"error":"denied"}"#, br#"[]"#, br#"invalid"#] {
        let mut transport = MockTransport::new();
        let response = payload.to_vec();
        transport
            .expect_start_get()
            .once()
            .return_once(move |_, completion| completion.succeed(response));
        assert_eq!(
            futures_executor::block_on(ClaudeProfileCache::new().fetch(
                "token".into(),
                "account".into(),
                100.0,
                String::new(),
                Arc::new(transport),
            )),
            ClaudeProfileResult {
                value: None,
                warning: Some("Profile unavailable: Provider returned an invalid profile".into()),
            }
        );
    }
}
