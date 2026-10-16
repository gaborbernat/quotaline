//! Checks the public behavior of the adjacent module.

use super::*;
use serde_json::json;

#[test]
fn nested_rollout_normalizes_windows_and_identity() {
    let result = rollout(&json!({
        "timestamp":"2026-10-10T10:00:00Z",
        "payload":[{"rate_limits":false}, {"event":{"rate_limits":{
            "plan_type":"pro",
            "rate_limit_reached_type":{"type":"usage"},
            "primary":{"used_percent":"42","window_minutes":"300",
                "resets_in_seconds":"5","resets_at":"1791630000","reset_at":1},
            "secondary":{"used_percent":7,"limit_window_seconds":604800,"reset_at":"200"},
            "credits":{"balance":"9.5"}
        }}}]
    }))
    .unwrap();
    let mapped = usage(&result.data, 100.0);
    assert_eq!(result.observed_at, Some(1_791_626_400.0));
    assert_eq!(mapped.plan_type.as_deref(), Some("pro"));
    assert_eq!(
        mapped.groups[0].primary.as_ref().unwrap().duration_seconds,
        Some(18_000.0)
    );
    assert_eq!(
        mapped.groups[0].primary.as_ref().unwrap().reset_at,
        Some(1_791_630_000.0)
    );
    assert_eq!(
        mapped.groups[0].primary.as_ref().unwrap().reset_after_seconds,
        Some(5.0)
    );
    assert_eq!(mapped.groups[0].secondary.as_ref().unwrap().reset_at, Some(200.0));
    assert!(mapped.groups[0].limit_reached);
    assert_eq!(mapped.credits.unwrap().balance.as_deref(), Some("9.5"));
    assert_eq!(mapped.notices[0].text, "Limit reached: Usage.");
}

#[test]
fn rollout_accepts_optional_windows_and_numeric_timestamp() {
    let result = rollout(&json!({
        "timestamp":200,
        "rate_limits":{"primary":{"window_minutes":5},
            "secondary":{"used_percent":35,"limit_window_seconds":"60",
                "reset_at":300}}
    }))
    .unwrap();
    assert_eq!(result.observed_at, Some(200.0));
    let mapped = usage(&result.data, 100.0);
    assert_eq!(mapped.groups[0].primary, None);
    assert_eq!(
        mapped.groups[0].secondary.as_ref().unwrap().duration_seconds,
        Some(60.0)
    );
    assert_eq!(mapped.groups[0].secondary.as_ref().unwrap().reset_at, Some(300.0));
}

#[test]
fn rollout_ignores_invalid_shapes_and_unusable_numbers() {
    for document in [
        json!(null),
        json!([false, "other"]),
        json!({"rate_limits":"invalid"}),
        json!({"payload":{"rate_limits":[]}}),
    ] {
        assert_eq!(rollout(&document), None);
    }
    let result = rollout(&json!({
        "timestamp":"invalid",
        "rate_limits":{"primary":{"used_percent":"NaN"},"secondary":false}
    }))
    .unwrap();
    assert_eq!(result.observed_at, None);
    assert!(usage(&result.data, 0.0).groups[0].primary.is_none());
    assert!(usage(&result.data, 0.0).groups[0].secondary.is_none());
}

#[test]
fn plan_names_cover_supported_tiers_and_humanized_fallbacks() {
    for (tier, expected) in [
        (None, "ChatGPT"),
        (Some(""), "ChatGPT"),
        (Some("pro"), "Pro (More)"),
        (Some("prolite"), "Pro"),
        (Some("promax"), "Pro (Max)"),
        (Some("plus"), "Plus"),
        (Some("go"), "Go"),
        (Some("free"), "Free"),
        (Some("team"), "Team"),
        (Some("free_workspace"), "Team"),
        (Some("business"), "Business"),
        (Some("self_serve_business_prolite"), "Business"),
        (Some("enterprise"), "Enterprise"),
        (Some("edu"), "Education"),
        (Some("education"), "Education"),
        (Some("k12"), "K12"),
        (Some("quorum_plus"), "Quorum Plus"),
        (Some("new-plan"), "New Plan"),
        (Some("-"), ""),
    ] {
        assert_eq!(plan_name(tier), expected);
    }
    assert_eq!(plan_name(Some("PRO")), "Pro (More)");
}

#[test]
fn empty_usage_has_no_display_data() {
    let result = usage(&json!({}), 100.0);
    assert_eq!(result.plan_type, None);
    assert_eq!(result.email, None);
    assert_eq!(result.groups.len(), 2);
    assert!(result.groups.iter().all(|group| group.primary.is_none()));
    assert_eq!(result.credits, None);
    assert_eq!(result.spend, None);
    assert!(result.notices.is_empty());
    assert_eq!(result.reset_credits, None);
    assert_eq!(result.dropped_additional_count, 0);
}

#[test]
fn usage_projects_windows_credits_spend_notices_and_resets() {
    let result = usage(
        &json!({
            "plan_type":"pro", "email":"user@example.com",
            "rate_limit":{"limit_reached":true,
                "primary_window":{"used_percent":80,"limit_window_seconds":18000,"reset_at":200},
                "secondary_window":{"used_percent":"60","reset_after_seconds":30}},
            "code_review_rate_limit":{"primary_window":{"used_percent":15}},
            "additional_rate_limits":[
                {"limit_name":"Spark","rate_limit":{"primary_window":{"used_percent":95}}},
                {"name":"Fallback","rate_limit":{"secondary_window":{"used_percent":40}}},
                {"rate_limit":{"primary_window":{"used_percent":8}}}, false],
            "credits":{"balance":"42.5","unlimited":false,"has_credits":true,
                "overage_limit_reached":true,"approx_local_messages":[3,5],"approx_cloud_messages":[2,4]},
            "spend_control":{"reached":true,"individual_limit":{
                "used":"2.5","limit":"10","remaining":"7.5","source":"owner",
                "used_percent":25,"remaining_percent":75,"reset_at":500,"reset_after_seconds":30}},
            "rate_limit_reached_type":{"type":"workspace_owner_credits_depleted"},
            "promo":{"text":"Promo"}, "rate_limit_upsell":{"title":"Upgrade"},
            "rate_limit_reset_credits":{"available_count":2,"applicable_available_count":1,
                "total_earned_count":4,"immediate_reset_purchase_eligible":true,
                "credits":[{"id":"bonus","title":"Extra","status":"available","reset_type":"session_reset",
                    "description":"One reset","granted_at":"2026-10-01T00:00:00Z","expires_at":200},
                    {"status":"used","expires_at":300}]}
        }),
        100.0,
    );
    assert_eq!(result.plan_type.as_deref(), Some("pro"));
    assert_eq!(result.email.as_deref(), Some("user@example.com"));
    assert_eq!(result.groups.len(), 5);
    assert_eq!(result.groups[0].primary.as_ref().unwrap().reset_at, Some(200.0));
    assert_eq!(result.groups[0].secondary.as_ref().unwrap().used_percent, 60.0);
    assert_eq!(result.groups[3].kind, LimitKind::Additional("Fallback".into()));
    assert_eq!(result.groups[4].kind, LimitKind::Additional("Additional".into()));
    assert_eq!(result.dropped_additional_count, 1);
    let credits = result.credits.unwrap();
    assert_eq!(credits.balance.as_deref(), Some("42.5"));
    assert_eq!(credits.local_messages, Some((3, 5)));
    assert_eq!(credits.cloud_messages, Some((2, 4)));
    assert!(credits.overage_limit_reached);
    let spend = result.spend.unwrap();
    assert_eq!(spend.percent, Some(25.0));
    assert_eq!(spend.resets_at, Some(500.0));
    assert_eq!(spend.used.as_deref(), Some("2.5"));
    assert_eq!(spend.source.as_deref(), Some("owner"));
    assert_eq!(
        result.notices.iter().map(|notice| &notice.kind).collect::<Vec<_>>(),
        [
            &NoticeKind::LimitReached,
            &NoticeKind::SpendControl,
            &NoticeKind::Promotion,
            &NoticeKind::Info,
            &NoticeKind::SpendControl,
        ]
    );
    assert_eq!(
        result.notices[0].text,
        "Limit reached: Workspace Owner Credits Depleted."
    );
    let resets = result.reset_credits.unwrap();
    assert!(resets.complete());
    assert_eq!(
        (resets.available, resets.applicable, resets.total_earned),
        (2, 1, Some(4))
    );
    let entries = resets.credits.unwrap();
    assert_eq!(entries[0].id, "bonus");
    assert_eq!(entries[0].usable_expiry, Some(200.0));
    assert_eq!(entries[0].granted_at, Some(1_790_812_800.0));
    assert_eq!(entries[1].id, "credit-1");
    assert_eq!(entries[1].usable_expiry, None);
}

#[test]
fn usage_falls_back_to_relative_reset_and_remaining_percent() {
    let result = usage(
        &json!({
            "rate_limit":{"primary_window":{"used_percent":5,"reset_after_seconds":60}},
            "spend_control":{"individual_limit":{"remaining_percent":75,"reset_after_seconds":30}},
            "credits":{"balance":12.5,"approx_local_messages":[5,2],"approx_cloud_messages":[3]},
            "rate_limit_upsell":{"text":"More"}
        }),
        100.0,
    );
    assert_eq!(
        result.groups[0].primary.as_ref().unwrap().reset_after_seconds,
        Some(60.0)
    );
    assert_eq!(result.spend.unwrap().resets_at, Some(130.0));
    assert_eq!(result.credits.unwrap().local_messages, None);
    assert_eq!(result.notices[0].text, "More");
}

#[test]
fn spend_accepts_iso_reset_and_keeps_absolute_time_before_relative() {
    let result = usage(
        &json!({"spend_control":{"individual_limit":{
            "reset_at":"2026-10-01T00:00:00Z","reset_after_seconds":30
        }}}),
        100.0,
    );
    assert_eq!(result.spend.unwrap().resets_at, Some(1_790_812_800.0));
}

#[test]
fn malformed_optional_data_is_dropped_without_hiding_other_quotas() {
    let result = usage(
        &json!({
            "rate_limit":{"limit_reached":true,"primary_window":{"used_percent":"NaN"}},
            "additional_rate_limits":false,
            "credits":{"approx_local_messages":[],"approx_cloud_messages":["x",2]},
            "promo":{"pct":2}, "rate_limit_reached_type":"odd"
        }),
        100.0,
    );
    assert_eq!(result.dropped_additional_count, 1);
    assert_eq!(result.groups[0].primary, None);
    assert_eq!(result.credits.unwrap().cloud_messages, None);
    assert_eq!(result.notices[0].text, "Limit reached: Odd.");
    assert_eq!(result.notices[1].text, "pct: 2");
}

#[test]
fn plain_limit_reached_notice_and_invalid_message_ranges() {
    let result = usage(
        &json!({
            "rate_limit":{"limit_reached":true},
            "credits":{"approx_local_messages":true,"approx_cloud_messages":[1,"x"]}
        }),
        0.0,
    );
    assert_eq!(result.notices[0].text, "Usage limit reached.");
    assert_eq!(result.credits.unwrap().local_messages, None);
}

#[test]
fn reset_credits_accepts_epoch_and_iso_and_marks_only_usable_expiry() {
    let result = reset_credits(&json!({"available_count":3,"credits":[
        {"id":"a","expires_at":5},
        {"id":"b","status":"available","expires_at":"2026-09-03T10:00:00Z"},
        {"id":"c","status":"used","expires_at":10},
        {"expires_at":15},
        {"id":"d","expires_at":"soon"},
        {"id":"e","expires_at":true}
    ]}))
    .unwrap();
    assert!(!result.complete());
    assert_eq!(result.applicable, 3);
    let entries = result.credits.unwrap();
    assert_eq!(entries[0].usable_expiry, Some(5.0));
    assert_eq!(entries[1].usable_expiry, entries[1].expires_at);
    assert_eq!(entries[2].usable_expiry, None);
    assert_eq!(entries[3].usable_expiry, None);
    assert_eq!(entries[4].expires_at, None);
    assert_eq!(entries[4].expires_display.as_deref(), Some("soon"));
    assert_eq!(entries[5].expires_at, None);
    assert_eq!(reset_credits(&Value::Null), None);
    assert_eq!(reset_credits(&json!({"credits":[]})).unwrap().credits, Some(vec![]));
}

#[test]
fn odd_notice_values_preserve_readable_content() {
    let result = usage(
        &json!({
            "rate_limit_reached_type":{"nested":[true,false,null,"x",{"y":1}]},
            "promo":[1,2], "rate_limit_upsell":{"title":"Fallback"}
        }),
        0.0,
    );
    assert_eq!(
        result.notices[0].text,
        "Limit reached: Nested: true, false, null, x, y: 1."
    );
    assert_eq!(result.notices[1].text, "1, 2");
    assert_eq!(result.notices[2].text, "Fallback");
}
