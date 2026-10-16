//! Checks Codex provider state through the public usage API.

use serde_json::json;

#[test]
fn presentation_keeps_credits_spend_and_reset_details() {
    let now = 1_780_000_000.0;
    let usage = crate::codex_usage::usage(
        &json!({
            "plan_type":"plus",
            "rate_limit":{"primary_window":{"used_percent":25,"limit_window_seconds":18000}},
            "credits":{"balance":"2.5"},
            "spend_control":{"individual_limit":{"used":"2","limit":"10"}}
        }),
        now,
    );
    let resets = json!({"available_count":2,"credits":[{
        "title":"Promo reset","status":"available","expires_at":"2026-10-31T00:00:00Z"
    }]});
    let provider = usage.into_provider(crate::codex_usage::reset_credits(&resets), "fixture".into(), now);
    assert_eq!(provider.plan, "Plus");
    assert_eq!(provider.windows[0].id, "session");
    assert_eq!(provider.credit_balance, Some(2.5));
    assert_eq!(
        provider.spend.as_ref().unwrap().credit_allowance.as_ref().unwrap().used,
        Some(2.0)
    );
    assert_eq!(provider.reset_credits.as_ref().unwrap().available, 2);
    assert_eq!(
        provider.reset_credits.as_ref().unwrap().details[0].status.as_deref(),
        Some("Available")
    );
}

#[test]
fn presentation_rejects_nonfinite_credit_amounts() {
    let provider = crate::codex_usage::usage(&json!({"credits":{"balance":"NaN"}}), 0.0).into_provider(
        None,
        "fixture".into(),
        0.0,
    );
    assert_eq!(provider.credit_balance, None);
}

#[test]
fn presentation_keeps_reset_deadlines_and_all_notice_kinds() {
    let now = 1_780_000_000.0;
    let provider = crate::codex_usage::usage(
        &json!({
            "rate_limit":{"primary_window":{
                "used_percent":25,"limit_window_seconds":18000,"reset_at":1780000100
            },"limit_reached":true},
            "rate_limit_reached_type":"tokens",
            "credits":{"balance":"2.5","overage_limit_reached":true},
            "spend_control":{"individual_limit":{"used":"2","limit":"10","reset_at":1780000200},
                "reached":true},
            "rate_limit_reset_credits":{"available_count":1,"credits":[{
                "id":"reset-1","status":"available","expires_at":"2026-10-31T00:00:00Z"
            }]},
            "promo":{"text":"Promotion active"},
            "rate_limit_upsell":{"text":"More capacity available"}
        }),
        now,
    )
    .into_provider(None, "fixture".into(), now);
    assert!(provider.windows[0].reset.is_some());
    assert!(provider.spend.as_ref().unwrap().reset.is_some());
    assert_eq!(provider.reset_credit_expiries[0].id, "reset-1");
    assert_eq!(provider.notices.len(), 5);
    assert!(
        provider
            .notices
            .iter()
            .any(|notice| notice.kind == quotaline_quota::NoticeKind::Promotion)
    );
    assert!(
        provider
            .notices
            .iter()
            .any(|notice| notice.kind == quotaline_quota::NoticeKind::Info)
    );
    assert!(
        provider
            .notices
            .iter()
            .any(|notice| notice.kind == quotaline_quota::NoticeKind::SpendControl)
    );
    assert!(
        provider
            .notices
            .iter()
            .any(|notice| notice.kind == quotaline_quota::NoticeKind::LimitReached)
    );
}

#[test]
fn presentation_works_without_credit_details() {
    let provider = crate::codex_usage::usage(&json!({}), 0.0).into_provider(None, "fixture".into(), 0.0);
    assert_eq!(provider.credit_balance, None);
    assert!(provider.usage_credits.is_none());
}

#[test]
fn presentation_warns_about_unreadable_model_limits() {
    let mut usage = crate::codex_usage::usage(&json!({}), 0.0);
    usage.dropped_additional_count = 2;
    assert_eq!(
        usage.into_provider(None, "fixture".into(), 0.0).warnings,
        ["Skipped 2 unreadable Codex model limits. Other reported quotas remain visible."]
    );
}

#[test]
fn presentation_keeps_the_plan_type_and_window_severity() {
    let provider = crate::codex_usage::usage(
        &json!({"plan_type":"plus","rate_limit":{"primary_window":{"used_percent":95,"limit_window_seconds":18000}}}),
        0.0,
    )
    .into_provider(None, "fixture".into(), 0.0);
    assert_eq!(provider.tier.as_deref(), Some("plus"));
    assert_eq!(provider.windows[0].severity.as_deref(), Some("critical"));
}
