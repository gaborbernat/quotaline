//! Checks Claude plan labels and profile identity from provider responses.

use super::*;

#[test]
fn organization_type_names_the_plan() {
    for (value, expected) in [
        ("claude_max", "Max"),
        ("claude_pro", "Pro"),
        ("claude_team", "Team"),
        ("claude_enterprise", "Enterprise"),
        ("business_plus", "Business Plus"),
    ] {
        let provider = projection(
            json!({"five_hour": {"utilization": 1}}),
            Value::Null,
            Some(json!({"organization": {"organization_type": value}})),
        )
        .unwrap();
        assert_eq!(provider.plan, expected, "{value}");
    }
}

#[test]
fn account_flags_name_plans_without_a_subscription() {
    for (account, expected) in [
        (json!({"has_claude_max": true}), "Max"),
        (json!({"has_claude_pro": true}), "Pro"),
        (json!({}), "Claude"),
    ] {
        let provider = projection(
            json!({"five_hour": {"utilization": 1}}),
            Value::Null,
            Some(json!({"account": account})),
        )
        .unwrap();
        assert_eq!(provider.plan, expected);
    }
}

#[test]
fn subscription_and_rate_limit_tiers_supply_a_multiplier() {
    for (auth, profile, expected) in [
        (
            json!({"subscriptionType":"Pro", "rateLimitTier":"tier_20x"}),
            None,
            "Pro 20x",
        ),
        (json!({"subscriptionType":"Pro", "rateLimitTier":"tier_x"}), None, "Pro"),
        (
            json!({"subscriptionType":"Pro"}),
            Some(json!({"organization":{"rate_limit_tier":"tier_5x"}})),
            "Pro 5x",
        ),
    ] {
        let provider = projection(json!({"five_hour": {"utilization": 1}}), auth, profile).unwrap();
        assert_eq!(provider.plan, expected);
    }
}

#[test]
fn profile_overrides_auth_identity_and_adds_subscription_detail() {
    let provider = projection(
        json!({"five_hour": {"utilization": 1}}),
        json!({"emailAddress": "cli@example.com", "subscriptionType": "Pro"}),
        Some(json!({"account":{"email":"profile@example.com"},"organization":{"name":"Team A","organization_type":"team","subscription_status":"active_trial"}})),
    )
    .unwrap();
    assert_eq!(provider.identity, "profile@example.com");
    assert_eq!(provider.organization, "Team A");
    assert_eq!(provider.plan, "Team");
    assert_eq!(provider.details[0].value, "Active Trial");
}

#[test]
fn auth_identity_is_used_without_a_profile() {
    let provider = projection(
        json!({"five_hour": {"utilization": 1}}),
        json!({"emailAddress": "cli@example.com"}),
        None,
    )
    .unwrap();
    assert_eq!(provider.identity, "cli@example.com");
    assert!(provider.organization.is_empty());
}
