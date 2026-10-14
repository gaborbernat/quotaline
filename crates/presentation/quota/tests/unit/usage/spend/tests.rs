use super::*;
use crate::CreditAllowance;

#[test]
fn spend_projection_selects_complete_details_in_shared_order() {
    let projection = spend_projection(SpendFacts {
        used_credits: true,
        limit_credits: true,
        used_money: true,
        limit_money: true,
        percent: Some(31.0),
        remaining_credits: true,
        allowance_source: true,
        balance: true,
        reset: true,
        auto_reload: true,
        purchase: true,
    });
    assert_eq!(projection.used, Some(SpendAmount::Credits));
    assert_eq!(projection.limit, Some(SpendAmount::Credits));
    assert_eq!(projection.percent, Some(31.0));
    assert_eq!(
        projection.metrics.iter().map(|metric| metric.kind).collect::<Vec<_>>(),
        [
            SpendMetricKind::Remaining,
            SpendMetricKind::AllowanceSource,
            SpendMetricKind::Balance,
            SpendMetricKind::Resets,
            SpendMetricKind::AutoReload,
            SpendMetricKind::Purchase,
        ]
    );
}

#[test]
fn spend_projection_preserves_amount_sources_independently() {
    for (facts, used, limit) in [
        (
            SpendFacts {
                used_money: true,
                limit_credits: true,
                ..SpendFacts::default()
            },
            SpendAmount::Money,
            SpendAmount::Credits,
        ),
        (
            SpendFacts {
                used_credits: true,
                limit_money: true,
                ..SpendFacts::default()
            },
            SpendAmount::Credits,
            SpendAmount::Money,
        ),
        (
            SpendFacts {
                limit_money: true,
                ..SpendFacts::default()
            },
            SpendAmount::Unavailable,
            SpendAmount::Money,
        ),
        (
            SpendFacts {
                used_money: true,
                ..SpendFacts::default()
            },
            SpendAmount::Money,
            SpendAmount::UnreportedLimit,
        ),
    ] {
        let projection = spend_projection(facts);
        assert_eq!(projection.used, Some(used));
        assert_eq!(projection.limit, Some(limit));
    }
}

#[test]
fn spend_projection_handles_unreported_and_off_states() {
    let missing = spend_projection(SpendFacts::default());
    assert_eq!(missing.used, None);
    assert_eq!(missing.limit, None);
    assert!(missing.metrics.is_empty());

    let auto_reload = spend_projection(SpendFacts {
        auto_reload: true,
        ..SpendFacts::default()
    });
    assert_eq!(auto_reload.metrics[0].kind, SpendMetricKind::AutoReload);
}

#[test]
fn spend_facts_extract_only_present_values() {
    let spend = SpendControl {
        enabled: Some(true),
        can_toggle: false,
        disabled_reason: None,
        disabled_reason_code: None,
        used: Some(1.0),
        limit: None,
        used_money: None,
        limit_money: None,
        percent: Some(2.0),
        currency: None,
        reset: Some("tomorrow".into()),
        limit_reached: false,
        balance: Some(3.0),
        balance_money: None,
        auto_reload: Some(false),
        can_purchase_credits: true,
        credit_allowance: Some(CreditAllowance {
            used: None,
            limit: Some(4.0),
            remaining: Some(5.0),
            source: Some("organization".into()),
        }),
    };
    let facts = SpendFacts::from_spend(&spend);
    assert!(!facts.used_credits);
    assert!(facts.limit_credits);
    assert!(facts.used_money);
    assert!(!facts.limit_money);
    assert!(facts.remaining_credits);
    assert!(facts.allowance_source);
    assert!(facts.balance);
    assert!(facts.reset);
    assert!(facts.auto_reload);
    assert!(facts.purchase);
    assert_eq!(facts.percent, Some(2.0));

    let absent = SpendControl {
        credit_allowance: None,
        used: None,
        reset: None,
        balance: None,
        can_purchase_credits: false,
        percent: None,
        ..spend
    };
    let facts = SpendFacts::from_spend(&absent);
    assert!(!facts.limit_credits);
    assert!(!facts.remaining_credits);
    assert!(!facts.allowance_source);
    assert!(!facts.used_money);
    assert!(!facts.balance);
    assert!(!facts.reset);
    assert!(!facts.purchase);
}
