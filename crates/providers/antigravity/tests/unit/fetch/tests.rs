//! Drives the shared Antigravity cloud requests through a scripted transport.

use futures_executor::block_on as run;
use serde_json::json;

use super::*;
use quotaline_http::test_transport::{Scripted, unavailable};

const BASES: [&str; 2] = ["https://primary.test", "https://secondary.test"];

fn quota() -> Value {
    json!({"groups":[{"buckets":[{"bucketId":"gemini-5h","remainingFraction":0.25}]}]})
}

fn fetch(replies: Vec<Result<Vec<u8>, HttpError>>) -> CloudQuota {
    run(fetch_quota(&Scripted::new(replies), &BASES, "secret", "Sign in.", None))
}

#[test]
fn quota_comes_from_the_first_endpoint_that_answers() {
    let result = fetch(vec![Err(unavailable()), Ok(quota().to_string().into_bytes())]);
    assert_eq!(result.result, Ok((quota(), BASES[1].to_string())));
    assert_eq!(result.debug_messages, ["antigravity: https://primary.test HTTP 503"]);
}

#[test]
fn an_expired_session_stops_at_the_first_endpoint() {
    for reply in [
        Ok(br#"{"code":16}"#.to_vec()),
        Err(HttpError::Response {
            status: 401,
            body: String::new(),
            retry_after: None,
        }),
    ] {
        let result = fetch(vec![reply]);
        assert!(matches!(result.result, Err(HttpError::Response { status: 401, .. })));
    }
}

#[test]
fn the_last_failure_is_reported_when_every_endpoint_fails() {
    let result = fetch(vec![Ok(br#"{"code":3}"#.to_vec()), Ok(br#"{"code":0}"#.to_vec())]);
    assert_eq!(
        result.result,
        Err(HttpError::Decoding(
            "Antigravity response contains no quota groups".into()
        ))
    );
    assert_eq!(
        result.debug_messages[0],
        "antigravity: https://primary.test Unexpected response: Antigravity quota request failed at the RPC layer"
    );
}

#[test]
fn plan_details_or_the_reason_they_are_missing() {
    for (reply, expected) in [
        (
            Ok(br#"{"currentTier":{"name":"Pro"}}"#.to_vec()),
            Ok(json!({"currentTier":{"name":"Pro"}})),
        ),
        (Err(unavailable()), Err("HTTP 503".to_string())),
        (
            Ok(b"<html>".to_vec()),
            Err("Unexpected response: antigravity.load-code-assist: invalid payload".to_string()),
        ),
    ] {
        assert_eq!(
            run(fetch_plan(&Scripted::new(vec![reply]), BASES[0], "secret", None)),
            expected
        );
    }
}
