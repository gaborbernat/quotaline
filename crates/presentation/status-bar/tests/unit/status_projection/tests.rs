//! Selected status windows retain quota and label behavior.

use super::*;

fn window(id: &str, used: f64, reset_at: Option<f64>) -> StatusWindow {
    StatusWindow {
        id: id.into(),
        label: id.into(),
        group: id.into(),
        used,
        active: true,
        reset_at,
        scope: None,
    }
}

fn selected(provider: &str, window_id: &str) -> StatusSelectedWindow {
    StatusSelectedWindow {
        provider: provider.into(),
        window_id: window_id.into(),
    }
}

#[test]
fn selected_window_uses_resolved_short_label() {
    let providers = [StatusProvider {
        id: "claude".into(),
        windows: vec![window("session", 25.0, Some(200.0))],
    }];
    assert_eq!(
        status_entries(
            &providers,
            &[selected("claude", "session")],
            &BTreeMap::from([("claude:session".into(), "EDIT".into())]),
            100.0,
        ),
        [StatusProjectedEntry {
            provider: "claude".into(),
            provider_code: "CC",
            provider_name: "Claude",
            window_id: "session".into(),
            window_label: "session".into(),
            window_scope: None,
            short_label: "EDIT".into(),
            used: 25.0,
            reset_at: Some(200.0),
            active: true,
            restricted: false,
            restrictions: vec![],
        }]
    );
}

#[test]
fn exhausted_weekly_limit_projects_selected_session() {
    let providers = [StatusProvider {
        id: "claude".into(),
        windows: vec![
            window("session", 25.0, Some(150.0)),
            window("weekly", 100.0, Some(200.0)),
        ],
    }];
    let entries = status_entries(&providers, &[selected("claude", "session")], &BTreeMap::new(), 100.0);
    assert_eq!(entries[0].used, 100.0);
    assert!(entries[0].restricted);
    assert_eq!(entries[0].reset_at, Some(200.0));
    assert_eq!(
        entries[0].restrictions,
        [StatusRestriction {
            label: "weekly".into(),
            reset_at: Some(200.0)
        }]
    );
}

#[test]
fn restricted_window_without_reset_keeps_unknown_deadline() {
    let providers = [StatusProvider {
        id: "claude".into(),
        windows: vec![window("session", 0.0, Some(150.0)), window("weekly", 100.0, None)],
    }];
    let entries = status_entries(&providers, &[selected("claude", "session")], &BTreeMap::new(), 100.0);
    assert_eq!(entries[0].reset_at, None);
    assert_eq!(entries[0].used, 100.0);
    assert!(entries[0].restricted);
}

#[test]
fn missing_provider_window_or_label_does_not_create_an_entry() {
    let providers = [StatusProvider {
        id: "claude".into(),
        windows: vec![window("session", 25.0, None)],
    }];
    let entries = status_entries(
        &providers,
        &[
            selected("codex", "session"),
            selected("claude", "missing"),
            selected("claude", "session"),
        ],
        &BTreeMap::new(),
        100.0,
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].window_id, "session");
}

#[test]
fn unknown_provider_has_no_status_entry() {
    let providers = [StatusProvider {
        id: "unknown".into(),
        windows: vec![window("session", 25.0, None)],
    }];
    assert!(status_entries(&providers, &[selected("unknown", "session")], &BTreeMap::new(), 100.0).is_empty());
}

#[test]
fn inactive_limit_does_not_restrict_selected_window() {
    let mut weekly = window("weekly", 100.0, Some(200.0));
    weekly.active = false;
    let providers = [StatusProvider {
        id: "claude".into(),
        windows: vec![window("session", 25.0, Some(150.0)), weekly],
    }];
    let entries = status_entries(&providers, &[selected("claude", "session")], &BTreeMap::new(), 100.0);
    assert_eq!(entries[0].used, 25.0);
    assert!(entries[0].restrictions.is_empty());
}

#[test]
fn inactive_selected_window_without_blocking_quota_is_not_restricted() {
    let mut session = window("session", 100.0, Some(200.0));
    session.active = false;
    let providers = [StatusProvider {
        id: "claude".into(),
        windows: vec![session],
    }];
    let entries = status_entries(&providers, &[selected("claude", "session")], &BTreeMap::new(), 100.0);
    assert!(!entries[0].restricted);
}

#[test]
fn active_quota_can_restrict_inactive_selected_window() {
    let mut session = window("session", 20.0, Some(150.0));
    session.active = false;
    let providers = [StatusProvider {
        id: "claude".into(),
        windows: vec![session, window("weekly", 100.0, Some(200.0))],
    }];
    let entries = status_entries(&providers, &[selected("claude", "session")], &BTreeMap::new(), 100.0);
    assert!(entries[0].restricted);
}
