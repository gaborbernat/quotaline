use super::*;

#[test]
fn credit_metrics_cross_the_swift_boundary_in_shared_order() {
    let metrics = usage_credit_metrics(
        Some(CreditUsageInput {
            unlimited: false,
            overage_limit_reached: true,
            local_messages: Some(CreditRangeInput { lower: 1, upper: 2 }),
            cloud_messages: Some(CreditRangeInput { lower: 3, upper: 4 }),
        }),
        Some(CreditResetInput {
            available: 2,
            applicable: 1,
            total_earned: Some(3),
            purchase_eligible: true,
        }),
        vec![200, 100],
    );
    assert_eq!(
        metrics.iter().map(|metric| metric.kind).collect::<Vec<_>>(),
        [
            CreditMetricKind::Credits,
            CreditMetricKind::Overage,
            CreditMetricKind::LocalMessages,
            CreditMetricKind::CloudMessages,
            CreditMetricKind::LimitResets,
            CreditMetricKind::NextExpiry,
            CreditMetricKind::ResetPurchase,
            CreditMetricKind::ResetsEarned,
        ]
    );
    assert_eq!(metrics[5].epoch, Some(100));
    assert!(metrics[7].supporting);
}

#[test]
fn absent_optional_credit_fields_cross_the_boundary() {
    let metrics = usage_credit_metrics(
        Some(CreditUsageInput {
            unlimited: false,
            overage_limit_reached: false,
            local_messages: None,
            cloud_messages: None,
        }),
        Some(CreditResetInput {
            available: 0,
            applicable: 0,
            total_earned: None,
            purchase_eligible: false,
        }),
        vec![],
    );
    assert_eq!(metrics.len(), 2);
    assert_eq!(metrics[0].kind, CreditMetricKind::Credits);
    assert_eq!(metrics[1].kind, CreditMetricKind::LimitResets);
}

#[test]
fn absent_credit_sources_cross_the_boundary() {
    assert!(usage_credit_metrics(None, None, vec![]).is_empty());
}
