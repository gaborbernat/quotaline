//! Drives the shared Codex pipeline through a scripted transport.

use futures_executor::block_on as run;

use super::*;
use quotaline_http::test_transport::{Scripted, unavailable};

const ACCOUNT: Account<'static> = Account {
    base: "https://codex.test",
    token: "token",
    account_id: Some("account"),
};

#[test]
fn usage_returns_the_document_from_the_usage_endpoint() {
    let transport = Scripted::new(vec![Ok(br#"{"plan_type":"plus"}"#.to_vec())]);
    let usage = run(fetch_usage(&transport, &ACCOUNT)).unwrap();
    assert_eq!(usage, serde_json::json!({"plan_type":"plus"}));
    let sent = transport.sent.lock().unwrap();
    assert_eq!(
        (sent[0].url.as_str(), sent[0].operation.as_str()),
        ("https://codex.test/wham/usage", "codex.usage")
    );
    assert!(
        sent[0]
            .headers
            .contains(&("ChatGPT-Account-Id".into(), "account".into()))
    );
}

#[test]
fn usage_rejects_a_document_that_is_not_an_object() {
    let transport = Scripted::new(vec![Ok(b"[]".to_vec())]);
    assert_eq!(
        run(fetch_usage(&transport, &ACCOUNT)),
        Err(HttpError::Decoding("codex.usage: invalid payload".into()))
    );
}

#[test]
fn usage_passes_transport_failures_through() {
    let transport = Scripted::new(vec![Err(unavailable())]);
    assert_eq!(run(fetch_usage(&transport, &ACCOUNT)), Err(unavailable()));
}

#[test]
fn reset_credits_parse_the_reset_credit_endpoint() {
    let transport = Scripted::new(vec![Ok(br#"{"available_count":2}"#.to_vec())]);
    let credits = run(fetch_reset_credits(&transport, &ACCOUNT)).unwrap();
    assert_eq!((credits.available, credits.applicable), (2, 2));
    assert_eq!(
        transport.sent.lock().unwrap()[0].url,
        "https://codex.test/wham/rate-limit-reset-credits"
    );
}

#[test]
fn reset_credits_reject_a_document_that_is_not_an_object() {
    let transport = Scripted::new(vec![Ok(b"null".to_vec())]);
    let error = run(fetch_reset_credits(&transport, &ACCOUNT)).unwrap_err();
    assert_eq!(
        reset_credits_warning(&error, None),
        "Reset credits unavailable: Unexpected response: codex.reset-credits: invalid payload"
    );
}

#[test]
fn reset_credits_reject_a_payload_that_is_not_json() {
    let transport = Scripted::new(vec![Ok(b"<html>".to_vec())]);
    assert_eq!(
        run(fetch_reset_credits(&transport, &ACCOUNT)),
        Err(HttpError::Decoding("codex.reset-credits: invalid payload".into()))
    );
}

const NOW: f64 = 1_760_400_000.0;

fn analytics_context() -> AnalyticsContext<'static> {
    AnalyticsContext {
        fingerprint: "fingerprint",
        now: NOW,
        retention_days: 7,
        home: None,
    }
}

fn watermarks() -> Mutex<Watermarks> {
    Mutex::new(Watermarks::load(None, "fingerprint".into(), NOW, 7))
}

fn token_usage() -> Result<Vec<u8>, HttpError> {
    Ok(br#"{"data":[{"date":"2025-10-14","product_surface_usage_values":{"cli":30}}]}"#.to_vec())
}

#[test]
fn analytics_request_every_endpoint_from_its_watermark_and_the_credit_events() {
    let transport = Scripted::new(vec![token_usage(); 6]);
    run(fetch_analytics(
        &transport,
        &ACCOUNT,
        &watermarks(),
        &analytics_context(),
    ));
    let sent = transport.sent.lock().unwrap();
    assert_eq!(
        sent.iter()
            .map(|request| request.operation.as_str())
            .collect::<Vec<_>>(),
        [
            "codex.tokenUsage",
            "codex.workspaceCounts",
            "codex.skills",
            "codex.plugins",
            "codex.codeReview",
            "codex.credit-events"
        ]
    );
    assert!(sent[0].url.starts_with(
        "https://codex.test/wham/usage/daily-token-usage-breakdown?start_date=2025-10-08&end_date=2025-10-14"
    ));
    assert_eq!(sent[5].url, "https://codex.test/wham/usage/credit-usage-events");
}

#[test]
fn analytics_store_points_under_the_account_fingerprint() {
    let transport = Scripted::new(vec![token_usage(); 6]);
    let result = run(fetch_analytics(
        &transport,
        &ACCOUNT,
        &watermarks(),
        &analytics_context(),
    ));
    let snapshot = result.snapshot.unwrap();
    assert_eq!(snapshot.account_fingerprint.as_deref(), Some("fingerprint"));
    assert!(snapshot.points.iter().any(|point| point.day == "2025-10-14"));
}

#[test]
fn analytics_advance_the_watermarks_of_successful_endpoints() {
    let watermarks = watermarks();
    run(fetch_analytics(
        &Scripted::new(vec![token_usage(); 6]),
        &ACCOUNT,
        &watermarks,
        &analytics_context(),
    ));
    let plan = CodexBatchPlan::new(&mut watermarks.lock().unwrap(), NOW, 7);
    assert!(plan.starts.iter().all(|(_, start)| start == "2025-10-13"));
}

#[test]
fn analytics_word_a_failed_endpoint_as_a_warning() {
    let mut replies = vec![token_usage(); 5];
    replies[1] = Err(unavailable());
    replies.push(Ok(br#"{"data":[]}"#.to_vec()));
    let transport = Scripted::new(replies);
    let result = run(fetch_analytics(
        &transport,
        &ACCOUNT,
        &watermarks(),
        &analytics_context(),
    ));
    assert_eq!(result.warnings, ["WorkspaceCounts analytics unavailable: HTTP 503"]);
}
