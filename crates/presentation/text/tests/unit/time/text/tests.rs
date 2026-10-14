//! Words durations and reset times through the shared formatters.

use super::*;
use crate::test_clock::Clock;

// The Shell renders live countdowns in JavaScript; its tests read the same cases so both stay identical.
fn cases(name: &str) -> Vec<(f64, String)> {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/time-text.json")).unwrap();
    serde_json::from_value(fixture[name].clone()).unwrap()
}

#[test]
fn durations_match_the_shared_cases() {
    type Format = fn(f64) -> String;
    let formatters: [(&str, Format); 4] = [
        ("countdown", countdown),
        ("compact_countdown", compact_countdown),
        ("remaining", |seconds| remaining(seconds as i64)),
        ("relative_age", relative_age),
    ];
    for (name, format) in formatters {
        for (seconds, expected) in cases(name) {
            assert_eq!(format(seconds), expected, "{name} {seconds}");
        }
    }
}

#[test]
fn reset_clock_chooses_the_closest_calendar_detail() {
    let now = 1_000_000;
    let cases = [
        (now, ResetPrecision::Instant, "now".to_string()),
        (now + 60, ResetPrecision::Day, format!("day {}", now + 60)),
        (now + 60, ResetPrecision::Instant, format!("time {}", now + 60)),
        (
            now + 86_400,
            ResetPrecision::Instant,
            format!("weekday {}", now + 86_400),
        ),
        (
            now + 7 * 86_400,
            ResetPrecision::Instant,
            format!("date {}", now + 7 * 86_400),
        ),
    ];
    for (at, precision, expected) in cases {
        assert_eq!(reset_clock(at, precision, now, &Clock), expected, "{at}");
    }
}
