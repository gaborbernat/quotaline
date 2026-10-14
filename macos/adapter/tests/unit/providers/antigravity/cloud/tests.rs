use super::*;
use crate::antigravity::{AntigravityQuotaResponse, antigravity_default_identity, antigravity_identity};
use crate::http::{HttpErrorRecord, ProviderFailureKind};
use crate::mock_http::MockTransport;

fn quota() -> Vec<u8> {
    br#"{"groups":[{"buckets":[{"bucketId":"gemini-5h","remainingFraction":0.25}]}]}"#.to_vec()
}

fn fetch(fetcher: &AntigravityCloudFetcher, now: f64, transport: Arc<MockTransport>) -> AntigravityCloudResult {
    futures_executor::block_on(fetcher.fetch("secret".into(), now, "Sign in".into(), String::new(), transport))
}

#[test]
fn cloud_fetch_uses_the_quota_endpoint_and_caches_tier_identity() {
    let mut transport = MockTransport::new();
    let requests = Arc::new(Mutex::new(vec![]));
    let observed = requests.clone();
    transport
        .expect_start_post_json()
        .times(5)
        .returning(move |request, completion| {
            assert_eq!(request.headers.get("Authorization"), Some(&"Bearer secret".into()));
            observed.lock().unwrap().push(request.operation.clone());
            if request.operation == "antigravity.quota" {
                assert_eq!(
                    request.url,
                    "https://daily-cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary"
                );
                assert_eq!(request.body, b"{}");
                completion.succeed(quota());
            } else {
                assert_eq!(
                    request.url,
                    "https://daily-cloudcode-pa.googleapis.com/v1internal:loadCodeAssist"
                );
                completion.succeed(br#"{"currentTier":{"name":"Pro"}}"#.to_vec());
            }
        });
    let fetcher = AntigravityCloudFetcher::new();
    let transport = Arc::new(transport);
    for now in [100.0, 160.0, 3701.0] {
        let fetched = fetch(&fetcher, now, transport.clone());
        assert_eq!(
            fetched.outcome,
            AntigravityCloudOutcome::Success(Box::new(AntigravityCloudSnapshot {
                provider: AntigravityQuotaResponse::new(quota())
                    .unwrap()
                    .provider(antigravity_identity(br#"{"currentTier":{"name":"Pro"}}"#.to_vec()).unwrap()),
                warning: None,
            }))
        );
        assert!(fetched.debug_messages.is_empty());
    }
    assert_eq!(
        requests.lock().unwrap().as_slice(),
        [
            "antigravity.quota",
            "antigravity.load-code-assist",
            "antigravity.quota",
            "antigravity.quota",
            "antigravity.load-code-assist",
        ]
    );
}

#[test]
fn cloud_fetch_uses_second_base_after_server_failure_and_warns_for_tier_failure() {
    let mut transport = MockTransport::new();
    transport
        .expect_start_post_json()
        .once()
        .return_once(|request, completion| {
            assert!(request.url.contains("daily-cloudcode-pa"));
            completion.fail(HttpErrorRecord::Response {
                status: 503,
                body: String::new(),
                retry_after: None,
            });
        });
    transport
        .expect_start_post_json()
        .once()
        .return_once(|request, completion| {
            assert!(request.url.contains("cloudcode-pa.googleapis.com/v1internal:retrieve"));
            completion.succeed(quota());
        });
    transport
        .expect_start_post_json()
        .once()
        .return_once(|request, completion| {
            assert!(request.url.contains("cloudcode-pa.googleapis.com/v1internal:load"));
            completion.fail(HttpErrorRecord::Response {
                status: 500,
                body: String::new(),
                retry_after: None,
            });
        });
    let fetched = fetch(&AntigravityCloudFetcher::new(), 100.0, Arc::new(transport));
    assert_eq!(
        fetched.debug_messages,
        ["antigravity: https://daily-cloudcode-pa.googleapis.com HTTP 503"]
    );
    assert_eq!(
        fetched.outcome,
        AntigravityCloudOutcome::Success(Box::new(AntigravityCloudSnapshot {
            provider: AntigravityQuotaResponse::new(quota())
                .unwrap()
                .provider(antigravity_default_identity()),
            warning: Some("Plan details unavailable: HTTP 500".into()),
        }))
    );
}

#[test]
fn cloud_fetch_stops_after_authentication_failure() {
    let mut transport = MockTransport::new();
    transport.expect_start_post_json().once().return_once(|_, completion| {
        completion.fail(HttpErrorRecord::Response {
            status: 401,
            body: String::new(),
            retry_after: None,
        });
    });
    assert_eq!(
        fetch(&AntigravityCloudFetcher::new(), 100.0, Arc::new(transport)).outcome,
        AntigravityCloudOutcome::Failure(ProviderFailure {
            kind: ProviderFailureKind::NotAuthenticated,
            message: "HTTP 401. Sign in".into(),
            retry_after: None,
        })
    );
}

#[test]
fn cloud_fetch_reports_last_failure_after_both_bases_fail() {
    let mut transport = MockTransport::new();
    transport
        .expect_start_post_json()
        .times(2)
        .returning(|request, completion| {
            completion.fail(HttpErrorRecord::Network {
                detail: if request.url.contains("daily-") {
                    "first"
                } else {
                    "second"
                }
                .into(),
            });
        });
    let fetched = fetch(&AntigravityCloudFetcher::new(), 100.0, Arc::new(transport));
    assert_eq!(fetched.debug_messages.len(), 2);
    assert_eq!(
        fetched.outcome,
        AntigravityCloudOutcome::Failure(ProviderFailure {
            kind: ProviderFailureKind::NetworkUnavailable,
            message: "second".into(),
            retry_after: None,
        })
    );
}

#[test]
fn cloud_fetch_maps_each_quota_payload_error() {
    for (payload, expected) in [
        (
            b"{".as_slice(),
            "Unexpected response: antigravity.quota: invalid payload",
        ),
        (
            br#"{"code":7}"#,
            "Unexpected response: Antigravity quota request failed at the RPC layer",
        ),
        (
            br#"{}"#,
            "Unexpected response: Antigravity response contains no quota groups",
        ),
    ] {
        let mut transport = MockTransport::new();
        let response = payload.to_vec();
        transport
            .expect_start_post_json()
            .times(2)
            .returning(move |_, completion| {
                completion.succeed(response.clone());
            });
        assert_eq!(
            fetch(&AntigravityCloudFetcher::new(), 100.0, Arc::new(transport)).outcome,
            AntigravityCloudOutcome::Failure(ProviderFailure {
                kind: ProviderFailureKind::Failed,
                message: expected.into(),
                retry_after: None,
            })
        );
    }
    let mut transport = MockTransport::new();
    transport.expect_start_post_json().once().return_once(|_, completion| {
        completion.succeed(br#"{"code":16}"#.to_vec());
    });
    assert_eq!(
        fetch(&AntigravityCloudFetcher::new(), 100.0, Arc::new(transport)).outcome,
        AntigravityCloudOutcome::Failure(ProviderFailure {
            kind: ProviderFailureKind::NotAuthenticated,
            message: "HTTP 401. Sign in".into(),
            retry_after: None,
        })
    );
}

#[test]
fn cloud_fetch_warns_when_tier_payload_is_invalid() {
    let mut transport = MockTransport::new();
    transport
        .expect_start_post_json()
        .once()
        .return_once(|_, completion| completion.succeed(quota()));
    transport
        .expect_start_post_json()
        .once()
        .return_once(|_, completion| completion.succeed(b"{".to_vec()));
    let fetched = fetch(&AntigravityCloudFetcher::new(), 100.0, Arc::new(transport));
    assert_eq!(
        fetched.outcome,
        AntigravityCloudOutcome::Success(Box::new(AntigravityCloudSnapshot {
            provider: AntigravityQuotaResponse::new(quota())
                .unwrap()
                .provider(antigravity_default_identity()),
            warning: Some(
                "Plan details unavailable: Unexpected response: antigravity.load-code-assist: invalid payload".into()
            ),
        }))
    );
}
