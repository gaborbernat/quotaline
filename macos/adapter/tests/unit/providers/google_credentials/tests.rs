use super::*;
use serde_json::json;

#[test]
fn updated_gemini_credentials_preserve_shape_and_rotations() {
    let token = GoogleRefreshToken {
        access_token: Some("new".into()),
        refresh_token: Some("r".into()),
        id_token: Some("i".into()),
        expires_in: 60.0,
    };
    let updated =
        gemini_updated_credentials(br#"{"token":{"accessToken":"old"},"other":1}"#.to_vec(), token, 1000).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&updated).unwrap(),
        json!({"token":{"accessToken":"new","refreshToken":"r","idToken":"i","expiresAt":61000},"other":1})
    );
}

#[test]
fn updated_gemini_credentials_reject_invalid_documents_and_missing_access() {
    let token = GoogleRefreshToken {
        access_token: Some("new".into()),
        refresh_token: None,
        id_token: None,
        expires_in: 3600.0,
    };
    assert_eq!(gemini_updated_credentials(b"invalid".to_vec(), token.clone(), 0), None);
    assert_eq!(gemini_updated_credentials(b"[]".to_vec(), token, 0), None);
    let missing = GoogleRefreshToken {
        access_token: None,
        refresh_token: None,
        id_token: None,
        expires_in: 3600.0,
    };
    assert_eq!(gemini_updated_credentials(b"{}".to_vec(), missing, 0), None);
}
