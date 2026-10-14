//! Drives the shared Claude requests through a scripted transport.

use futures_executor::block_on as run;

use super::*;
use quotaline_http::test_transport::{Scripted, unavailable};

#[test]
fn usage_is_read_with_the_oauth_headers() {
    let transport = Scripted::new(vec![Ok(br#"{"five_hour":{}}"#.to_vec())]);
    let usage = run(fetch_usage(&transport, request::USAGE_URL, "secret")).unwrap();
    assert_eq!(usage, serde_json::json!({"five_hour":{}}));
    let sent = transport.sent.lock().unwrap();
    assert_eq!(
        (sent[0].operation.as_str(), &sent[0].headers[0]),
        ("claude.usage", &("Authorization".into(), "Bearer secret".into()))
    );
}

#[test]
fn usage_failures_are_errors() {
    let transport = Scripted::new(vec![Err(unavailable())]);
    assert_eq!(
        run(fetch_usage(&transport, request::USAGE_URL, "secret")).unwrap_err(),
        unavailable()
    );
}

#[test]
fn profiles_without_an_account_or_organization_are_warnings() {
    for (reply, expected) in [
        (Ok(br#"{"account":{}}"#.to_vec()), Ok(serde_json::json!({"account":{}}))),
        (
            Ok(br#"{"other":1}"#.to_vec()),
            Err("Provider returned an invalid profile".to_string()),
        ),
        (Err(unavailable()), Err("HTTP 503".to_string())),
    ] {
        let transport = Scripted::new(vec![reply]);
        assert_eq!(
            run(fetch_profile(&transport, request::PROFILE_URL, "secret", None)),
            expected
        );
    }
}
