//! Checks the public behavior of the adjacent module.

use super::*;
use crate::http::{HttpErrorRecord, ProviderFailureKind};
use crate::mock_http::MockTransport;

#[test]
fn request_headers_name_the_cli_and_architecture() {
    assert_eq!(
        gemini_request_headers("secret".into(), "arm64".into()),
        std::collections::HashMap::from([
            ("Authorization".into(), "Bearer secret".into()),
            ("User-Agent".into(), "GeminiCLI/quotaline (darwin; arm64; cli)".into()),
        ])
    );
}

#[test]
fn bridge_projects_quota_with_plan_credits() {
    let assist = serde_json::json!({
        "cloudaicompanionProject":{"id":"project"},
        "currentTier":{"id":"standard-tier","availableCredits":[{"creditType":"GOOGLE_ONE_AI","creditAmount":"2.5"}]}
    });
    let result = project(
        &serde_json::json!({"buckets":[{"modelId":"gemini-2-pro","remainingFraction":0.7}]}),
        &assist,
        Some("example.com".into()),
        Some("me@example.com".into()),
    );
    assert_eq!(result.plan, "Standard");
    assert_eq!(result.tier.as_deref(), Some("standard-tier"));
    assert_eq!(
        (result.identity.as_str(), result.organization.as_str()),
        ("me@example.com", "example.com")
    );
    assert_eq!(result.windows[0].label, "Gemini 2 Pro");
    assert_eq!(result.credit_balance, Some(2.5));
    assert_eq!(result.details[0].value, "2.5 credits");
}

#[test]
fn bridge_detects_subscription_errors() {
    assert!(gemini_subscription_required("SUBSCRIPTION_REQUIRED".into()));
    assert!(!gemini_subscription_required("not required".into()));
    assert!(gemini_unsupported_client_message().contains("Gemini Code Assist"));
}

fn fetch_input() -> GeminiFetchInput {
    GeminiFetchInput {
        token: "secret".into(),
        fingerprint: "account".into(),
        architecture: "arm64".into(),
        hosted_domain: None,
        email: None,
        now: 100.0,
        hint: "Sign in".into(),
        home: String::new(),
    }
}

#[test]
fn fetch_uses_assist_project_for_quota_and_projects_response() {
    let assist = br#"{"cloudaicompanionProject":{"id":"project"},"currentTier":{"id":"standard-tier"}}"#.to_vec();
    let quota = br#"{"buckets":[{"modelId":"gemini-2-pro","remainingFraction":0.7}]}"#.to_vec();
    let mut transport = MockTransport::new();
    let assist_response = assist.clone();
    let quota_response = quota.clone();
    transport
        .expect_start_post_json()
        .times(2)
        .returning(move |request, completion| {
            assert_eq!(request.headers.get("Authorization"), Some(&"Bearer secret".into()));
            if request.operation == "gemini.load-code-assist" {
                completion.succeed(assist_response.clone());
            } else {
                assert_eq!(request.operation, "gemini.quota");
                assert_eq!(
                    request.url,
                    "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota"
                );
                assert_eq!(
                    serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
                    serde_json::json!({"project":"project"})
                );
                completion.succeed(quota_response.clone());
            }
        });
    let result = futures_executor::block_on(gemini_fetch(
        fetch_input(),
        Arc::new(GeminiAssistCache::new()),
        Arc::new(transport),
    ));
    let expected = project(
        &serde_json::from_slice(&quota).unwrap(),
        &serde_json::from_slice(&assist).unwrap(),
        None,
        None,
    );
    assert_eq!(result.outcome, GeminiFetchOutcome::Provider(Box::new(expected)));
    assert!(result.warnings.is_empty());
}

#[test]
fn fetch_rejects_unsupported_assist_without_requesting_quota() {
    let mut transport = MockTransport::new();
    transport.expect_start_post_json().once().return_once(|_, completion| {
        completion.succeed(br#"{"ineligibleTiers":[{"reasonCode":"UNSUPPORTED_CLIENT"}]}"#.to_vec());
    });
    let result = futures_executor::block_on(gemini_fetch(
        fetch_input(),
        Arc::new(GeminiAssistCache::new()),
        Arc::new(transport),
    ));
    assert!(matches!(result.outcome, GeminiFetchOutcome::Unsupported(_)));
}

#[test]
fn fetch_propagates_assist_authentication_and_rate_limits() {
    for (status, kind, message) in [
        (401, ProviderFailureKind::NotAuthenticated, "HTTP 401. Sign in"),
        (429, ProviderFailureKind::RateLimited, "HTTP 429"),
    ] {
        let mut transport = MockTransport::new();
        transport
            .expect_start_post_json()
            .once()
            .return_once(move |_, completion| {
                completion.fail(HttpErrorRecord::Response {
                    status,
                    body: String::new(),
                    retry_after: None,
                });
            });
        let result = futures_executor::block_on(gemini_fetch(
            fetch_input(),
            Arc::new(GeminiAssistCache::new()),
            Arc::new(transport),
        ));
        assert_eq!(
            result.outcome,
            GeminiFetchOutcome::Failure(ProviderFailure {
                kind,
                message: message.into(),
                retry_after: None,
            })
        );
        assert!(result.warnings.is_empty());
    }
}

#[test]
fn fetch_uses_empty_assist_when_plan_details_fail() {
    let mut transport = MockTransport::new();
    transport.expect_start_post_json().once().return_once(|_, completion| {
        completion.fail(HttpErrorRecord::Network {
            detail: "offline".into(),
        });
    });
    transport
        .expect_start_post_json()
        .once()
        .return_once(|request, completion| {
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
                serde_json::json!({})
            );
            completion.succeed(br#"{"buckets":[]}"#.to_vec());
        });
    let result = futures_executor::block_on(gemini_fetch(
        fetch_input(),
        Arc::new(GeminiAssistCache::new()),
        Arc::new(transport),
    ));
    assert_eq!(result.warnings, ["Plan details unavailable: Network error: offline"]);
    assert_eq!(
        result.outcome,
        GeminiFetchOutcome::Provider(Box::new(project(
            &serde_json::json!({"buckets":[]}),
            &serde_json::Value::Null,
            None,
            None
        )))
    );
}

#[test]
fn fetch_maps_quota_decoding_and_network_failures() {
    for (response, expected) in [
        (
            Ok(b"{".to_vec()),
            ProviderFailure {
                kind: ProviderFailureKind::Failed,
                message: "Unexpected response: gemini.quota: invalid payload".into(),
                retry_after: None,
            },
        ),
        (
            Err(HttpErrorRecord::Network {
                detail: "offline".into(),
            }),
            ProviderFailure {
                kind: ProviderFailureKind::NetworkUnavailable,
                message: "offline".into(),
                retry_after: None,
            },
        ),
        (
            Err(HttpErrorRecord::Response {
                status: 403,
                body: "denied".into(),
                retry_after: None,
            }),
            crate::http::http_provider_failure(
                HttpErrorRecord::Response {
                    status: 403,
                    body: "denied".into(),
                    retry_after: None,
                },
                "Sign in".into(),
                Some(String::new()),
            ),
        ),
    ] {
        let mut transport = MockTransport::new();
        transport
            .expect_start_post_json()
            .once()
            .return_once(|_, completion| completion.succeed(b"{}".to_vec()));
        transport
            .expect_start_post_json()
            .once()
            .return_once(move |_, completion| match response {
                Ok(payload) => completion.succeed(payload),
                Err(error) => completion.fail(error),
            });
        let result = futures_executor::block_on(gemini_fetch(
            fetch_input(),
            Arc::new(GeminiAssistCache::new()),
            Arc::new(transport),
        ));
        assert_eq!(result.outcome, GeminiFetchOutcome::Failure(expected));
    }
}

#[test]
fn fetch_detects_subscription_required_quota_error() {
    let mut transport = MockTransport::new();
    transport
        .expect_start_post_json()
        .once()
        .return_once(|_, completion| completion.succeed(b"{}".to_vec()));
    transport.expect_start_post_json().once().return_once(|_, completion| {
        completion.fail(HttpErrorRecord::Response {
            status: 403,
            body: r#"{"error":{"status":"SUBSCRIPTION_REQUIRED"}}"#.into(),
            retry_after: None,
        });
    });
    assert_eq!(
        futures_executor::block_on(gemini_fetch(
            fetch_input(),
            Arc::new(GeminiAssistCache::new()),
            Arc::new(transport),
        ))
        .outcome,
        GeminiFetchOutcome::Unsupported(gemini::UNSUPPORTED_CLIENT_MESSAGE.into())
    );
}
