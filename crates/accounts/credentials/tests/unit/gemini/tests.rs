use super::*;
use base64::Engine;
use serde_json::json;

#[test]
fn initial_document_contains_supplied_gemini_fields() {
    assert_eq!(
        gemini_initial_document("access", Some("refresh"), Some("identity"), Some(1_500.5)),
        json!({"access_token":"access","token_type":"Bearer","refresh_token":"refresh",
            "id_token":"identity","expiry_date":1500.5})
    );
}

#[test]
fn initial_document_omits_absent_gemini_fields() {
    assert_eq!(
        gemini_initial_document("access", None, None, None),
        json!({"access_token":"access","token_type":"Bearer"})
    );
}

#[test]
fn keychain_document_preserves_wrapped_fields() {
    assert_eq!(
        gemini_keychain_document(
            &json!({"future":"keep","token":{"accessToken":"access","scope":"openid"}}),
            2_000.5
        ),
        Some(json!({"future":"keep","serverName":"main-account","updatedAt":2000.5,
            "token":{"accessToken":"access","scope":"openid"}}))
    );
}

#[test]
fn keychain_document_keeps_existing_server_name() {
    assert_eq!(
        gemini_keychain_document(&json!({"serverName":"other","token":{"accessToken":"access"}}), 1.0).unwrap()["serverName"],
        "other"
    );
}

#[test]
fn keychain_document_wraps_flat_credentials() {
    assert_eq!(
        gemini_keychain_document(
            &json!({"access_token":"access","refresh_token":"refresh",
            "id_token":"identity","expiry_date":1500.5}),
            2_000.5
        ),
        Some(json!({"serverName":"main-account","updatedAt":2000.5,
            "token":{"accessToken":"access","tokenType":"Bearer","refreshToken":"refresh",
                "idToken":"identity","expiresAt":1500.5}}))
    );
}

#[test]
fn keychain_document_omits_absent_flat_fields() {
    assert_eq!(
        gemini_keychain_document(&json!({"access_token":"access"}), 2_000.0),
        Some(json!({"serverName":"main-account","updatedAt":2000.0,
            "token":{"accessToken":"access","tokenType":"Bearer"}}))
    );
}

#[test]
fn keychain_document_rejects_invalid_credentials() {
    assert_eq!(gemini_keychain_document(&json!({}), 1.0), None);
    assert_eq!(gemini_keychain_document(&json!([]), 1.0), None);
}

#[test]
fn storage_mode_requires_exact_cli_value() {
    for (value, expected) in [
        (None, GeminiStorage::File),
        (Some("TRUE"), GeminiStorage::File),
        (Some("true"), GeminiStorage::Keychain),
    ] {
        assert_eq!(gemini_storage_mode(value), expected, "{value:?}");
    }
}

#[test]
fn document_prefers_flat_fields_and_projects_identity() {
    let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(json!({"email":"user@example.com","hd":"example.com"}).to_string());
    let id_token = format!("e30.{claims}.signature");
    assert_eq!(
        gemini_credential_document(&json!({
            "access_token":"flat","refresh_token":"flat-refresh","id_token":id_token,"expiry_date":1500,
            "token":{"accessToken":"wrapped","refreshToken":"wrapped-refresh","expiresAt":2500}
        })),
        Some(GeminiCredentialDocument {
            access_token: "flat".into(),
            refresh_token: Some("flat-refresh".into()),
            id_token: Some(id_token),
            expires_at: Some(1.5),
            email: Some("user@example.com".into()),
            hosted_domain: Some("example.com".into()),
        })
    );
}

#[test]
fn document_reads_wrapped_fields_and_requires_access() {
    assert_eq!(
        gemini_credential_document(
            &json!({"token":{"accessToken":"wrapped","refreshToken":"refresh","expiresAt":2000}})
        ),
        Some(GeminiCredentialDocument {
            access_token: "wrapped".into(),
            refresh_token: Some("refresh".into()),
            id_token: None,
            expires_at: Some(2.0),
            email: None,
            hosted_domain: None,
        })
    );
    assert_eq!(gemini_credential_document(&json!({})), None);
}
