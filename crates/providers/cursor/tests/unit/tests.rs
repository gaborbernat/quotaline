//! Checks the public behavior of the adjacent module.

use super::*;
use serde_json::json;

fn minor(money: Option<&MoneyAmount>) -> Option<i64> {
    money.map(|money| money.amount_minor)
}

#[test]
fn summary_maps_aggregate_quotas_spend_and_details() {
    let result = project(
        &json!({
            "billingCycleStart":"2026-08-10T00:00:00Z",
            "billingCycleEnd":"2026-09-10T00:00:00Z",
            "membershipType":"pro_plus",
            "individualUsage":{
                "plan":{"enabled":true,"totalPercentUsed":30,"autoPercentUsed":12,
                    "apiPercentUsed":48,"used":3000,"limit":10000,
                    "breakdown":{"included":5000,"bonus":2000,"total":7000}},
                "onDemand":{"enabled":true,"used":500,"limit":10000,"remaining":9500},
                "overall":{"used":25,"limit":100}
            },
            "teamUsage":{"pooled":{"totalPercentUsed":25,"enabled":false},
                "onDemand":{"totalPercentUsed":15,"enabled":true}},
            "limitType":"monthly","autoModelSelectedDisplayMessage":"Auto pool",
            "namedModelSelectedDisplayMessage":"Named pool"
        }),
        Some("fallback"),
        Some("cached@example.com"),
        Some("account@example.com"),
        None,
        "fixture".into(),
    );
    assert_eq!(result.plan, "Pro Plus");
    assert_eq!(result.identity, "account@example.com");
    assert_eq!(result.source, "fixture");
    assert_eq!(
        result
            .windows
            .iter()
            .map(|window| window.id.as_str())
            .collect::<Vec<_>>(),
        ["plan", "on_demand", "overall", "team_on_demand"]
    );
    assert_eq!(result.windows[0].used, 30.0);
    assert_eq!(result.windows[0].reset.as_deref(), Some("2026-09-10T00:00:00Z"));
    assert_eq!(result.windows[0].duration_seconds, Some(2_678_400));
    assert_eq!(result.windows[0].detail, "Auto 12% · API 48%");
    assert_eq!(result.windows[1].detail, "");
    let spend = result.spend.as_ref().unwrap();
    assert_eq!(minor(spend.used_money.as_ref()), Some(500));
    assert_eq!(minor(spend.limit_money.as_ref()), Some(10_000));
    assert_eq!((spend.used, spend.limit), (Some(5.0), Some(100.0)));
    assert_eq!(spend.percent, Some(5.0));
    assert_eq!(spend.reset.as_deref(), Some("2026-09-10T00:00:00Z"));
    assert!(!spend.limit_reached);
    assert_eq!(result.details[0].id, "plan");
    assert!(result.details[0].value.contains("Included: $50.00"));
    assert_eq!(result.details[1].id, "on-demand");
    assert_eq!(result.details[3].id, "team-on-demand");
    assert_eq!(result.details[4].id, "team-pool");
    assert_eq!(result.details[5].id, "limit-type");
    assert_eq!(result.details[7].value, "Named pool");
}

#[test]
fn component_only_buckets_keep_separate_windows() {
    let result = project(
        &json!({"individualUsage":{
                "plan":{"autoPercentUsed":10,"apiPercentUsed":20},
                "onDemand":{"enabled":true,"limit":0,"apiPercentUsed":50},
                "overall":{"enabled":false,"autoPercentUsed":40}
            },"teamUsage":{"pooled":{"apiPercentUsed":7}}}),
        None,
        None,
        None,
        None,
        "fixture".into(),
    );
    assert_eq!(
        result
            .windows
            .iter()
            .map(|window| window.id.as_str())
            .collect::<Vec<_>>(),
        ["plan:auto", "plan:api", "team_pool:api"]
    );
    assert_eq!(result.windows[0].label, "Plan usage (Auto)");
    assert_eq!(result.windows[1].used, 20.0);
    assert_eq!(result.windows[0].duration_seconds, None);
    assert_eq!(result.spend.as_ref().unwrap().percent, None);
    assert_eq!(result.details[0].id, "on-demand");
    assert_eq!(result.details[1].id, "overall");
}

#[test]
fn period_fallback_preserves_plan_and_message() {
    let period = json!({
        "billingCycleEnd":"2026-09-10T00:00:00Z",
        "planUsage":{"used":60,"limit":100},
        "displayMessage":"You have used 60% of your plan."
    });
    let summary = period_summary(&period);
    let result = project(
        &summary,
        Some("pro"),
        Some("cached@example.com"),
        None,
        period["displayMessage"].as_str(),
        "fixture".into(),
    );
    assert_eq!(result.plan, "Pro");
    assert_eq!(result.identity, "cached@example.com");
    assert_eq!(result.windows[0].used, 60.0);
    assert!(result.spend.is_none());
    assert_eq!(
        result.notices,
        [Notice::new(NoticeKind::Info, "You have used 60% of your plan.")]
    );
    assert_eq!(
        profile_email(&json!({"email":"me@example.com"})),
        Some("me@example.com")
    );
}

#[test]
fn unlimited_and_reached_spend_are_projected_without_a_window() {
    let result = project(
        &json!({"isUnlimited":true,"billingCycleStart":"bad",
                "individualUsage":{"onDemand":{"enabled":false,"used":100,"limit":100,"remaining":0}}}),
        None,
        None,
        None,
        Some(""),
        "fixture".into(),
    );
    assert_eq!(result.plan, "Cursor");
    assert!(result.windows.is_empty());
    let spend = result.spend.as_ref().unwrap();
    assert_eq!(spend.enabled, Some(false));
    assert_eq!(spend.percent, Some(100.0));
    assert_eq!(spend.reset, None);
    assert!(spend.limit_reached);
    assert_eq!(
        result.notices,
        [Notice::new(NoticeKind::Info, "This plan has unlimited usage.")]
    );
    assert_eq!(
        result.details[0].value,
        "Off · Used: $1.00 · Limit: $1.00 · Remaining: $0.00"
    );
}

#[test]
fn invalid_optional_data_keeps_valid_allowances() {
    let result = project(
        &json!({"billingCycleStart":"2026-10-10T00:00:00Z","billingCycleEnd":"bad",
                "individualUsage":{"plan":{"used":25,"limit":0,"autoPercentUsed":"NaN"},
                    "onDemand":{"used":4.5,"limit":0,"remaining":-1}},
                "limitType":"", "autoModelSelectedDisplayMessage":"Auto only"}),
        Some(""),
        None,
        None,
        None,
        "fixture".into(),
    );
    assert!(result.windows.is_empty());
    let spend = result.spend.as_ref().unwrap();
    assert_eq!(minor(spend.used_money.as_ref()), Some(5));
    assert_eq!(spend.percent, Some(0.0));
    assert!(!spend.limit_reached);
    assert_eq!(result.details.last().unwrap().value, "Auto only");
    assert!(result.details.iter().all(|detail| detail.id != "limit-type"));
    assert_eq!(result.identity, "");
    assert_eq!(profile_email(&json!({"email":""})), None);
}

#[test]
fn negative_billing_cycle_drops_window_duration() {
    let result = project(
        &json!({
            "billingCycleStart":"2026-09-10T00:00:00Z",
            "billingCycleEnd":"2026-09-01T00:00:00Z",
            "individualUsage":{"plan":{"totalPercentUsed":25}}
        }),
        None,
        None,
        None,
        None,
        "fixture".into(),
    );
    assert_eq!(result.windows[0].duration_seconds, None);
}
