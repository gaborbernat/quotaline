//! Checks UTC day keys, midnight timestamps, and retention windows.

use super::*;

#[test]
fn day_floors_to_the_utc_date() {
    assert_eq!(day(1_760_400_000.0), "2025-10-14");
    assert_eq!(day(-0.5), "1969-12-31");
}

#[test]
fn day_out_of_range_falls_back_to_the_epoch() {
    assert_eq!(day(f64::MAX), "1970-01-01");
}

#[test]
fn midnight_parses_a_day() {
    assert_eq!(midnight("2025-10-14"), Some(1_760_400_000.0));
}

#[test]
fn midnight_rejects_an_invalid_day() {
    assert_eq!(midnight("2025-13-01"), None);
}

#[test]
fn retained_days_span_the_retention_window() {
    assert_eq!(
        retained_utc_days(1_760_400_000.0, 7),
        ("2025-10-08".into(), "2025-10-14".into())
    );
}

#[test]
fn retained_days_with_zero_retention_keep_today() {
    assert_eq!(
        retained_utc_days(1_760_400_000.0, 0),
        ("2025-10-14".into(), "2025-10-14".into())
    );
}
