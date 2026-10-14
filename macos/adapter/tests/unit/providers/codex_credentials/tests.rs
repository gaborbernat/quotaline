use super::*;
use serde_json::json;

#[test]
fn updated_credentials_preserve_unrelated_fields() {
    let token = CodexRefreshToken {
        access_token: Some("new".into()),
        refresh_token: None,
        id_token: None,
    };
    let updated = codex_updated_credentials(
        br#"{"tokens":{"access_token":"old","refresh_token":"original"},"auth_mode":"chatgpt"}"#.to_vec(),
        token,
        "now".into(),
    )
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&updated).unwrap(),
        json!({"tokens":{"access_token":"new","refresh_token":"original"},"auth_mode":"chatgpt","last_refresh":"now"})
    );
}

#[test]
fn updated_credentials_reject_invalid_documents_and_missing_access() {
    let token = CodexRefreshToken {
        access_token: Some("new".into()),
        refresh_token: None,
        id_token: None,
    };
    assert_eq!(
        codex_updated_credentials(b"invalid".to_vec(), token.clone(), "now".into()),
        None
    );
    assert_eq!(codex_updated_credentials(b"[]".to_vec(), token, "now".into()), None);
    let missing = CodexRefreshToken {
        access_token: None,
        refresh_token: None,
        id_token: None,
    };
    assert_eq!(
        codex_updated_credentials(br#"{"tokens":{}}"#.to_vec(), missing, "now".into()),
        None
    );
}
