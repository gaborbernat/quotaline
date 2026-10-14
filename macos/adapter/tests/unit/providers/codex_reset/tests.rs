//! Tests the Codex reading macOS receives: usage, account fallbacks, reset credits, and reset confirmation.

use super::*;
use crate::http::HttpErrorRecord;
use crate::mock_http::MockTransport;
use serde_json::json;

const NOW: f64 = 1_791_864_000.0;

fn window(used: f64) -> Window {
    Window::new(
        "session",
        "Session",
        used,
        Some("2026-10-13T05:00:00+00:00".into()),
        "session",
    )
}

#[test]
fn clear_discards_pending_reset_confirmation() {
    let guard = CodexResetGuard::new();
    assert_eq!(guard.confirm(vec![window(42.0)], NOW)[0].used, 42.0);
    assert_eq!(guard.confirm(vec![window(0.0)], NOW)[0].detail, "Confirming reset");
    guard.clear();
    assert_eq!(guard.confirm(vec![window(0.0)], NOW)[0].detail, "");
}

fn input() -> CodexUsageFetchInput {
    CodexUsageFetchInput {
        token: "secret".into(),
        account_id: Some("account".into()),
        account: CodexAccountFacts {
            fingerprint: Some("fingerprint".into()),
            plan_type: Some("pro".into()),
            email: Some("user@example.com".into()),
            subscription_active_until: Some("2026-11-01T00:00:00Z".into()),
        },
        now: NOW,
        hint: "Sign in".into(),
        home: String::new(),
    }
}

fn fetch(usage: serde_json::Value, resets: Result<serde_json::Value, HttpErrorRecord>) -> CodexUsageFetchOutcome {
    let mut transport = MockTransport::new();
    transport.expect_start_get().returning(move |request, completion| {
        assert_eq!(request.headers.get("Authorization"), Some(&"Bearer secret".into()));
        assert_eq!(request.headers.get("ChatGPT-Account-Id"), Some(&"account".into()));
        match request.operation.as_str() {
            "codex.usage" => completion.succeed(serde_json::to_vec(&usage).unwrap()),
            _ => match resets.clone() {
                Ok(body) => completion.succeed(serde_json::to_vec(&body).unwrap()),
                Err(error) => completion.fail(error),
            },
        }
    });
    futures_executor::block_on(CodexResetGuard::new().fetch_usage(
        input(),
        Arc::new(CodexResetCreditCache::new()),
        Arc::new(transport),
    ))
}

fn reading(outcome: CodexUsageFetchOutcome) -> Option<ProviderRecord> {
    match outcome {
        CodexUsageFetchOutcome::Provider(provider) => Some(*provider),
        _ => None,
    }
}

fn provider(usage: serde_json::Value, resets: Result<serde_json::Value, HttpErrorRecord>) -> ProviderRecord {
    reading(fetch(usage, resets)).unwrap()
}

#[test]
fn response_identity_wins_over_the_stored_account() {
    let provider = provider(
        json!({"plan_type":"team","email":"response@example.com","rate_limit":{"primary_window":{"used_percent":40}}}),
        Ok(json!({"available_count":0})),
    );
    assert_eq!(
        (
            provider.plan.as_str(),
            provider.tier.as_deref(),
            provider.identity.as_str()
        ),
        ("Team", Some("team"), "response@example.com")
    );
    assert_eq!(provider.windows[0].used, 40.0);
}

#[test]
fn stored_account_fills_what_the_response_omits() {
    let provider = provider(json!({}), Ok(json!({"available_count":0})));
    assert_eq!(
        (provider.plan.as_str(), provider.identity.as_str()),
        ("Pro (More)", "user@example.com")
    );
    assert_eq!(provider.account_fingerprint.as_deref(), Some("fingerprint"));
    assert_eq!(
        provider.subscription_active_until.as_deref(),
        Some("2026-11-01T00:00:00Z")
    );
}

#[test]
fn supplemental_reset_credits_reach_the_reading() {
    let provider = provider(json!({}), Ok(json!({"available_count":3})));
    assert_eq!(provider.reset_credits.unwrap().available, 3);
    assert!(provider.warnings.is_empty());
}

#[test]
fn a_failed_reset_credit_lookup_becomes_a_warning() {
    let provider = provider(
        json!({}),
        Err(HttpErrorRecord::Network {
            detail: "offline".into(),
        }),
    );
    assert_eq!(provider.warnings, ["Reset credits unavailable: Network error: offline"]);
}

#[test]
fn invalid_usage_payload_is_a_failure() {
    let mut transport = MockTransport::new();
    transport
        .expect_start_get()
        .once()
        .return_once(|_, completion| completion.succeed(b"{".to_vec()));
    let outcome = futures_executor::block_on(CodexResetGuard::new().fetch_usage(
        input(),
        Arc::new(CodexResetCreditCache::new()),
        Arc::new(transport),
    ));
    assert!(reading(outcome.clone()).is_none());
    assert_eq!(
        outcome,
        CodexUsageFetchOutcome::Failure(ProviderFailure {
            kind: crate::http::ProviderFailureKind::Failed,
            message: "Unexpected response: codex.usage: invalid payload".into(),
            retry_after: None,
        })
    );
}

#[test]
fn authentication_failure_asks_to_sign_in() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().once().return_once(|_, completion| {
        completion.fail(HttpErrorRecord::Response {
            status: 401,
            body: String::new(),
            retry_after: None,
        });
    });
    assert_eq!(
        futures_executor::block_on(CodexResetGuard::new().fetch_usage(
            input(),
            Arc::new(CodexResetCreditCache::new()),
            Arc::new(transport)
        )),
        CodexUsageFetchOutcome::Failure(ProviderFailure {
            kind: crate::http::ProviderFailureKind::NotAuthenticated,
            message: "HTTP 401. Sign in".into(),
            retry_after: None,
        })
    );
}
