//! Checks Codex readings built from usage documents and local rollouts.

use super::*;
use serde_json::json;

fn account() -> CodexAccountFacts {
    CodexAccountFacts {
        fingerprint: Some("fingerprint".into()),
        plan_type: Some("pro".into()),
        email: Some("token@example.com".into()),
        subscription_active_until: None,
    }
}

#[test]
fn reading_keeps_every_display_field_of_the_usage_document() {
    let usage = quotaline_codex::codex_usage::usage(
        &json!({
            "plan_type":"plus","email":"response@example.com",
            "rate_limit":{"primary_window":{"used_percent":25}},
            "credits":{"balance":"12.5","has_credits":true,"approx_local_messages":[2,3],
                "approx_cloud_messages":[4,5]},
            "spend_control":{"reached":true,"individual_limit":{"used":"1","limit":"2",
                "remaining":"1","source":"owner","used_percent":50,"reset_at":200}},
            "promo":{"title":"Offer"},
            "rate_limit_reset_credits":{"available_count":1,"total_earned_count":2,
                "immediate_reset_purchase_eligible":true,
                "credits":[{"id":"bonus","title":"Reset","status":"available","reset_type":"session",
                    "description":"Extra","granted_at":5,"expires_at":100}]}
        }),
        10.0,
    );
    let provider = ProviderRecord::from(account_provider(usage, None, &account(), 10.0));
    assert_eq!(
        (provider.plan.as_str(), provider.tier.as_deref()),
        ("Plus", Some("plus"))
    );
    assert_eq!(provider.identity, "response@example.com");
    assert_eq!(provider.windows[0].used, 25.0);
    let credits = provider.usage_credits.unwrap();
    assert_eq!(
        (credits.local_messages.unwrap().low, provider.credit_balance),
        (2, Some(12.5))
    );
    let spend = provider.spend.unwrap();
    assert_eq!(spend.percent, Some(50.0));
    assert_eq!(spend.credit_allowance.unwrap().source.as_deref(), Some("owner"));
    assert_eq!(provider.notices[1].text, "Offer");
    assert_eq!(provider.reset_credits.unwrap().details[0].id, "bonus");
    assert_eq!(provider.reset_credit_expiries[0].at, 100);
}

#[test]
fn rollout_reading_keeps_the_logged_values_and_observed_time() {
    let rollout = codex_rollout(
        serde_json::to_vec(&json!({
            "timestamp":"2026-10-10T10:00:00Z",
            "payload":{"rate_limits":{"plan_type":"pro",
                "primary":{"used_percent":42,"window_minutes":300,"resets_in_seconds":30},
                "credits":{"balance":"9.5"}}}
        }))
        .unwrap(),
        100.0,
    )
    .unwrap();
    assert_eq!(rollout.observed_at(), Some(1_791_626_400.0));
    let provider = rollout.provider(account());
    assert_eq!(provider.plan, "Pro (More)");
    assert_eq!(provider.windows[0].used, 42.0);
    assert_eq!(provider.windows[0].reset.as_deref(), Some("1970-01-01T00:02:10+00:00"));
    assert_eq!(provider.credit_balance, Some(9.5));
}

#[test]
fn rollout_without_a_plan_uses_the_stored_account() {
    let rollout = codex_rollout(
        serde_json::to_vec(&json!({"payload":{"rate_limits":{"primary":{"used_percent":1}}}})).unwrap(),
        0.0,
    )
    .unwrap();
    let provider = rollout.provider(account());
    assert_eq!(
        (provider.plan.as_str(), provider.identity.as_str()),
        ("Pro (More)", "token@example.com")
    );
    assert_eq!(provider.account_fingerprint.as_deref(), Some("fingerprint"));
}

#[test]
fn rollout_rejects_documents_without_rate_limits() {
    for payload in [b"{".as_slice(), b"[]", b"{}", b"{\"rate_limits\":false}"] {
        assert!(codex_rollout(payload.to_vec(), 0.0).is_none());
    }
}

#[test]
fn local_fallback_follows_the_shared_rule() {
    use crate::http::ProviderFailureKind;
    assert_eq!(
        [
            ProviderFailureKind::NotAuthenticated,
            ProviderFailureKind::NetworkUnavailable,
            ProviderFailureKind::RateLimited,
            ProviderFailureKind::Failed,
        ]
        .map(codex_local_fallback),
        [true, true, false, false]
    );
}
