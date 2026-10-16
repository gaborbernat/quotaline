//! Checks the public behavior of the adjacent module.

use super::*;
use crate::http::HttpErrorRecord;
use crate::mock_http::MockTransport;

#[test]
fn bridge_passes_only_request_dates_and_persisted_bytes() {
    let session = CodexAnalyticsWatermarks::new(None, "account".into(), 1_777_636_800.0, 7);
    let planned = session.plan("2026-04-25".into(), "2026-05-01".into());
    assert_eq!(planned.len(), 5);
    assert_eq!(planned[0].day, "2026-04-25");
    session.complete(
        vec![
            Endpoint::TokenUsage,
            Endpoint::WorkspaceCounts,
            Endpoint::Skills,
            Endpoint::Plugins,
            Endpoint::CodeReview,
        ],
        "2026-05-01".into(),
        1_777_636_800.0,
        7,
    );
    let raw = session.encoded_if_changed().unwrap();
    assert!(session.encoded_if_changed().is_none());
    let relaunched = CodexAnalyticsWatermarks::new(Some(raw), "account".into(), 1_777_723_200.0, 7);
    let planned = relaunched.plan("2026-04-26".into(), "2026-05-02".into());
    assert!(planned.iter().all(|start| start.day == "2026-04-30"));
}

fn fetch_input() -> CodexAnalyticsFetchInput {
    CodexAnalyticsFetchInput {
        token: "secret".into(),
        account_id: Some("account-id".into()),
        account: "account".into(),
        now: 1_760_400_000.0,
        retention_days: 7,
        home: String::new(),
    }
}

#[test]
fn fetch_batch_requests_endpoints_and_credit_events_concurrently() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().times(6).returning(|request, completion| {
        assert_eq!(request.headers.get("Authorization"), Some(&"Bearer secret".into()));
        assert_eq!(request.headers.get("ChatGPT-Account-Id"), Some(&"account-id".into()));
        if request.operation == "codex.credit-events" {
            completion.succeed(br#"{"data":[{"date":"2025-10-14","usage_id":"event","credit_amount":3}]}"#.to_vec());
        } else {
            assert!(request.url.contains("start_date=2025-10-08&end_date=2025-10-14"));
            assert!(request.operation.starts_with("codex."));
            completion.succeed(br#"{"data":[]}"#.to_vec());
        }
    });
    let session = CodexAnalyticsWatermarks::new(None, "account".into(), 1_760_400_000.0, 7);
    let result = futures_executor::block_on(session.fetch_batch(fetch_input(), Arc::new(transport)));
    assert!(result.warnings.is_empty());
    assert_eq!(result.snapshot.unwrap().credit_events[0].id, "event");
    assert!(session.encoded_if_changed().is_some());
}

#[test]
fn fetch_batch_collects_endpoint_and_credit_failures() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().times(6).returning(|_, completion| {
        completion.fail(HttpErrorRecord::Network {
            detail: "offline".into(),
        });
    });
    let session = CodexAnalyticsWatermarks::new(None, "account".into(), 1_760_400_000.0, 7);
    let result = futures_executor::block_on(session.fetch_batch(fetch_input(), Arc::new(transport)));
    assert_eq!(result.snapshot, None);
    assert_eq!(result.warnings.len(), 6);
    assert!(
        result
            .warnings
            .iter()
            .all(|warning| warning.contains("Network error: offline"))
    );
    assert!(session.encoded_if_changed().is_none());
}
