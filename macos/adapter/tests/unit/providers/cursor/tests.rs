//! Checks the public behavior of the adjacent module.

use super::*;
use crate::http::HttpErrorRecord;
use crate::mock_http::MockTransport;
use serde_json::json;

#[test]
fn summary_bridge_keeps_identity_windows_spend_and_details() {
    let response = CursorResponse::new(
        serde_json::to_vec(&json!({
            "membershipType":"pro_plus","billingCycleEnd":"2026-09-10T00:00:00Z",
            "individualUsage":{"plan":{"used":25,"limit":100,"enabled":true},
                "onDemand":{"used":500,"limit":10000,"remaining":9500}}
        }))
        .unwrap(),
        None,
    )
    .unwrap();
    let result = response.project(
        Some("fallback".into()),
        Some("cached@example.com".into()),
        Some("me@example.com".into()),
    );
    assert_eq!(result.plan, "Pro Plus");
    assert_eq!(result.identity, "me@example.com");
    assert_eq!(result.windows[0].used, 25.0);
    assert_eq!(result.windows[0].detail, "");
    assert_eq!(result.spend.unwrap().used_money.unwrap().amount_minor, 500);
    assert_eq!(result.details[0].title, "Included plan");
    assert!(result.notices.is_empty());
    assert_eq!(response.warning(), None);
}

#[test]
fn period_bridge_normalizes_plan_and_preserves_message() {
    let response = CursorResponse::new(
        b"{\"planUsage\":{\"used\":60,\"limit\":100},\"displayMessage\":\"60% used\"}".to_vec(),
        Some("Provider returned HTTP 503".into()),
    )
    .unwrap();
    let result = response.project(Some("pro".into()), Some("cached@example.com".into()), None);
    assert_eq!(result.plan, "Pro");
    assert_eq!(result.identity, "cached@example.com");
    assert_eq!(result.windows[0].used, 60.0);
    assert_eq!(result.spend, None);
    assert_eq!(result.notices[0].text, "60% used");
    assert_eq!(
        response.warning().as_deref(),
        Some("Dashboard summary unavailable: Provider returned HTTP 503")
    );
}

#[test]
fn bridge_handles_missing_messages_and_invalid_json() {
    let response = CursorResponse::new(b"{}".to_vec(), Some("offline".into())).unwrap();
    let result = response.project(None, None, None);
    assert_eq!(result.plan, "Cursor");
    assert!(result.notices.is_empty());
    assert_eq!(
        CursorResponse::new(b"{".to_vec(), None).err(),
        Some(CursorError::InvalidJson)
    );
}

#[test]
fn fetch_uses_dashboard_summary_without_period_request() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().return_once(|request, completion| {
        assert_eq!(request.url, request::SUMMARY_URL);
        assert_eq!(request.operation, "cursor.usage-summary");
        assert_eq!(request.headers.get("Cookie"), Some(&"session=secret".into()));
        completion.succeed(br#"{"membershipType":"pro"}"#.to_vec());
    });
    transport.expect_start_post_json().never();
    assert_eq!(
        futures_executor::block_on(cursor_fetch_usage(
            "session=secret".into(),
            "token".into(),
            "sign in".into(),
            String::new(),
            Arc::new(transport),
        )),
        CursorUsageFetch::Response(Arc::new(
            CursorResponse::new(br#"{"membershipType":"pro"}"#.to_vec(), None).unwrap()
        ))
    );
}

#[test]
fn fetch_falls_back_after_invalid_summary_payload() {
    let mut transport = MockTransport::new();
    transport
        .expect_start_get()
        .return_once(|_, completion| completion.succeed(b"{".to_vec()));
    transport.expect_start_post_json().return_once(|request, completion| {
        assert_eq!(request.url, request::PERIOD_URL);
        assert_eq!(request.operation, "cursor.period-usage");
        assert_eq!(request.headers.get("Authorization"), Some(&"Bearer token".into()));
        assert_eq!(request.body, b"{}".to_vec());
        completion.succeed(br#"{"planUsage":{"used":5,"limit":10}}"#.to_vec());
    });
    assert_eq!(
        futures_executor::block_on(cursor_fetch_usage(
            "cookie".into(),
            "token".into(),
            "sign in".into(),
            String::new(),
            Arc::new(transport),
        )),
        CursorUsageFetch::Response(Arc::new(
            CursorResponse::new(
                br#"{"planUsage":{"used":5,"limit":10}}"#.to_vec(),
                Some("Unexpected response: cursor.usage-summary: invalid payload".into()),
            )
            .unwrap()
        ))
    );
}

#[test]
fn fetch_falls_back_after_http_failure() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().return_once(|_, completion| {
        completion.fail(HttpErrorRecord::Response {
            status: 503,
            body: String::new(),
            retry_after: None,
        });
    });
    transport
        .expect_start_post_json()
        .return_once(|_, completion| completion.succeed(b"{}".to_vec()));
    assert_eq!(
        futures_executor::block_on(cursor_fetch_usage(
            "cookie".into(),
            "token".into(),
            "sign in".into(),
            String::new(),
            Arc::new(transport),
        )),
        CursorUsageFetch::Response(Arc::new(
            CursorResponse::new(b"{}".to_vec(), Some("HTTP 503".into())).unwrap()
        ))
    );
}

#[test]
fn fetch_reports_period_transport_and_payload_failures() {
    for (period, expected) in [
        (
            Err(HttpErrorRecord::Network {
                detail: "offline".into(),
            }),
            ProviderFailure {
                kind: crate::http::ProviderFailureKind::NetworkUnavailable,
                message: "offline".into(),
                retry_after: None,
            },
        ),
        (
            Ok(b"{".to_vec()),
            ProviderFailure {
                kind: crate::http::ProviderFailureKind::Failed,
                message: "Unexpected response: cursor.period-usage: invalid payload".into(),
                retry_after: None,
            },
        ),
    ] {
        let mut transport = MockTransport::new();
        transport.expect_start_get().return_once(|_, completion| {
            completion.fail(HttpErrorRecord::Network {
                detail: "summary offline".into(),
            });
        });
        transport
            .expect_start_post_json()
            .return_once(move |_, completion| match period {
                Ok(payload) => completion.succeed(payload),
                Err(error) => completion.fail(error),
            });
        assert_eq!(
            futures_executor::block_on(cursor_fetch_usage(
                "cookie".into(),
                "token".into(),
                "sign in".into(),
                String::new(),
                Arc::new(transport),
            )),
            CursorUsageFetch::Failure(expected)
        );
    }
}
