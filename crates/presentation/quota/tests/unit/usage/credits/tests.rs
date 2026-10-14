use super::*;

fn usage() -> UsageCredits {
    UsageCredits {
        unlimited: false,
        has_credits: None,
        overage_limit_reached: false,
        local_messages: None,
        cloud_messages: None,
    }
}

fn resets() -> ResetCredits {
    ResetCredits {
        available: 0,
        applicable: 0,
        total_earned: None,
        purchase_eligible: false,
        details: Vec::new(),
    }
}

#[test]
fn absent_credit_sources_have_no_metrics() {
    assert!(credit_metrics(None, None, &[]).is_empty());
}

#[test]
fn unlimited_credit_usage_keeps_estimates_as_supporting_metrics() {
    let credits = UsageCredits {
        unlimited: true,
        overage_limit_reached: true,
        local_messages: Some((2, 4)),
        cloud_messages: Some((6, 8)),
        ..usage()
    };
    let metrics = credit_metrics(Some(&credits), None, &[]);
    assert_eq!(
        metrics.iter().map(|entry| entry.kind).collect::<Vec<_>>(),
        [
            CreditMetricKind::Credits,
            CreditMetricKind::Overage,
            CreditMetricKind::LocalMessages,
            CreditMetricKind::CloudMessages,
        ]
    );
    assert!(!metrics[0].supporting);
    assert!(metrics[2].supporting);
    assert!(metrics[3].supporting);
}

#[test]
fn finite_balance_selects_credit_metric() {
    let metrics = credit_metrics(Some(&usage()), None, &[]);
    assert_eq!(metrics.len(), 1);
    assert_eq!(metrics[0].kind, CreditMetricKind::Credits);
}

#[test]
fn reset_credit_metrics_select_earliest_expiry_and_optional_details() {
    let resets = ResetCredits {
        available: 3,
        applicable: 2,
        total_earned: Some(0),
        purchase_eligible: true,
        ..resets()
    };
    let metrics = credit_metrics(None, Some(&resets), &[200, 100]);
    assert_eq!(
        metrics.iter().map(|entry| entry.kind).collect::<Vec<_>>(),
        [
            CreditMetricKind::LimitResets,
            CreditMetricKind::NextExpiry,
            CreditMetricKind::ResetPurchase,
            CreditMetricKind::ResetsEarned,
        ]
    );
    assert_eq!(metrics[1].epoch, Some(100));
    assert!(metrics[3].supporting);
}

#[test]
fn reset_credits_without_optional_details_keep_available_count() {
    let metrics = credit_metrics(None, Some(&resets()), &[]);
    assert_eq!(metrics.len(), 1);
    assert_eq!(metrics[0].kind, CreditMetricKind::LimitResets);
}
