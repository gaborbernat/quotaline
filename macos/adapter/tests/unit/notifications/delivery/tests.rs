use super::*;

#[test]
fn ffi_delivery_ledger_maps_every_event_kind() {
    let ledger = NotificationDeliveryLedger::new();
    let events = [
        NotificationKindRecord::Threshold,
        NotificationKindRecord::Reset,
        NotificationKindRecord::Authentication,
        NotificationKindRecord::Credits,
        NotificationKindRecord::ExpiringCredit,
        NotificationKindRecord::WillRunOut,
        NotificationKindRecord::AheadOfPace,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, kind)| DeliveryEventRecord {
        id: index.to_string(),
        kind,
        window_id: Some("five-hour".into()),
    })
    .collect();
    assert_eq!(
        ledger.filter(
            "claude".into(),
            Some(DeliveryCredentialRecord::Missing {
                reason: "missing".into()
            }),
            None,
            events
        ),
        [true; 7]
    );
}

#[test]
fn ffi_delivery_ledger_maps_credential_variants_and_forgets_state() {
    let ledger = NotificationDeliveryLedger::new();
    let auth = || {
        vec![DeliveryEventRecord {
            id: "auth".into(),
            kind: NotificationKindRecord::Authentication,
            window_id: None,
        }]
    };
    assert_eq!(
        ledger.filter(
            "claude".into(),
            Some(DeliveryCredentialRecord::Expired { at: 42.0 }),
            None,
            auth()
        ),
        [true]
    );
    assert_eq!(
        ledger.filter(
            "claude".into(),
            Some(DeliveryCredentialRecord::Expired { at: 42.0 }),
            None,
            auth()
        ),
        [false]
    );
    assert_eq!(
        ledger.filter(
            "claude".into(),
            Some(DeliveryCredentialRecord::Valid { expires_at: None }),
            None,
            auth()
        ),
        [true]
    );
    ledger.forget_sign_in("claude".into());
    assert_eq!(
        ledger.filter(
            "claude".into(),
            Some(DeliveryCredentialRecord::Valid { expires_at: None }),
            None,
            auth()
        ),
        [true]
    );
    ledger.clear_pace();
    ledger.forget_pace("claude".into());
    ledger.retain_providers(vec!["codex".into()]);
    assert_eq!(
        ledger.filter(
            "claude".into(),
            Some(DeliveryCredentialRecord::Valid { expires_at: None }),
            Some(vec![]),
            auth()
        ),
        [true]
    );
}
