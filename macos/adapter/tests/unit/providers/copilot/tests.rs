//! Checks the public behavior of the adjacent module.

use super::*;
use serde_json::json;

use crate::mock_http::MockTransport;

#[test]
fn bridge_exposes_the_shared_quota_request() {
    assert_eq!(
        copilot_request("company.ghe.com".into(), "secret".into()),
        Some(CopilotRequest {
            url: "https://api.company.ghe.com/copilot_internal/user".into(),
            headers: std::collections::HashMap::from([
                ("Authorization".into(), "token secret".into()),
                ("Editor-Version".into(), "vscode/1.96.2".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.26.7".into()),
                ("User-Agent".into(), "GitHubCopilotChat/0.26.7".into()),
                ("X-Github-Api-Version".into(), "2025-04-01".into()),
            ]),
        })
    );
}

#[test]
fn bridge_rejects_invalid_hosts_without_emitting_request_headers() {
    assert_eq!(copilot_request("bad/path".into(), "secret".into()), None);
}

fn fetch(payload: serde_json::Value) -> CopilotFetch {
    let mut transport = MockTransport::new();
    transport.expect_start_get().returning(move |request, completion| {
        assert_eq!(request.url, "https://api.github.com/copilot_internal/user");
        assert_eq!(request.operation, "copilot.user");
        assert_eq!(request.headers.get("Authorization"), Some(&"token secret".into()));
        completion.succeed(serde_json::to_vec(&payload).unwrap());
    });
    futures_executor::block_on(copilot_fetch(
        "github.com".into(),
        "secret".into(),
        Some("octocat".into()),
        "sign in".into(),
        String::new(),
        std::sync::Arc::new(transport),
    ))
}

fn reading(outcome: CopilotFetch) -> Option<ProviderRecord> {
    match outcome {
        CopilotFetch::Provider(provider) => Some(*provider),
        _ => None,
    }
}

fn provider(payload: serde_json::Value) -> ProviderRecord {
    reading(fetch(payload)).unwrap()
}

#[test]
fn fetch_returns_the_shared_provider_reading_for_the_signed_in_user() {
    let provider = provider(json!({"copilot_plan":"individual"}));
    assert_eq!(
        (provider.id.as_str(), provider.plan.as_str(), provider.identity.as_str()),
        ("copilot", "Pro", "octocat")
    );
}

#[test]
fn fetch_keeps_the_seat_tier_and_day_precision_resets() {
    let provider = provider(json!({
        "copilot_plan":"individual_pro","access_type_sku":"copilot_pro_seat",
        "quota_reset_date":"2026-09-01",
        "quota_snapshots":{
            "premium_interactions":{"entitlement":100,"remaining":-10,"overage_permitted":true},
            "chat":{"percent_remaining":20,"quota_reset_at":1788558705}
        }
    }));
    assert_eq!(provider.plan, "Pro+");
    assert_eq!(provider.tier.as_deref(), Some("copilot_pro_seat"));
    assert!(provider.windows[0].reset_by_day);
    assert!(!provider.windows[1].reset_by_day);
    assert_eq!(provider.notices[0].kind, "info");
}

#[test]
fn fetch_reports_an_exhausted_quota_as_a_limit_notice() {
    let provider = provider(json!({"quota_snapshots":{"chat":{"percent_remaining":-5}}}));
    assert_eq!(provider.notices[0].kind, "limitReached");
}

#[test]
fn fetch_rejects_invalid_hosts_before_requesting_data() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().never();
    let outcome = futures_executor::block_on(copilot_fetch(
        "bad/path".into(),
        "secret".into(),
        None,
        "sign in".into(),
        String::new(),
        std::sync::Arc::new(transport),
    ));
    assert!(reading(outcome.clone()).is_none());
    assert_eq!(outcome, CopilotFetch::InvalidHost);
}

#[test]
fn fetch_maps_transport_and_payload_errors() {
    for (response, expected) in [
        (
            Err(crate::http::HttpErrorRecord::Response {
                status: 401,
                body: "bad token".into(),
                retry_after: None,
            }),
            ProviderFailure {
                kind: crate::http::ProviderFailureKind::NotAuthenticated,
                message: "HTTP 401. sign in".into(),
                retry_after: None,
            },
        ),
        (
            Ok(b"{".to_vec()),
            ProviderFailure {
                kind: crate::http::ProviderFailureKind::Failed,
                message: "Unexpected response: copilot.user: invalid payload".into(),
                retry_after: None,
            },
        ),
    ] {
        let mut transport = MockTransport::new();
        transport
            .expect_start_get()
            .return_once(move |_, completion| match response {
                Ok(payload) => completion.succeed(payload),
                Err(error) => completion.fail(error),
            });
        assert_eq!(
            futures_executor::block_on(copilot_fetch(
                "github.com".into(),
                "secret".into(),
                None,
                "sign in".into(),
                String::new(),
                std::sync::Arc::new(transport),
            )),
            CopilotFetch::Failure(expected)
        );
    }
}
