use super::*;

#[test]
fn local_metrics_keep_shared_order_and_cost_precision() {
    let metrics = local_metrics(3.456, 42.4, 10.0);
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
        [None, Some(2), Some(0), None, None, Some(0)]
    );
}

#[test]
fn local_money_precision_changes_at_ten() {
    assert_eq!(local_money_fraction_digits(9.999), 2);
    assert_eq!(local_money_fraction_digits(10.0), 0);
}
