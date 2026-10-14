use super::*;

#[test]
fn local_metrics_cross_swift_boundary_in_shared_order() {
    let metrics = usage_local_metrics(3.0, 11.0, 5.0);
    assert_eq!(
        metrics.iter().map(|metric| metric.kind).collect::<Vec<_>>(),
        [
            LocalMetricKind::WindowTokens,
            LocalMetricKind::WindowCost,
            LocalMetricKind::CostPerHour,
            LocalMetricKind::TodayTokens,
            LocalMetricKind::TodayMessages,
            LocalMetricKind::TodayCost,
        ]
    );
    assert_eq!(
        metrics
            .iter()
            .map(|metric| metric.money_fraction_digits)
            .collect::<Vec<_>>(),
        [None, Some(2), Some(0), None, None, Some(2)]
    );
}

#[test]
fn local_precision_and_kinds_cross_swift_boundary() {
    assert_eq!(usage_local_money_fraction_digits(9.0), 2);
    assert_eq!(usage_local_money_fraction_digits(10.0), 0);
    for (core, bridge) in [
        (CoreKind::WindowTokens, LocalMetricKind::WindowTokens),
        (CoreKind::WindowCost, LocalMetricKind::WindowCost),
        (CoreKind::CostPerHour, LocalMetricKind::CostPerHour),
        (CoreKind::TodayTokens, LocalMetricKind::TodayTokens),
        (CoreKind::TodayMessages, LocalMetricKind::TodayMessages),
        (CoreKind::TodayCost, LocalMetricKind::TodayCost),
    ] {
        assert_eq!(LocalMetricKind::from(core), bridge);
    }
}
