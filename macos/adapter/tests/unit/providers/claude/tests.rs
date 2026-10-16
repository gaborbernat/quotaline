//! Verifies the macOS Claude bridge against shared Rust projections.

use super::*;
use crate::mock_http::MockTransport;

#[test]
fn structured_response_maps_dates_money_and_severity() {
    let result = claude_projection(
        br#"{"limits":[{"kind":"weekly_all","percent":100,"severity":"normal","resets_at":"2026-09-10T12:00:00Z"}],"spend":{"used":{"amount_minor":125,"currency":"CAD","exponent":2}}}"#.to_vec(),
        br#"{"claudeAiOauth":{"subscriptionType":"claude_max"}}"#.to_vec(),
        None,
    )
    .unwrap();
    assert_eq!(result.plan, "Max");
    assert_eq!(result.windows[0].severity.as_deref(), Some("normal"));
    assert_eq!(result.windows[0].reset.as_deref(), Some("2026-09-10T12:00:00Z"));
    assert_eq!(result.notices[0].resets_at.as_deref(), Some("2026-09-10T12:00:00Z"));
    assert_eq!(result.spend.unwrap().used_money.unwrap().amount_minor, 125);
}

#[test]
fn cloud_session_credits_cross_the_claude_projection() {
    let result = claude_projection(
        br#"{"iguana_necktie":{"limit_dollars":250,"remaining_dollars":83,"resets_at":"2026-11-04T23:59:00Z","locked_reason":"organization_locked"}}"#.to_vec(),
        br#"{"claudeAiOauth":{}}"#.to_vec(),
        None,
    )
    .unwrap();
    assert_eq!(
        result.cloud_credits,
        Some(quotaline_quota::CloudCredits {
            limit: 250.0,
            remaining: 83.0,
            expires_at: Some("2026-11-04T23:59:00Z".into()),
            locked_reason: Some("organization_locked".into()),
        })
    );
}

#[test]
fn profile_detail_is_projected_with_a_stable_id() {
    let result = claude_projection(
        br#"{"five_hour":{"utilization":1}}"#.to_vec(),
        br#"{"claudeAiOauth":{}}"#.to_vec(),
        Some(br#"{"organization":{"subscription_status":"active_trial"}}"#.to_vec()),
    )
    .unwrap();
    assert_eq!(
        result.details,
        vec![quotaline_quota::Detail {
            id: "subscription".into(),
            title: "Subscription".into(),
            value: "Active Trial".into(),
            explanation: "Subscription status reported by the Claude OAuth profile.".into(),
        }]
    );
}

#[test]
fn projection_rejects_invalid_envelopes() {
    for (usage, auth, profile) in [
        (b"invalid".as_slice(), br#"{}"#.as_slice(), None),
        (br#"{}"#, b"invalid", None),
        (br#"{}"#, br#"{}"#, Some(b"invalid".as_slice())),
        (br#"[]"#, br#"{}"#, None),
        (br#"{"error":"unauthorized"}"#, br#"{}"#, None),
        (br#"{}"#, br#"[]"#, None),
    ] {
        assert_eq!(
            claude_projection(usage.to_vec(), auth.to_vec(), profile.map(<[u8]>::to_vec)),
            Err(ClaudeProjectionError::InvalidJson),
            "{usage:?} {auth:?} {profile:?}"
        );
    }
}

#[test]
fn refreshed_credentials_keep_the_original_document() {
    let updated = claude_updated_credentials(
        br#"{"other":true,"claudeAiOauth":{"refreshToken":"keep"}}"#.to_vec(),
        "new".into(),
        None,
        1.0,
        1000,
    )
    .unwrap();
    let document: Value = serde_json::from_slice(&updated).unwrap();
    assert_eq!(document["other"], true);
    assert_eq!(document["claudeAiOauth"]["refreshToken"], "keep");
    assert_eq!(document["claudeAiOauth"]["expiresAt"], 2000);
    assert!(claude_updated_credentials(b"bad".to_vec(), "new".into(), None, 1.0, 0).is_none());
    assert!(claude_updated_credentials(b"[]".to_vec(), "new".into(), None, 1.0, 0).is_none());
}

#[test]
fn fetch_projects_concurrent_usage_and_profile_responses() {
    let usage = br#"{"five_hour":{"utilization":0.5}}"#.to_vec();
    let profile = br#"{"account":{"email":"account@example.com"}}"#.to_vec();
    let auth = br#"{"claudeAiOauth":{}}"#.to_vec();
    let mut transport = MockTransport::new();
    let usage_response = usage.clone();
    let profile_response = profile.clone();
    transport
        .expect_start_get()
        .times(2)
        .returning(move |request, completion| {
            assert_eq!(request.headers.get("Authorization"), Some(&"Bearer token".into()));
            if request.operation == "claude.usage" {
                assert_eq!(request.url, request::USAGE_URL);
                completion.succeed(usage_response.clone());
            } else {
                assert_eq!(request.operation, "claude.profile");
                assert_eq!(request.url, request::PROFILE_URL);
                completion.succeed(profile_response.clone());
            }
        });
    assert_eq!(
        futures_executor::block_on(claude_fetch(
            ClaudeFetchInput {
                token: "token".into(),
                auth: auth.clone(),
                fingerprint: "account".into(),
                now: 100.0,
                hint: "sign in".into(),
                home: String::new(),
            },
            Arc::new(ClaudeProfileCache::new()),
            Arc::new(transport),
        )),
        ClaudeFetchResult {
            outcome: ClaudeFetchOutcome::Provider(Box::new(claude_projection(usage, auth, Some(profile)).unwrap())),
            profile_warning: None,
        }
    );
}

#[test]
fn fetch_preserves_profile_warning_when_usage_fails() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().times(2).returning(|request, completion| {
        completion.fail(crate::http::HttpErrorRecord::Network {
            detail: if request.operation == "claude.usage" {
                "usage"
            } else {
                "profile"
            }
            .into(),
        });
    });
    assert_eq!(
        futures_executor::block_on(claude_fetch(
            ClaudeFetchInput {
                token: "token".into(),
                auth: br#"{"claudeAiOauth":{}}"#.to_vec(),
                fingerprint: "account".into(),
                now: 100.0,
                hint: "sign in".into(),
                home: String::new(),
            },
            Arc::new(ClaudeProfileCache::new()),
            Arc::new(transport),
        )),
        ClaudeFetchResult {
            outcome: ClaudeFetchOutcome::Failure(ProviderFailure {
                kind: crate::http::ProviderFailureKind::NetworkUnavailable,
                message: "usage".into(),
                retry_after: None,
            }),
            profile_warning: Some("Profile unavailable: Network error: profile".into()),
        }
    );
}

#[test]
fn fetch_maps_unreadable_usage_to_provider_failure() {
    for usage in [b"{".as_slice(), br#"{"error":"denied"}"#.as_slice()] {
        fetch_reports_unreadable_usage(usage.to_vec());
    }
}

fn fetch_reports_unreadable_usage(usage: Vec<u8>) {
    let mut transport = MockTransport::new();
    transport
        .expect_start_get()
        .times(2)
        .returning(move |request, completion| {
            completion.succeed(if request.operation == "claude.usage" {
                usage.clone()
            } else {
                br#"{"account":{"email":"account@example.com"}}"#.to_vec()
            });
        });
    assert_eq!(
        futures_executor::block_on(claude_fetch(
            ClaudeFetchInput {
                token: "token".into(),
                auth: br#"{"claudeAiOauth":{}}"#.to_vec(),
                fingerprint: "account".into(),
                now: 100.0,
                hint: "sign in".into(),
                home: String::new(),
            },
            Arc::new(ClaudeProfileCache::new()),
            Arc::new(transport),
        )),
        ClaudeFetchResult {
            outcome: ClaudeFetchOutcome::Failure(ProviderFailure {
                kind: crate::http::ProviderFailureKind::Failed,
                message: "Unexpected response: claude.usage: invalid payload".into(),
                retry_after: None,
            }),
            profile_warning: None,
        }
    );
}
