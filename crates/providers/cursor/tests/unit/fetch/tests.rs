//! Drives the shared Cursor pipeline through a scripted transport.

use futures_executor::block_on as run;

use super::*;
use quotaline_http::test_transport::{Scripted, unavailable};
use quotaline_http::transport::Body;

#[test]
fn summary_answers_without_the_period_request() {
    let transport = Scripted::new(vec![Ok(br#"{"membershipType":"pro"}"#.to_vec())]);
    let usage = run(fetch_usage(&transport, &Endpoints::default(), "cookie", "token", None)).unwrap();
    assert_eq!(
        usage,
        UsageResponse::summary(serde_json::json!({"membershipType":"pro"}))
    );
    let sent = transport.sent.lock().unwrap();
    assert_eq!(
        (sent.len(), sent[0].url.as_str(), sent[0].operation.as_str()),
        (1, request::SUMMARY_URL, "cursor.usage-summary")
    );
}

#[test]
fn a_failed_summary_falls_back_to_the_period_with_its_reason() {
    let transport = Scripted::new(vec![
        Err(unavailable()),
        Ok(br#"{"displayMessage":"Pro plan"}"#.to_vec()),
    ]);
    let usage = run(fetch_usage(&transport, &Endpoints::default(), "cookie", "token", None)).unwrap();
    assert_eq!(usage.period_message.as_deref(), Some("Pro plan"));
    assert_eq!(
        usage.warning.as_deref(),
        Some("Dashboard summary unavailable: HTTP 503")
    );
    let sent = transport.sent.lock().unwrap();
    assert_eq!(sent[1].body, Body::Json(b"{}".to_vec()));
    assert_eq!(sent[1].headers[0], ("Authorization".into(), "Bearer token".into()));
}

#[test]
fn an_unreadable_summary_also_falls_back() {
    let transport = Scripted::new(vec![Ok(b"<html>".to_vec()), Ok(b"{}".to_vec())]);
    let usage = run(fetch_usage(&transport, &Endpoints::default(), "cookie", "token", None)).unwrap();
    assert_eq!(
        usage.warning.as_deref(),
        Some("Dashboard summary unavailable: Unexpected response: cursor.usage-summary: invalid payload")
    );
}

#[test]
fn period_failures_are_returned() {
    for (period, expected) in [
        (Err(unavailable()), unavailable()),
        (
            Ok(b"<html>".to_vec()),
            HttpError::Decoding("cursor.period-usage: invalid payload".into()),
        ),
    ] {
        let transport = Scripted::new(vec![Err(unavailable()), period]);
        let error = run(fetch_usage(&transport, &Endpoints::default(), "cookie", "token", None)).unwrap_err();
        assert_eq!(error, expected);
    }
}

#[test]
fn email_reads_the_profile_with_the_session_cookie() {
    let transport = Scripted::new(vec![Ok(br#"{"email":"me@example.com"}"#.to_vec())]);
    let email = run(fetch_email(&transport, request::PROFILE_URL, "cookie"));
    assert_eq!(email.as_deref(), Some("me@example.com"));
    let sent = transport.sent.lock().unwrap();
    assert_eq!(
        (sent[0].url.as_str(), sent[0].operation.as_str()),
        (request::PROFILE_URL, "cursor.me")
    );
    assert_eq!(sent[0].headers, header_list(request::cookie_headers("cookie")));
}

#[test]
fn email_is_unknown_when_the_lookup_fails() {
    let transport = Scripted::new(vec![Err(unavailable())]);
    assert_eq!(run(fetch_email(&transport, request::PROFILE_URL, "cookie")), None);
}

#[test]
fn email_is_unknown_when_the_profile_is_not_json() {
    let transport = Scripted::new(vec![Ok(b"<html>".to_vec())]);
    assert_eq!(run(fetch_email(&transport, request::PROFILE_URL, "cookie")), None);
}
