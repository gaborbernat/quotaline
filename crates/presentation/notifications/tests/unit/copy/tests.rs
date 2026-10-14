//! Words every planned notification and the settings summary.

use super::*;
use crate::ThresholdCrossing;
use quotaline_text::test_clock::Clock;

const NOW: i64 = 1_791_975_600;

fn worded(facts: Facts) -> (String, String) {
    let moments = Moments {
        now: NOW,
        fetched_at: NOW,
    };
    let Copy { title, body } = copy(&facts, "Claude", "Run claude auth login.", moments, &Clock);
    (title, body)
}

fn pair(title: &str, body: &str) -> (String, String) {
    (title.into(), body.into())
}

#[test]
fn authentication_prefers_the_provider_detail_over_the_login_hint() {
    assert_eq!(
        worded(Facts::Authentication {
            detail: Some("Token expired.".into())
        }),
        pair("Claude sign-in needed", "Token expired.")
    );
    assert_eq!(
        worded(Facts::Authentication { detail: None }),
        pair("Claude sign-in needed", "Run claude auth login.")
    );
}

#[test]
fn resets_groups_and_credits_name_the_change() {
    let cases = [
        (
            Facts::Reset {
                label: "Weekly".into(),
                used: 5.0,
            },
            pair("Claude Weekly reset", "Usage is back to 5%."),
        ),
        (
            Facts::ThresholdGroup {
                crossings: vec![
                    ThresholdCrossing {
                        label: "Session".into(),
                        used: 92.0,
                    },
                    ThresholdCrossing {
                        label: "Weekly".into(),
                        used: 78.5,
                    },
                ],
                lowest: 75,
            },
            pair("Claude: 2 limits crossed 75%", "Session at 92%, Weekly at 78%."),
        ),
        (
            Facts::Credits,
            pair(
                "Claude credits depleted",
                "Usage credits ran out; plan limits now apply.",
            ),
        ),
    ];
    for (facts, expected) in cases {
        assert_eq!(worded(facts), expected);
    }
}

fn threshold(used: f64, threshold: u8, reset_at: Option<i64>, precision: ResetPrecision) -> Facts {
    Facts::Threshold {
        label: "Current session".into(),
        used,
        threshold,
        reset_at,
        precision,
    }
}

#[test]
fn thresholds_explain_when_the_limit_resets() {
    let title = "Claude Current session at 75%";
    let limit = "Claude Current session at 100%";
    let cases = [
        (
            threshold(75.0, 75, None, ResetPrecision::Instant),
            pair(title, "Crossed 75% of the current session limit."),
        ),
        (
            threshold(75.0, 75, Some(NOW + 3600), ResetPrecision::Instant),
            pair(
                title,
                &format!("Crossed 75% of the current session limit. Resets time {}.", NOW + 3600),
            ),
        ),
        (
            threshold(100.0, 100, Some(NOW + 9000), ResetPrecision::Instant),
            pair(
                limit,
                &format!("Limit reached. Resets in 2 hr 30 min · time {}.", NOW + 9000),
            ),
        ),
        (
            threshold(100.0, 100, Some(NOW - 60), ResetPrecision::Instant),
            pair(limit, "Limit reached. Reset due · Awaiting provider refresh."),
        ),
        (
            threshold(100.0, 100, Some(NOW + 86_400), ResetPrecision::Day),
            pair(limit, &format!("Limit reached. Resets day {}.", NOW + 86_400)),
        ),
    ];
    for (facts, expected) in cases {
        assert_eq!(worded(facts), expected);
    }
}

#[test]
fn percentages_round_halves_to_even() {
    assert_eq!(
        worded(threshold(76.5, 75, None, ResetPrecision::Instant)).0,
        "Claude Current session at 76%"
    );
}

#[test]
fn reset_clock_reads_the_fetch_time_and_countdowns_read_now() {
    let moments = Moments {
        now: NOW + 3600,
        fetched_at: NOW,
    };
    let facts = threshold(100.0, 100, Some(NOW + 2 * 3600), ResetPrecision::Instant);
    let worded = copy(&facts, "Claude", "", moments, &Clock);
    assert_eq!(
        worded.body,
        format!("Limit reached. Resets in 1 hr 0 min · time {}.", NOW + 7200)
    );
}

#[test]
fn pace_and_credit_notices_count_down_to_their_deadline() {
    let cases = [
        (
            Facts::ExpiringCredit {
                expires_at: NOW + 86_400,
            },
            pair("Claude reset credit expires in 1 d", "Use it before it lapses."),
        ),
        (
            Facts::WillRunOut {
                label: "Session".into(),
                exhaustion_at: NOW + 3600,
                reset_at: NOW + 7200,
            },
            pair(
                "Claude Session will run out in 1 h",
                "At this pace it hits 100% before the reset in 2 h.",
            ),
        ),
        (
            Facts::AheadOfPace {
                label: "Session".into(),
                used: 70.0,
                reset_at: NOW + 9000,
            },
            pair("Claude Session is ahead of pace", "70% used with 2 hr 30 min left."),
        ),
    ];
    for (facts, expected) in cases {
        assert_eq!(worded(facts), expected);
    }
}

fn selection(enabled: bool, thresholds: &[u8], toggles: bool) -> String {
    summary(&Selection {
        enabled,
        thresholds,
        resets: toggles,
        sign_in: toggles,
        pace: toggles,
        expiring_credits: toggles,
    })
}

#[test]
fn summary_lists_the_selected_alerts() {
    assert_eq!(selection(false, &[75], true), "Off");
    assert_eq!(selection(true, &[], false), "On · No alerts selected");
    assert_eq!(
        selection(true, &[75, 90], true),
        "On · 75%, 90%, resets, sign-in, pace, credit expiry"
    );
}
