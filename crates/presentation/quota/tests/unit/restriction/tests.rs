//! Quota restriction cases.

use super::*;

fn window<'a>(id: &'a str, group: &'a str, used: f64, reset_at: Option<f64>) -> RestrictionWindow<'a> {
    RestrictionWindow {
        id,
        label: id,
        group,
        used,
        active: true,
        reset_at,
    }
}

#[test]
fn claude_exhausted_session_and_weekly_limits_apply_to_other_windows() {
    let target = window("weekly:fable", "weekly", 30.0, None);
    assert!(restriction_applies(
        "claude",
        window("session", "session", 100.0, None),
        target,
        0.0
    ));
    assert!(restriction_applies(
        "claude",
        window("weekly", "weekly", 100.0, None),
        target,
        0.0
    ));
    assert!(!restriction_applies(
        "claude",
        window("monthly", "monthly", 100.0, None),
        target,
        0.0
    ));
}

#[test]
fn codex_restriction_matches_model_prefix_but_excludes_other_groups() {
    let target = window("spark:session", "session", 20.0, None);
    assert!(restriction_applies(
        "codex",
        window("spark:weekly", "weekly", 100.0, None),
        target,
        0.0
    ));
    assert!(!restriction_applies(
        "codex",
        window("other:weekly", "weekly", 100.0, None),
        target,
        0.0
    ));
    assert!(!restriction_applies(
        "codex",
        window("spark:weekly", "other", 100.0, None),
        target,
        0.0
    ));
    assert!(!restriction_applies(
        "codex",
        window("spark:weekly", "weekly", 100.0, None),
        window("other", "other", 0.0, None),
        0.0
    ));
}

#[test]
fn inactive_available_and_expired_windows_do_not_restrict() {
    let target = window("weekly:fable", "weekly", 20.0, None);
    let mut limit = window("session", "session", 100.0, Some(10.0));
    assert!(!restriction_applies("claude", limit, target, 10.0));
    limit.reset_at = Some(11.0);
    assert!(restriction_applies("claude", limit, target, 10.0));
    limit.active = false;
    assert!(!restriction_applies("claude", limit, target, 10.0));
    limit.active = true;
    limit.used = 99.0;
    assert!(!restriction_applies("claude", limit, target, 10.0));
}

#[test]
fn a_window_can_restrict_itself_but_is_not_a_cross_window_restriction() {
    let target = window("session", "session", 100.0, None);
    assert!(restriction_applies("gemini", target, target, 0.0));
    assert!(restrictive_indices("gemini", target, &[target], 0.0).is_none());
}

#[test]
fn restrictions_keep_self_and_other_exhausted_limits_in_source_order() {
    let target = window("weekly:fable", "weekly", 100.0, Some(90.0));
    let candidates = [
        target,
        window("session", "session", 100.0, Some(80.0)),
        window("monthly", "monthly", 100.0, None),
    ];
    assert_eq!(
        restrictive_indices("claude", target, &candidates, 0.0),
        Some(vec![0, 1])
    );
}

#[test]
fn projected_reset_uses_latest_known_limit_or_unknown() {
    let candidates = [
        window("first", "weekly", 100.0, Some(30.0)),
        window("second", "weekly", 100.0, Some(90.0)),
    ];
    assert_eq!(projected_restriction_reset(&candidates, &[0, 1]), Some(90.0));
    assert_eq!(projected_restriction_reset(&candidates, &[]), None);
    let unknown = [window("first", "weekly", 100.0, None), candidates[1]];
    assert_eq!(projected_restriction_reset(&unknown, &[0, 1]), None);
}
