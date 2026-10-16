use super::*;

fn event(id: &str, kind: Kind, window: Option<&str>) -> DeliveryEvent {
    DeliveryEvent {
        id: id.into(),
        kind,
        window_id: window.map(str::to_owned),
    }
}

#[test]
fn authentication_is_announced_once_per_credential_state() {
    let mut ledger = DeliveryLedger::default();
    let events = [
        event("auth", Kind::Authentication, None),
        event("reset", Kind::Reset, None),
    ];
    let missing = CredentialKey::Missing("expired session".into());
    assert_eq!(ledger.filter("claude", Some(&missing), None, &events), [true, true]);
    assert_eq!(ledger.filter("claude", Some(&missing), None, &events), [false, true]);
    let expired = CredentialKey::Expired(1000.0);
    assert_eq!(ledger.filter("claude", Some(&expired), None, &events), [true, true]);
    assert_eq!(ledger.filter("codex", Some(&missing), None, &events), [true, true]);
    ledger.forget_sign_in("claude");
    assert_eq!(ledger.filter("claude", Some(&expired), None, &events), [true, true]);
}

#[test]
fn absent_credentials_match_the_unset_state() {
    let mut ledger = DeliveryLedger::default();
    let events = [event("auth", Kind::Authentication, None)];
    assert_eq!(ledger.filter("claude", None, None, &events), [false]);
    ledger.filter("claude", Some(&CredentialKey::Valid(None)), None, &events);
    assert_eq!(ledger.filter("claude", None, None, &events), [true]);
    assert_eq!(ledger.filter("claude", None, None, &events), [false]);
}

#[test]
fn non_authentication_events_do_not_change_sign_in_state() {
    let mut ledger = DeliveryLedger::default();
    let missing = CredentialKey::Missing("missing".into());
    assert_eq!(
        ledger.filter("claude", Some(&missing), None, &[event("reset", Kind::Reset, None)]),
        [true]
    );
    assert_eq!(
        ledger.filter(
            "claude",
            Some(&missing),
            None,
            &[event("auth", Kind::Authentication, None)]
        ),
        [true]
    );
}

#[test]
fn pace_ids_are_deduplicated_by_provider_window_and_kind() {
    let mut ledger = DeliveryLedger::default();
    let events = [
        event("pace", Kind::WillRunOut, Some("five-hour")),
        event("pace", Kind::AheadOfPace, Some("five-hour")),
        event("pace", Kind::WillRunOut, None),
    ];
    assert_eq!(ledger.filter("claude", None, None, &events), [true, true, true]);
    assert_eq!(ledger.filter("claude", None, None, &events), [false, false, true]);
    assert_eq!(ledger.filter("codex", None, None, &events), [true, true, true]);
    assert_eq!(
        ledger.filter(
            "claude",
            None,
            None,
            &[event("new", Kind::WillRunOut, Some("five-hour"))]
        ),
        [true]
    );
}

#[test]
fn snapshot_prunes_missing_windows_and_clear_resets_pace() {
    let mut ledger = DeliveryLedger::default();
    let item = event("pace", Kind::WillRunOut, Some("five-hour"));
    assert_eq!(ledger.filter("claude", None, None, &[item]), [true]);
    let item = event("pace", Kind::WillRunOut, Some("five-hour"));
    assert_eq!(ledger.filter("codex", None, None, &[item]), [true]);
    let item = event("pace", Kind::WillRunOut, Some("five-hour"));
    assert_eq!(
        ledger.filter("claude", None, Some(&BTreeSet::from(["five-hour".into()])), &[item]),
        [false]
    );
    let item = event("pace", Kind::WillRunOut, Some("five-hour"));
    assert_eq!(ledger.filter("claude", None, Some(&BTreeSet::new()), &[item]), [true]);
    let item = event("pace", Kind::WillRunOut, Some("five-hour"));
    assert_eq!(ledger.filter("claude", None, None, &[item]), [false]);
    ledger.clear_pace();
    let item = event("pace", Kind::WillRunOut, Some("five-hour"));
    assert_eq!(ledger.filter("claude", None, None, &[item]), [true]);
}

#[test]
fn forgetting_pace_and_removed_providers_clear_only_their_state() {
    let mut ledger = DeliveryLedger::default();
    let item = event("pace", Kind::AheadOfPace, Some("day"));
    ledger.filter("claude", None, None, &[item]);
    let item = event("pace", Kind::AheadOfPace, Some("day"));
    ledger.filter("codex", None, None, &[item]);
    ledger.forget_pace("claude");
    let item = event("pace", Kind::AheadOfPace, Some("day"));
    assert_eq!(ledger.filter("claude", None, None, &[item]), [true]);
    let item = event("pace", Kind::AheadOfPace, Some("day"));
    assert_eq!(ledger.filter("codex", None, None, &[item]), [false]);
    let credentials = CredentialKey::Valid(Some(42.0));
    let auth = [event("auth", Kind::Authentication, None)];
    ledger.filter("claude", Some(&credentials), None, &auth);
    ledger.filter("codex", Some(&credentials), None, &auth);
    ledger.retain_providers(&BTreeSet::from(["codex".into()]));
    assert_eq!(ledger.filter("claude", Some(&credentials), None, &auth), [true]);
    assert_eq!(ledger.filter("codex", Some(&credentials), None, &auth), [false]);
}
