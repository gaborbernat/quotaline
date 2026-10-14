//! Checks notification transitions and message text.

use super::*;

#[test]
fn clock_rollback_does_not_suppress_credit_expiry() {
    let now = 1_780_000_000;
    let credit = CreditExpiry {
        id: "credit".into(),
        at: now + 3600,
    };
    let mut before = provider(&[], now, now + 7200);
    before.reset_credit_expiries.push(credit.clone());
    let mut after = provider(&[], now, now);
    after.reset_credit_expiries.push(credit);
    let events = plan(Some(&before), &after, &Settings::default(), &[], now);
    assert_eq!(events.len(), 1);
    assert!(events[0].id.contains(":reset-credit:"));
}

#[test]
fn first_sign_in_notice_uses_zero_when_previous_update_is_unknown() {
    let now = 1_780_000_000;
    let mut before = provider(&[], now, now);
    before.updated = None;
    let mut after = provider(&[], now, now);
    after.status = "sign-in".into();
    let notice = auth_problem(Some(&before), &after, &Settings::default(), false).unwrap();
    assert_eq!(notice.id, "codex:auth:0");
}

#[test]
fn sign_in_notice_respects_its_switch() {
    let now = 1_780_000_000;
    let before = provider(&[], now, now - 60);
    let mut after = provider(&[], now, now);
    after.status = "sign-in".into();
    let settings = Settings {
        notify_sign_in: false,
        ..Settings::default()
    };
    assert!(auth_problem(Some(&before), &after, &settings, false).is_none());
}

#[test]
fn sign_in_notice_follows_a_previous_error() {
    let now = 1_780_000_000;
    let mut before = provider(&[], now, now - 60);
    before.status = "error".into();
    let mut after = provider(&[], now, now);
    after.status = "sign-in".into();
    assert!(auth_problem(Some(&before), &after, &Settings::default(), false).is_some());
}

#[test]
fn repeated_sign_in_does_not_notify_again() {
    let now = 1_780_000_000;
    let mut before = provider(&[], now, now - 60);
    before.status = "sign-in".into();
    let mut after = provider(&[], now, now);
    after.status = "sign-in".into();
    assert!(auth_problem(Some(&before), &after, &Settings::default(), false).is_none());
}

#[test]
fn cached_stale_sign_in_notice_does_not_repeat_after_restart() {
    let now = 1_780_000_000;
    let mut before = provider(&[75.0], now, now - 60);
    before.status = "stale".into();
    before.notice = "Token expired".into();
    let mut after = before.clone();
    after.status = "sign-in".into();
    assert!(auth_problem(Some(&before), &after, &Settings::default(), false).is_none());
}

#[test]
fn connected_refresh_does_not_need_sign_in_notice() {
    let now = 1_780_000_000;
    let before = provider(&[], now, now - 60);
    let after = provider(&[], now, now);
    assert!(auth_problem(Some(&before), &after, &Settings::default(), false).is_none());
}

#[test]
fn disabled_notifications_emit_no_events() {
    let now = 1_780_000_000;
    let before = provider(&[70.0], now + 50_000, now - 60);
    let mut after = provider(&[100.0], now + 50_000, now);
    let settings = Settings {
        notifications: false,
        ..Settings::default()
    };
    assert!(plan(Some(&before), &after, &settings, &[], now).is_empty());
    after.status = "sign-in".into();
    assert!(auth_problem(Some(&before), &after, &settings, false).is_none());
}

#[test]
fn reset_detection_uses_usage_when_timestamps_are_absent() {
    let now = 1_780_000_000;
    let mut before = provider(&[80.0], now + 10, now - 60);
    let mut after = provider(&[2.0], now + 50_000, now);
    before.windows[0].reset = None;
    after.windows[0].reset = None;
    let events = plan(Some(&before), &after, &Settings::default(), &[], now);
    assert!(events.iter().any(|event| event.id.contains(":reset:")));
}

#[test]
fn threshold_facts_preserve_usage_deadline_and_precision() {
    let now = 1_780_000_000;
    for (used, threshold, precision, reset_at) in [
        (75.0, 75, ResetPrecision::Instant, Some(now + 3600)),
        (100.0, 100, ResetPrecision::Instant, Some(now - 60)),
        (75.0, 75, ResetPrecision::Day, Some(now + 86_400)),
        (100.0, 100, ResetPrecision::Day, Some(now + 86_400)),
        (75.0, 75, ResetPrecision::Instant, None),
    ] {
        let mut before = provider(&[74.0_f64.min(used - 1.0)], reset_at.unwrap_or(now), now - 60);
        let mut after = provider(&[used], reset_at.unwrap_or(now), now);
        before.windows[0].reset_precision = precision;
        after.windows[0].reset_precision = precision;
        if reset_at.is_none() {
            before.windows[0].reset = None;
            after.windows[0].reset = None;
        }
        let settings = Settings {
            notify_pace: false,
            ..Settings::default()
        };
        let events = plan(Some(&before), &after, &settings, &[], now);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].facts,
            Facts::Threshold {
                label: "Window 0".into(),
                used,
                threshold,
                reset_at,
                precision,
            }
        );
    }
}

#[test]
fn expiring_credit_facts_preserve_each_deadline() {
    let now = 1_780_000_000;
    let mut after = provider(&[], now, now);
    for (id, offset) in [("day", 86_400), ("hour", 3600), ("minute", 30)] {
        after.reset_credit_expiries.push(CreditExpiry {
            id: id.into(),
            at: now + offset,
        });
    }
    let events = plan(None, &after, &Settings::default(), &[], now);
    assert_eq!(
        events.iter().map(|event| &event.facts).collect::<Vec<_>>(),
        vec![
            &Facts::ExpiringCredit {
                expires_at: now + 86_400
            },
            &Facts::ExpiringCredit { expires_at: now + 3600 },
            &Facts::ExpiringCredit { expires_at: now + 30 },
        ]
    );
}

#[test]
fn expired_and_distant_credits_are_not_announced() {
    let now = 1_780_000_000;
    let mut after = provider(&[], now, now);
    for offset in [-1, 0, 86_401] {
        after.reset_credit_expiries.push(CreditExpiry {
            id: offset.to_string(),
            at: now + offset,
        });
    }
    assert!(plan(None, &after, &Settings::default(), &[], now).is_empty());
}

#[test]
fn new_window_does_not_announce_past_thresholds() {
    let now = 1_780_000_000;
    let before = provider(&[], now + 50_000, now - 60);
    let after = provider(&[90.0], now + 50_000, now);
    let settings = Settings {
        notify_pace: false,
        ..Settings::default()
    };
    assert!(plan(Some(&before), &after, &settings, &[], now).is_empty());
}

#[test]
fn credit_expiry_toggle_suppresses_credit_events() {
    let now = 1_780_000_000;
    let mut after = provider(&[], now, now);
    after.reset_credit_expiries.push(CreditExpiry {
        id: "one".into(),
        at: now + 60,
    });
    let settings = Settings {
        notify_expiring_credits: false,
        ..Settings::default()
    };
    assert!(plan(None, &after, &settings, &[], now).is_empty());
}
