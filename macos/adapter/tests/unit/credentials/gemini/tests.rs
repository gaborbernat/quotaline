use super::*;

#[test]
fn initial_credentials_bridge_serializes_gemini_document() {
    let payload = gemini_initial_credentials("access".into(), Some("refresh".into()), None, Some(1_500.5));
    let document: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(
        document,
        serde_json::json!({
            "access_token":"access","token_type":"Bearer","refresh_token":"refresh","expiry_date":1500.5
        })
    );
}

#[test]
fn keychain_credentials_bridge_preserves_wrapped_fields() {
    let payload = gemini_keychain_credentials(
        br#"{"token":{"accessToken":"access","extra":"keep"}}"#.to_vec(),
        2_000.5,
    )
    .unwrap();
    let document: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(
        document,
        serde_json::json!({
            "token":{"accessToken":"access","extra":"keep"},"serverName":"main-account","updatedAt":2000.5
        })
    );
}

#[test]
fn keychain_credentials_bridge_rejects_invalid_document() {
    assert_eq!(gemini_keychain_credentials(b"invalid".to_vec(), 1.0), None);
    assert_eq!(gemini_keychain_credentials(b"{}".to_vec(), 1.0), None);
}

#[test]
fn credential_storage_bridge_maps_cli_setting() {
    assert_eq!(gemini_credential_storage(None), GeminiStorage::File);
    assert_eq!(gemini_credential_storage(Some("true".into())), GeminiStorage::Keychain);
}

#[test]
fn credential_record_exposes_wrapped_token_fields() {
    let record = gemini_credential_record(
        br#"{"token":{"accessToken":"access","refreshToken":"refresh","expiresAt":2000}}"#.to_vec(),
    )
    .unwrap();
    assert_eq!(record.access_token, "access");
    assert_eq!(record.refresh_token.as_deref(), Some("refresh"));
    assert_eq!(record.expires_at, Some(2.0));
    assert_eq!(gemini_credential_record(b"invalid".to_vec()), None);
}
