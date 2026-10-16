//! Drives the shared Gemini plan and quota requests through a scripted transport.

use futures_executor::block_on as run;
use serde_json::json;

use super::*;
use quotaline_http::test_transport::{Scripted, unavailable};

fn context() -> Context<'static> {
    Context {
        base: request::BASE_URL,
        headers: vec![("Authorization".into(), "Bearer secret".into())],
        fingerprint: "account",
        now: 1_000.0,
        hint: "Run gemini.",
        home: None,
    }
}

fn status(status: i32, body: &str) -> HttpError {
    HttpError::Response {
        status,
        body: body.into(),
        retry_after: None,
    }
}

fn run_with(cache: &Mutex<AssistCache<Value>>, replies: Vec<Result<Vec<u8>, HttpError>>) -> (Fetch, Vec<String>) {
    let transport = Scripted::new(replies);
    let fetch = run(super::fetch(&transport, cache, context()));
    let operations = transport
        .sent
        .lock()
        .unwrap()
        .iter()
        .map(|request| request.operation.clone())
        .collect();
    (fetch, operations)
}

#[test]
fn quota_uses_the_plan_project_and_caches_the_plan() {
    let cache = Mutex::new(AssistCache::default());
    let assist = json!({"currentTier":{"id":"standard"},"cloudaicompanionProject":"project-1"});
    let (fetch, operations) = run_with(
        &cache,
        vec![Ok(assist.to_string().into_bytes()), Ok(br#"{"buckets":[]}"#.to_vec())],
    );
    assert_eq!(
        fetch,
        Fetch {
            outcome: Outcome::Quota {
                quota: json!({"buckets":[]}),
                assist: assist.clone()
            },
            warnings: Vec::new()
        }
    );
    assert_eq!(operations, ["gemini.load-code-assist", "gemini.quota"]);
    assert_eq!(cache.lock().unwrap().read("account", 1_001.0), Some(assist));
}

#[test]
fn a_cached_plan_skips_its_request() {
    let cache = Mutex::new(AssistCache::default());
    cache
        .lock()
        .unwrap()
        .record("account", json!({"currentTier":{}}), false, 1_000.0);
    let (_, operations) = run_with(&cache, vec![Ok(b"{}".to_vec())]);
    assert_eq!(operations, ["gemini.quota"]);
}

#[test]
fn sign_in_and_rate_limit_plan_failures_stop_the_fetch() {
    for code in [401, 429] {
        let (fetch, operations) = run_with(&Mutex::new(AssistCache::default()), vec![Err(status(code, ""))]);
        assert_eq!(fetch.outcome, Outcome::Failure(status(code, "")));
        assert_eq!(operations, ["gemini.load-code-assist"]);
    }
}

#[test]
fn other_plan_failures_leave_a_warning_and_still_read_quota() {
    let (fetch, _) = run_with(
        &Mutex::new(AssistCache::default()),
        vec![Err(unavailable()), Ok(b"{}".to_vec())],
    );
    assert_eq!(fetch.warnings, ["Plan details unavailable: HTTP 503"]);
    assert_eq!(
        fetch.outcome,
        Outcome::Quota {
            quota: json!({}),
            assist: Value::Null
        }
    );
}

#[test]
fn ineligible_plans_and_subscription_errors_are_unsupported() {
    let ineligible = json!({"ineligibleTiers":[{"reasonMessage":"Not eligible"}]})
        .to_string()
        .into_bytes();
    let (fetch, _) = run_with(&Mutex::new(AssistCache::default()), vec![Ok(ineligible)]);
    assert_eq!(fetch.outcome, Outcome::Unsupported("Not eligible".into()));
    let (fetch, _) = run_with(
        &Mutex::new(AssistCache::default()),
        vec![
            Ok(br#"{"currentTier":{}}"#.to_vec()),
            Err(status(403, r#"{"error":{"status":"SUBSCRIPTION_REQUIRED"}}"#)),
        ],
    );
    assert_eq!(fetch.outcome, Outcome::Unsupported(UNSUPPORTED_CLIENT_MESSAGE.into()));
}

#[test]
fn other_quota_failures_are_reported() {
    for (reply, expected) in [
        (Err(status(403, "denied")), status(403, "denied")),
        (
            Ok(b"<html>".to_vec()),
            HttpError::Decoding("gemini.quota: invalid payload".into()),
        ),
    ] {
        let (fetch, _) = run_with(
            &Mutex::new(AssistCache::default()),
            vec![Ok(br#"{"currentTier":{}}"#.to_vec()), reply],
        );
        assert_eq!(fetch.outcome, Outcome::Failure(expected));
    }
}
