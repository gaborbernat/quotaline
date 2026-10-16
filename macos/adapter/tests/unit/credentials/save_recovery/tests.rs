use super::*;

#[test]
fn bridge_compares_credential_documents_by_value() {
    assert_eq!(
        credential_save_check(Some(br#"{"a":1,"b":2}"#.to_vec()), br#"{"b":2,"a":1}"#.to_vec()),
        Ok(CredentialSaveCheck::Write)
    );
    assert_eq!(
        credential_save_check(Some(br#"{"a":2}"#.to_vec()), br#"{"a":1}"#.to_vec()),
        Ok(CredentialSaveCheck::Changed)
    );
    assert_eq!(
        credential_save_check(None, b"{}".to_vec()),
        Ok(CredentialSaveCheck::Removed)
    );
}

#[test]
fn bridge_rejects_an_invalid_expected_or_current_document() {
    assert_eq!(
        credential_save_check(None, b"{".to_vec()),
        Err(CredentialRecoveryError::InvalidJson)
    );
    assert_eq!(
        credential_save_check(Some(b"{".to_vec()), b"{}".to_vec()),
        Err(CredentialRecoveryError::InvalidJson)
    );
}

fn source() -> CredentialRecoverySource {
    CredentialRecoverySource {
        id: "file".into(),
        provider: "codex".into(),
        title: "File".into(),
        detail: "Credential file".into(),
    }
}

fn proposed() -> CredentialRecoveryPending {
    CredentialRecoveryPending {
        document: br#"{"token":"new"}"#.to_vec(),
        replacing: br#"{"token":"old"}"#.to_vec(),
        source: source(),
        detail: String::new(),
    }
}

#[test]
fn bridge_propagates_unrecoverable_read_error() {
    let ledger = CredentialRecoveryLedger::new();
    assert_eq!(ledger.probe_read_error(), CredentialRecoveryReadProbe::ReadError);
}

#[test]
fn bridge_resolves_loaded_credential() {
    let ledger = CredentialRecoveryLedger::new();
    let probe = ledger
        .probe_loaded(Some(CredentialRecoveryStored {
            document: br#"{"token":"old"}"#.to_vec(),
            source: source(),
        }))
        .unwrap();
    assert_eq!(
        probe,
        CredentialRecoveryLoadedProbe::Resolved(CredentialRecoveryResult {
            document: Some(br#"{"token":"old"}"#.to_vec()),
            source: Some(source()),
            issue: None,
        })
    );
}

#[test]
fn bridge_retries_pending_write_after_read_failure() {
    let ledger = CredentialRecoveryLedger::new();
    let failure = ledger.finish(proposed(), CredentialRecoverySave::Failed, None).unwrap();
    assert_eq!(
        failure.issue.as_deref(),
        Some(quotaline_credentials::save_recovery::PERSISTENCE_DETAIL)
    );
    assert!(ledger.pending().is_some());
    assert_eq!(
        ledger.probe_read_error(),
        CredentialRecoveryReadProbe::Resolved(CredentialRecoveryResult {
            document: Some(br#"{"token":"new"}"#.to_vec()),
            source: Some(source()),
            issue: Some(quotaline_credentials::save_recovery::PERSISTENCE_DETAIL.into()),
        })
    );
    let retry = ledger
        .probe_loaded(Some(CredentialRecoveryStored {
            document: br#"{"token":"old"}"#.to_vec(),
            source: source(),
        }))
        .unwrap();
    assert_eq!(
        retry,
        CredentialRecoveryLoadedProbe::RetrySave(CredentialRecoveryPending {
            detail: quotaline_credentials::save_recovery::PERSISTENCE_DETAIL.into(),
            ..proposed()
        })
    );
}

#[test]
fn bridge_returns_changed_source() {
    let ledger = CredentialRecoveryLedger::new();
    let changed = CredentialRecoveryStored {
        document: br#"{"token":"other"}"#.to_vec(),
        source: source(),
    };
    let result = ledger
        .finish(proposed(), CredentialRecoverySave::Changed, Some(changed))
        .unwrap();
    assert_eq!(result.document, Some(br#"{"token":"other"}"#.to_vec()));
    assert_eq!(result.source, Some(source()));
}

#[test]
fn bridge_reports_removed_credential() {
    let ledger = CredentialRecoveryLedger::new();
    let result = ledger
        .finish(proposed(), CredentialRecoverySave::Removed, None)
        .unwrap();
    assert_eq!(result.document, None);
    assert_eq!(result.source, None);
}

#[test]
fn bridge_reports_saved_credential() {
    let ledger = CredentialRecoveryLedger::new();
    let result = ledger.finish(proposed(), CredentialRecoverySave::Saved, None).unwrap();
    assert_eq!(result.document, Some(br#"{"token":"new"}"#.to_vec()));
    assert_eq!(result.issue, None);
}

#[test]
fn bridge_rejects_invalid_loaded_document() {
    let ledger = CredentialRecoveryLedger::new();
    assert_eq!(
        ledger.probe_loaded(Some(CredentialRecoveryStored {
            document: b"{".to_vec(),
            source: source(),
        })),
        Err(CredentialRecoveryError::InvalidJson)
    );
}

#[test]
fn bridge_rejects_invalid_refreshed_document() {
    let ledger = CredentialRecoveryLedger::new();
    let mut proposed = proposed();
    proposed.document = b"{".to_vec();
    assert_eq!(
        ledger.finish(proposed, CredentialRecoverySave::Saved, None),
        Err(CredentialRecoveryError::InvalidJson)
    );
}

#[test]
fn bridge_rejects_invalid_replaced_document() {
    let ledger = CredentialRecoveryLedger::new();
    let mut proposed = proposed();
    proposed.replacing = b"{".to_vec();
    assert_eq!(
        ledger.finish(proposed, CredentialRecoverySave::Saved, None),
        Err(CredentialRecoveryError::InvalidJson)
    );
}

#[test]
fn bridge_rejects_missing_changed_document() {
    let ledger = CredentialRecoveryLedger::new();
    assert_eq!(
        ledger.finish(proposed(), CredentialRecoverySave::Changed, None),
        Err(CredentialRecoveryError::MissingChangedCredential)
    );
}

#[test]
fn bridge_rejects_invalid_changed_document() {
    let ledger = CredentialRecoveryLedger::new();
    assert_eq!(
        ledger.finish(
            proposed(),
            CredentialRecoverySave::Changed,
            Some(CredentialRecoveryStored {
                document: b"{".to_vec(),
                source: source(),
            }),
        ),
        Err(CredentialRecoveryError::InvalidJson)
    );
}

#[test]
fn bridge_reports_missing_loaded_document() {
    let ledger = CredentialRecoveryLedger::new();
    let probe = ledger.probe_loaded(None).unwrap();
    assert_eq!(
        probe,
        CredentialRecoveryLoadedProbe::Resolved(CredentialRecoveryResult {
            document: None,
            source: None,
            issue: None,
        })
    );
}
