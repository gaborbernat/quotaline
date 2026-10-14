//! Checks the public behavior of the adjacent module.

use super::*;
use serde_json::json;

fn provider(data: Value) -> Provider {
    project(&data, "fixture".into(), String::new())
}

#[test]
fn paid_quotas_keep_order_reset_precision_and_overage() {
    let result = provider(json!({
        "copilot_plan":"individual_pro", "access_type_sku":"copilot_pro_seat",
        "organization_login_list":["octo-org", false, "octo-labs"],
        "token_based_billing":true,
        "quota_reset_date":"2026-09-01",
        "quota_snapshots":{
            "premium_interactions":{"entitlement":"100","quota_remaining":-15,
                "overage_count":"15","overage_permitted":true,"credits_used":50},
            "chat":{"entitlement":100,"remaining":25,"quota_reset_at":1788558705},
            "completions":{"unlimited":true,"overage_permitted":false},
            "zeta":{"percent_remaining":30}
        }
    }));
    assert_eq!(result.plan, "Pro+ via octo-org, octo-labs");
    assert_eq!(result.tier.as_deref(), Some("copilot_pro_seat"));
    assert_eq!(
        result
            .windows
            .iter()
            .map(|window| window.id.as_str())
            .collect::<Vec<_>>(),
        ["premium_interactions", "chat", "zeta"]
    );
    assert_eq!(result.windows[0].used, 100.0);
    assert_eq!(result.windows[0].reset_precision, ResetPrecision::Day);
    assert_eq!(result.windows[0].reset.as_deref(), Some("2026-09-01T00:00:00Z"));
    assert_eq!(result.windows[1].reset_precision, ResetPrecision::Instant);
    assert_eq!(result.windows[1].reset.as_deref(), Some("2026-09-04T21:51:45+00:00"));
    assert_eq!(result.windows[2].used, 70.0);
    assert_eq!(
        result.notices,
        vec![Notice::new(
            NoticeKind::Info,
            "Premium credits: quota exceeded, 15 overage credits."
        )]
    );
    assert_eq!(result.details[0].id, "quota:chat");
    assert_eq!(result.details[1].value, "Unlimited · Overage off");
    assert_eq!(result.details[2].id, "quota:premium_interactions");
    assert!(result.details[2].value.contains("Credits used: 50 credits"));
}

#[test]
fn free_quotas_use_monthly_allowances_without_duplicate_windows() {
    let result = provider(json!({
        "copilot_plan":"individual", "quota_reset_date_utc":"invalid",
        "quota_reset_date":"2026-09-01T10:00:00Z",
        "limited_user_reset_date":"2026-09-11",
        "quota_snapshots":{"premium_interactions":{"percent_remaining":40},
            "chat":{"entitlement":0,"remaining":0}},
        "limited_user_quotas":{"chat":"41","completions":50,"premium_interactions":1,"other":1},
        "monthly_quotas":{"chat":50,"completions":50,"premium_interactions":10,"other":0}
    }));
    assert_eq!(
        result
            .windows
            .iter()
            .map(|window| window.id.as_str())
            .collect::<Vec<_>>(),
        ["premium_interactions", "free:chat", "free:completions"]
    );
    assert_eq!(result.windows[0].used, 60.0);
    assert_eq!(result.windows[0].reset_precision, ResetPrecision::Instant);
    assert!((result.windows[1].used - 18.0).abs() < 0.001);
    assert_eq!(result.windows[1].reset_precision, ResetPrecision::Day);
    assert_eq!(result.windows[2].used, 0.0);
    assert_eq!(result.plan, "Pro");
}

#[test]
fn malformed_quota_fields_do_not_hide_other_categories() {
    let result = provider(json!({
        "access_type_sku":"copilot_for_business_seat",
        "quota_reset_date":"whenever",
        "quota_snapshots":{
            "chat":{"percent_remaining":-10,"overage_permitted":false,
                "entitlement":"NaN","quota_reset_at":"invalid","overage_count":null},
            "premium_interactions":{"entitlement":0,"remaining":0},
            "completions":{"unlimited":true},
            "custom":{"entitlement":10,"quota_remaining":"NaN"}
        }
    }));
    assert_eq!(result.plan, "Copilot For Business Seat");
    assert_eq!(result.windows.len(), 1);
    assert_eq!(result.windows[0].id, "chat");
    assert_eq!(result.windows[0].reset, None);
    assert_eq!(
        result.notices,
        vec![Notice::new(
            NoticeKind::LimitReached,
            "Chat: quota exceeded, 0 overage requests."
        )]
    );
    assert_eq!(result.details[0].value, "Overage off");
    assert!(result.details.iter().any(|detail| detail.title == "Completions"));
}

#[test]
fn empty_response_has_only_the_default_identity() {
    let result = provider(json!({}));
    assert_eq!(result.plan, "Copilot");
    assert_eq!(result.tier, None);
    assert!(result.windows.is_empty());
    assert!(result.details.is_empty());
    assert!(result.notices.is_empty());
    assert_eq!(provider(json!(false)).plan, "Copilot");
}

#[test]
fn custom_overages_and_missing_monthly_limits_do_not_claim_a_quota() {
    let result = provider(json!({
        "quota_snapshots":{"custom":{"percent_remaining":-5}},
        "limited_user_quotas":{"chat":2},"monthly_quotas":{}
    }));
    assert_eq!(result.windows[0].id, "custom");
    assert_eq!(result.windows[0].used, 100.0);
    assert_eq!(result.windows.len(), 1);
    assert!(result.notices.is_empty());
}

#[test]
fn supported_plans_and_empty_organization_list_have_exact_names() {
    for (sku, name) in [
        ("individual", "Pro"),
        ("individual_pro", "Pro+"),
        ("individual_max", "Max"),
        ("individual_edu", "Student"),
        ("team-seat", "Team Seat"),
    ] {
        assert_eq!(
            provider(json!({"copilot_plan":sku,"organization_login_list":[]})).plan,
            name
        );
    }
}

#[test]
fn detail_values_preserve_units_and_fraction_precision() {
    let result = provider(json!({
        "quota_snapshots":{"premium_interactions":{
            "entitlement":10.1234567,"quota_remaining":4.5,"credits_used":2,
            "overage_count":1,"overage_entitlement":3,"overage_permitted":false
        }}
    }));
    assert_eq!(
        result.details[0].value,
        "Included: 10.123457 requests · Remaining: 4.5 requests · Credits used: 2 credits · Overage used: 1 requests · Overage allowance: 3 requests · Overage off"
    );
    assert_eq!(result.details[0].title, "Premium requests");
    assert_eq!(result.details[0].id, "quota:premium_interactions");
}

#[test]
fn provider_keeps_source_and_identity() {
    let result = project(&json!({}), "fixture".into(), "octocat".into());
    assert_eq!(
        (result.source.as_str(), result.identity.as_str()),
        ("fixture", "octocat")
    );
}
