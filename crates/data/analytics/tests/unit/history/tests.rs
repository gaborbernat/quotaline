//! Checks aggregate selection through the public history projection.

use super::*;

#[test]
fn usage_sampling_records_first_sample_and_meaningful_changes() {
    let previous = UsageSample { at: 1000.0, used: 25.0 };
    for (name, current, reset_changed, expected) in [
        ("first", UsageSample { at: 1000.0, used: 25.0 }, false, true),
        ("unchanged", UsageSample { at: 1299.0, used: 29.9 }, false, false),
        ("time", UsageSample { at: 1300.0, used: 25.0 }, false, true),
        ("usage", UsageSample { at: 1001.0, used: 30.0 }, false, true),
        ("reset", UsageSample { at: 1001.0, used: 25.0 }, true, true),
    ] {
        assert_eq!(
            should_record_usage(
                current,
                if name == "first" { None } else { Some(previous) },
                reset_changed
            ),
            expected,
            "{name}"
        );
    }
}

fn reading(provider: &str, bucket: &str, series: &str, value: f64) -> HistoryReading {
    HistoryReading {
        provider: provider.into(),
        bucket: bucket.into(),
        series: series.into(),
        value,
    }
}

#[test]
fn totals_take_priority_over_parallel_breakdowns() {
    let rows = [
        reading("codex", "2026-10-12", "total", 10.0),
        reading("codex", "2026-10-12", "surface:cli", 4.0),
        reading("codex", "2026-10-12", "surface:web", 6.0),
        reading("codex", "2026-10-13", "total", 20.0),
        reading("claude", "2026-10-12", "other", 3.0),
    ];
    let projection = history_projection(&rows);
    assert_eq!(projection.canonical_total, 33.0);
    assert!(
        projection
            .detailed_buckets
            .contains(&("codex".into(), "2026-10-12".into()))
    );
    assert!(
        !projection
            .detailed_buckets
            .contains(&("codex".into(), "2026-10-13".into()))
    );
}

#[test]
fn surfaces_take_priority_when_aggregate_is_missing() {
    let rows = [
        reading("codex", "1", "surface:cli", 4.0),
        reading("codex", "1", "surface:web", 6.0),
        reading("codex", "1", "other", 2.0),
        reading("codex", "2", "other", 5.0),
    ];
    assert_eq!(history_projection(&rows).canonical_total, 15.0);
    assert_eq!(history_projection(&[]).canonical_total, 0.0);
}
