use super::*;
use crate::test_clock::Clock;

#[test]
fn ffi_converts_iso_dates() {
    assert_eq!(parse_iso_epoch("1970-01-01T00:00:00Z".into()), Some(0.0));
    assert_eq!(parse_iso_epoch("invalid".into()), None);
    assert_eq!(format_iso_epoch(0.0).as_deref(), Some("1970-01-01T00:00:00.000Z"));
    assert_eq!(format_iso_epoch(f64::NAN), None);
}

#[test]
fn ffi_words_durations() {
    assert_eq!(time_countdown(9000.0), "2 hr 30 min");
    assert_eq!(time_compact_countdown(3900.0), "1h05m");
    assert_eq!(time_relative_age(125.0), "2 min ago");
}

#[test]
fn ffi_reset_clock_uses_the_swift_formatter() {
    let clock: Arc<dyn ClockFormatter> = Arc::new(Clock);
    let cases = [
        (100, false, "time 100"),
        (100, true, "day 100"),
        (86_400, false, "weekday 86400"),
        (7 * 86_400, false, "date 604800"),
    ];
    for (at, day_precision, expected) in cases {
        assert_eq!(time_reset_clock(at, day_precision, 0, clock.clone()), expected);
    }
}
