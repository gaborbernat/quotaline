use super::*;

#[test]
fn spend_projection_crosses_the_swift_boundary() {
    let projection = usage_spend_projection(SpendFactsInput {
        used_credits: false,
        limit_credits: true,
        used_money: true,
        limit_money: false,
        percent: Some(25.0),
        remaining_credits: true,
        allowance_source: true,
        balance: true,
        reset: true,
        auto_reload: true,
        purchase: true,
    });
    assert_eq!(projection.used, Some(SpendAmount::Money));
    assert_eq!(projection.limit, Some(SpendAmount::Credits));
    assert_eq!(projection.percent, Some(25.0));
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
fn every_spend_amount_and_metric_kind_crosses_the_boundary() {
    for (core, bridge) in [
        (CoreAmount::Credits, SpendAmount::Credits),
        (CoreAmount::Money, SpendAmount::Money),
        (CoreAmount::Unavailable, SpendAmount::Unavailable),
        (CoreAmount::UnreportedLimit, SpendAmount::UnreportedLimit),
    ] {
        assert_eq!(SpendAmount::from(core), bridge);
    }
    for (core, bridge) in [
        (CoreKind::Remaining, SpendMetricKind::Remaining),
        (CoreKind::AllowanceSource, SpendMetricKind::AllowanceSource),
        (CoreKind::Balance, SpendMetricKind::Balance),
        (CoreKind::Resets, SpendMetricKind::Resets),
        (CoreKind::AutoReload, SpendMetricKind::AutoReload),
        (CoreKind::Purchase, SpendMetricKind::Purchase),
    ] {
        assert_eq!(SpendMetricKind::from(core), bridge);
    }
}
