//! Checks the public behavior of the adjacent module.

use super::*;
use serde_json::json;

#[test]
fn request_headers_preserve_local_csrf() {
    assert_eq!(
        antigravity_server_headers(None),
        std::collections::HashMap::from([("Connect-Protocol-Version".into(), "1".into())])
    );
    assert_eq!(
        antigravity_server_headers(Some("csrf".into())),
        std::collections::HashMap::from([
            ("Connect-Protocol-Version".into(), "1".into()),
            ("X-Codeium-Csrf-Token".into(), "csrf".into()),
        ])
    );
}

#[test]
fn local_request_bodies_match_the_language_server_protocol() {
    for (body, expected) in [
        (antigravity_server_quota_body(), json!({"forceRefresh":true})),
        (
            antigravity_server_status_body(),
            json!({"metadata":{"ideName":"antigravity","extensionName":"antigravity","ideVersion":"unknown","locale":"en"}}),
        ),
    ] {
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&body).unwrap(), expected);
    }
}

#[test]
fn quota_bridge_keeps_windows_and_details() {
    let response = AntigravityQuotaResponse::new(
        serde_json::to_vec(&json!({"groups":[{"buckets":[
            {"bucketId":"gemini-5h","remainingFraction":0.25,"resetTime":"2026-10-10T00:00:00Z",
                "description":"Session pool"},
            {"bucketId":"disabled","disabled":true,"remaining":{"remainingAmount":1500}}
        ]}]}))
        .unwrap(),
    )
    .unwrap();
    let projection = response.provider(antigravity_default_identity());
    assert_eq!(projection.windows[0].id, "gemini:session");
    assert_eq!(projection.windows[0].label, "Gemini 5-hour");
    assert_eq!(projection.windows[0].group, "session");
    assert_eq!(projection.windows[0].used, 75.0);
    assert_eq!(projection.windows[0].reset.as_deref(), Some("2026-10-10T00:00:00Z"));
    assert_eq!(projection.windows[0].duration_seconds, Some(18_000));
    assert_eq!(projection.windows[0].detail, "Session pool");
    assert_eq!(projection.details[0].title, "disabled");
    assert_eq!(projection.details[0].value, "Disabled · 1.5K remaining");
    assert!(
        projection.details[0]
            .explanation
            .contains("did not report a usable percentage")
    );
}

#[test]
fn quota_bridge_preserves_rpc_errors() {
    for (payload, expected) in [
        ("{", AntigravityError::InvalidJson),
        (r#"{"code":16}"#, AntigravityError::Unauthenticated),
        (r#"{"code":7}"#, AntigravityError::RpcFailure),
        ("{}", AntigravityError::MissingGroups),
    ] {
        assert_eq!(
            AntigravityQuotaResponse::new(payload.as_bytes().to_vec()).err(),
            Some(expected)
        );
    }
    assert!(
        AntigravityQuotaResponse::new(br#"{"groups":[]}"#.to_vec())
            .unwrap()
            .provider(antigravity_default_identity())
            .windows
            .is_empty()
    );
}

#[test]
fn identity_bridge_keeps_local_and_cloud_accounts() {
    let local =
        antigravity_identity(br#"{"userStatus":{"userTier":{"name":"Ultra","id":"ultra"},"email":"a@b"}}"#.to_vec())
            .unwrap();
    assert_eq!(local.plan_name, "Ultra");
    assert_eq!(local.tier.as_deref(), Some("ultra"));
    assert_eq!(local.email.as_deref(), Some("a@b"));
    let cloud = antigravity_identity(br#"{"currentTier":{"name":"Pro"}}"#.to_vec()).unwrap();
    assert_eq!(cloud.plan_name, "Pro");
    assert_eq!(
        antigravity_default_identity(),
        antigravity_identity(b"{}".to_vec()).unwrap()
    );
    assert_eq!(antigravity_identity(b"{".to_vec()), Err(AntigravityError::InvalidJson));
}
