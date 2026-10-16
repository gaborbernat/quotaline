use super::*;

#[test]
fn save_check_writes_only_if_the_current_credential_is_unchanged() {
    assert_eq!(check_save(Some(&"original"), &"original"), SaveCheck::Write);
}

#[test]
fn save_check_returns_the_changed_credential() {
    assert_eq!(check_save(Some(&"new"), &"original"), SaveCheck::Changed);
}

#[test]
fn save_check_detects_a_removed_credential() {
    assert_eq!(check_save::<&str>(None, &"original"), SaveCheck::Removed);
}

fn proposed() -> Pending<&'static str, &'static str> {
    Pending {
        credential: "new",
        replacing: "old",
        source: "file",
        detail: String::new(),
    }
}

fn resolved(
    credential: Option<&'static str>,
    source: Option<&'static str>,
    issue: Option<&str>,
) -> Resolved<&'static str, &'static str> {
    Resolved {
        credential,
        source,
        issue: issue.map(str::to_owned),
    }
}

#[test]
fn unreadable_store_without_pending_save_propagates_the_read_error() {
    assert_eq!(
        Recovery::<&str, &str>::default().probe_read_error(),
        ReadProbe::ReadError
    );
}

#[test]
fn unreadable_store_keeps_the_pending_credential_active() {
    let mut recovery = Recovery::default();
    recovery.finish(proposed(), SaveOutcome::Failed);
    assert_eq!(
        recovery.probe_read_error(),
        ReadProbe::Resolved(resolved(Some("new"), Some("file"), Some(PERSISTENCE_DETAIL)))
    );
    assert!(recovery.pending().is_some());
}

#[test]
fn loaded_credential_without_pending_save_is_used() {
    assert_eq!(
        Recovery::<&str, &str>::default().probe_loaded(Some(("stored", "keychain"))),
        LoadedProbe::Resolved(resolved(Some("stored"), Some("keychain"), None))
    );
}

#[test]
fn missing_credential_without_pending_save_is_reported() {
    assert_eq!(
        Recovery::<&str, &str>::default().probe_loaded(None),
        LoadedProbe::Resolved(resolved(None, None, None))
    );
}

#[test]
fn previous_credential_triggers_pending_save_retry() {
    let mut recovery = Recovery::default();
    recovery.finish(proposed(), SaveOutcome::Failed);
    assert_eq!(
        recovery.probe_loaded(Some(("old", "file"))),
        LoadedProbe::Retry(Pending {
            detail: PERSISTENCE_DETAIL.into(),
            ..proposed()
        })
    );
    assert!(recovery.pending().is_some());
}

#[test]
fn confirmed_write_clears_pending_state() {
    let mut recovery = Recovery::default();
    recovery.finish(proposed(), SaveOutcome::Failed);
    assert_eq!(
        recovery.probe_loaded(Some(("new", "file"))),
        LoadedProbe::Resolved(resolved(Some("new"), Some("file"), None))
    );
    assert!(recovery.pending().is_none());
}

#[test]
fn identical_refresh_does_not_retry_the_same_credential() {
    let mut recovery = Recovery::default();
    recovery.finish(
        Pending {
            credential: "same",
            replacing: "same",
            source: "file",
            detail: String::new(),
        },
        SaveOutcome::Failed,
    );
    assert_eq!(
        recovery.probe_loaded(Some(("same", "file"))),
        LoadedProbe::Resolved(resolved(Some("same"), Some("file"), None))
    );
    assert!(recovery.pending().is_none());
}

#[test]
fn changed_credential_clears_pending_state() {
    let mut recovery = Recovery::default();
    recovery.finish(proposed(), SaveOutcome::Failed);
    assert_eq!(
        recovery.probe_loaded(Some(("other", "keychain"))),
        LoadedProbe::Resolved(resolved(Some("other"), Some("keychain"), None))
    );
    assert!(recovery.pending().is_none());
}

#[test]
fn removed_credential_clears_pending_state() {
    let mut recovery = Recovery::default();
    recovery.finish(proposed(), SaveOutcome::Failed);
    assert_eq!(
        recovery.probe_loaded(None),
        LoadedProbe::Resolved(resolved(None, None, None))
    );
    assert!(recovery.pending().is_none());
}

#[test]
fn completed_save_returns_refreshed_credential() {
    let resolved = Recovery::default().finish(proposed(), SaveOutcome::Saved);
    assert_eq!(resolved.credential, Some("new"));
    assert_eq!(resolved.source, Some("file"));
    assert_eq!(resolved.issue, None);
}

#[test]
fn changed_source_returns_the_current_credential() {
    let resolved = Recovery::default().finish(proposed(), SaveOutcome::Changed("other", "keychain"));
    assert_eq!(resolved.credential, Some("other"));
    assert_eq!(resolved.source, Some("keychain"));
}

#[test]
fn removed_source_returns_no_credential() {
    let resolved = Recovery::default().finish(proposed(), SaveOutcome::Removed);
    assert_eq!(resolved.credential, None);
    assert_eq!(resolved.source, None);
}
