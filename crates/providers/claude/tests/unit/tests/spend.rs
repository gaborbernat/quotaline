//! Checks cloud credits, spend controls, and related notices.

use super::*;

#[test]
fn cloud_credits_keep_reported_remaining_and_expiry() {
    let provider = projection(
        json!({"iguana_necktie":{"limit_dollars":250,"remaining_dollars":83,"used_dollars":167,"resets_at":"2026-11-05T07:59:00Z","locked_reason":"provider_locked"}}),
        Value::Null,
        None,
    )
    .unwrap();
    let credits = provider.cloud_credits.unwrap();
    assert_eq!((credits.limit, credits.remaining), (250.0, 83.0));
    assert_eq!(credits.expires_at.as_deref(), Some("2026-11-05T07:59:00Z"));
    assert_eq!(credits.locked_reason.as_deref(), Some("provider_locked"));
}

#[test]
fn cloud_credits_derive_and_clamp_remaining() {
    for (credit, expected) in [
        (json!({"limit_dollars":250,"used_dollars":167}), 83.0),
        (json!({"limit_dollars":250,"utilization":66.8}), 83.0),
        (json!({"limit_dollars":250}), 250.0),
        (json!({"limit_dollars":250,"remaining_dollars":-1}), 0.0),
        (json!({"limit_dollars":250,"remaining_dollars":300}), 250.0),
    ] {
        let provider = projection(json!({"iguana_necktie":credit}), Value::Null, None).unwrap();
        assert!((provider.cloud_credits.unwrap().remaining - expected).abs() < 0.000_001);
    }
}

#[test]
fn invalid_cloud_limits_do_not_create_credits() {
    for limit in [json!(null), json!(0), json!(-1), json!("bad")] {
        let provider = projection(
            json!({"five_hour":{"utilization":1},"iguana_necktie":{"limit_dollars":limit}}),
            Value::Null,
            None,
        )
        .unwrap();
        assert!(provider.cloud_credits.is_none());
    }
}

#[test]
fn spend_block_reports_minor_units_and_controls() {
    let provider = projection(
        json!({"spend":{"used":{"amount_minor":250,"currency":"EUR"},"limit":{"amount_minor":1000},"balance":{"amount_minor":750},"enabled":false,"can_toggle":true,"can_purchase_credits":true,"disabled_reason":"account_locked","auto_reload":{"enabled":true},"resets_at":"2026-10-10T12:00:00Z"}}),
        Value::Null,
        None,
    )
    .unwrap();
    let spend = provider.spend.unwrap();
    assert_eq!(
        (spend.used, spend.limit, spend.percent),
        (Some(2.5), Some(10.0), Some(25.0))
    );
    assert_eq!(spend.currency.as_deref(), Some("EUR"));
    assert_eq!(spend.disabled_reason.as_deref(), Some("Account Locked"));
    assert_eq!(spend.disabled_reason_code.as_deref(), Some("account_locked"));
    assert_eq!(spend.auto_reload, Some(true));
    assert!(spend.can_toggle);
    assert!(spend.can_purchase_credits);
    assert_eq!(spend.reset.as_deref(), Some("2026-10-10T12:00:00Z"));
    assert_eq!(provider.credit_balance, Some(7.5));
}

#[test]
fn extra_usage_fills_absent_spend_values() {
    let provider = projection(
        json!({"spend":{"enabled":true},"extra_usage":{"used_credits":"500","monthly_limit":"1000","decimal_places":3,"currency":"GBP","is_enabled":true,"utilization":50,"resets_at":"2026-10-11T12:00:00Z"}}),
        Value::Null,
        None,
    )
    .unwrap();
    let spend = provider.spend.unwrap();
    assert_eq!(
        (spend.used, spend.limit, spend.percent),
        (Some(0.5), Some(1.0), Some(50.0))
    );
    assert_eq!(spend.currency.as_deref(), Some("GBP"));
    assert_eq!(spend.reset.as_deref(), Some("2026-10-11T12:00:00Z"));
    assert_eq!(spend.disabled_reason, None);
}

#[test]
fn extra_usage_supports_zero_limit_and_invalid_exponent() {
    for (extra, expected) in [
        (json!({"used_credits":"50","monthly_limit":"0"}), 0.0),
        (
            json!({"used_credits":"50","monthly_limit":"100","decimal_places":3_000_000_000_i64}),
            50.0,
        ),
    ] {
        let provider = projection(json!({"extra_usage":extra}), Value::Null, None).unwrap();
        assert_eq!(provider.spend.unwrap().percent, Some(expected));
    }
}

#[test]
fn out_of_range_money_exponent_uses_the_default() {
    let provider = projection(
        json!({"spend":{"used":{"amount_minor":50,"exponent":3_000_000_000_i64}}}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(provider.spend.unwrap().used, Some(0.5));
}

#[test]
fn out_of_range_exponent_has_no_amount() {
    let provider = projection(
        json!({"spend":{"used":{"amount_minor":50,"exponent":10}}}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(provider.spend.unwrap().used, None);
}

#[test]
fn nonintegral_and_out_of_range_minor_amounts_are_ignored() {
    for amount in [json!(1.5), json!("9223372036854775808"), json!("-9223372036854777856")] {
        let provider = projection(json!({"spend":{"used":amount}}), Value::Null, None).unwrap();
        assert!(provider.spend.unwrap().used_money.is_none());
    }
}

#[test]
fn spend_limit_reached_emits_a_notice() {
    let provider = projection(
        json!({"extra_usage":{"spend_limit_reached":true,"is_enabled":false,"disabled_reason":"limit_reached"}}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(provider.notices.len(), 1);
    assert_eq!(provider.notices[0].kind, NoticeKind::SpendControl);
    assert!(provider.spend.unwrap().limit_reached);
}

#[test]
fn malformed_reset_is_not_published() {
    let provider = projection(json!({"spend":{"resets_at":"tomorrow"}}), Value::Null, None).unwrap();
    assert_eq!(provider.spend.unwrap().reset, None);
}
