//! Checks the public behavior of the adjacent module.

use super::*;

fn window(used: f64, reset: Option<&str>) -> Window {
    Window::new("session", "Session", used, reset.map(str::to_owned), "session")
}

#[test]
fn early_zero_requires_a_second_fetch() {
    let mut guard = Guard::default();
    let previous = window(42.0, Some("2026-10-13T05:00:00Z"));
    assert_eq!(guard.confirm(vec![previous.clone()], 1_791_864_000.0)[0].used, 42.0);
    let first = guard.confirm(vec![window(0.0, Some("2026-10-13T10:00:00Z"))], 1_791_864_000.0);
    assert_eq!(first[0].used, 42.0);
    assert_eq!(first[0].detail, "Confirming reset");
    let second = guard.confirm(vec![window(0.0, Some("2026-10-13T10:00:00Z"))], 1_791_864_000.0);
    assert_eq!(second[0].used, 0.0);
    assert!(second[0].detail.is_empty());
}

#[test]
fn ordinary_readings_and_elapsed_resets_pass_through() {
    let mut guard = Guard::default();
    for (previous, current, reset, now) in [
        (42.0, 5.0, Some("2026-10-13T05:00:00Z"), 1_791_864_000.0),
        (0.0, 0.0, Some("2026-10-13T05:00:00Z"), 1_791_864_000.0),
        (42.0, 0.0, Some("2026-10-13T05:00:00Z"), 1_791_871_200.0),
        (42.0, 0.0, None, 1_791_864_000.0),
        (42.0, 0.0, Some("bad"), 1_791_864_000.0),
    ] {
        guard.clear();
        guard.confirm(vec![window(previous, reset)], now);
        let result = guard.confirm(vec![window(current, reset)], now);
        assert_eq!(result[0].used, current);
        assert!(result[0].detail.is_empty());
    }
}

#[test]
fn clear_discards_pending_and_previous_windows() {
    let mut guard = Guard::default();
    let now = 1_791_864_000.0;
    guard.confirm(vec![window(42.0, Some("2026-10-13T05:00:00Z"))], now);
    guard.confirm(vec![window(0.0, None)], now);
    guard.clear();
    assert_eq!(guard.confirm(vec![window(0.0, None)], now)[0].used, 0.0);
    assert!(guard.confirm(vec![], now).is_empty());
}
