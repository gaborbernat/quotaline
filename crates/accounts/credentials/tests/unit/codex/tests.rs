use super::*;

#[test]
fn initial_document_contains_supplied_codex_fields() {
    assert_eq!(
        codex_initial_document("access", Some("refresh"), Some("id"), Some("account"), Some("now")),
        json!({"tokens":{"access_token":"access","refresh_token":"refresh","id_token":"id","account_id":"account"},
            "auth_mode":"chatgpt","OPENAI_API_KEY":null,"last_refresh":"now"})
    );
}

#[test]
fn initial_document_omits_absent_codex_fields() {
    assert_eq!(
        codex_initial_document("access", None, None, None, None),
        json!({"tokens":{"access_token":"access"},"auth_mode":"chatgpt","OPENAI_API_KEY":null})
    );
}

#[test]
fn request_headers_include_optional_account() {
    assert_eq!(
        codex_request_headers("access", Some("account")),
        vec![
            ("Authorization", "Bearer access".into()),
            ("originator", "codex_cli_rs".into()),
            ("User-Agent", "codex_cli_rs/quotaline".into()),
            ("ChatGPT-Account-Id", "account".into()),
        ]
    );
    assert_eq!(codex_request_headers("access", None).len(), 3);
}

#[test]
fn refresh_request_uses_cli_client() {
    assert_eq!(
        codex_refresh_request("secret"),
        json!({"client_id":CODEX_CLIENT_ID,"grant_type":"refresh_token","refresh_token":"secret"})
    );
}

#[test]
fn refresh_response_preserves_optional_fields() {
    assert_eq!(
        codex_refresh_response(&json!({"access_token":"a","refresh_token":"r","id_token":"i"})),
        Some(CodexRefresh {
            access_token: Some("a".into()),
            refresh_token: Some("r".into()),
            id_token: Some("i".into()),
            error: None,
        })
    );
    assert_eq!(
        codex_refresh_response(&json!({"error":"invalid_grant"})),
        Some(CodexRefresh {
            access_token: None,
            refresh_token: None,
            id_token: None,
            error: Some("invalid_grant".into()),
        })
    );
    assert_eq!(codex_refresh_response(&json!([])), None);
}

#[test]
fn refreshed_document_preserves_unrelated_fields_and_missing_rotations() {
    let document = json!({"tokens":{"access_token":"old","refresh_token":"original","id_token":"original-id","account_id":"account"},"auth_mode":"chatgpt"});
    let token = codex_refresh_response(&json!({"access_token":"new"})).unwrap();
    assert_eq!(
        codex_refreshed_document(&document, &token, "2026-10-13T00:00:00Z"),
        Some(
            json!({"tokens":{"access_token":"new","refresh_token":"original","id_token":"original-id","account_id":"account"},"auth_mode":"chatgpt","last_refresh":"2026-10-13T00:00:00Z"})
        )
    );
}

#[test]
fn refreshed_document_applies_rotated_tokens() {
    let token = codex_refresh_response(&json!({"access_token":"new","refresh_token":"r","id_token":"i"})).unwrap();
    assert_eq!(
        codex_refreshed_document(&json!({"tokens":{}}), &token, "now"),
        Some(json!({"tokens":{"access_token":"new","refresh_token":"r","id_token":"i"},"last_refresh":"now"}))
    );
}

#[test]
fn refreshed_document_requires_access_token_and_object_document() {
    let missing = codex_refresh_response(&json!({"error":"invalid_grant"})).unwrap();
    assert_eq!(codex_refreshed_document(&json!({"tokens":{}}), &missing, "now"), None);
    let valid = codex_refresh_response(&json!({"access_token":"new"})).unwrap();
    assert_eq!(
        codex_refreshed_document(&json!({"tokens":[]}), &valid, "now"),
        Some(json!({"tokens":{"access_token":"new"},"last_refresh":"now"}))
    );
    assert_eq!(
        codex_refreshed_document(&json!({"OPENAI_API_KEY":"key"}), &valid, "now"),
        Some(json!({"OPENAI_API_KEY":"key","tokens":{"access_token":"new"},"last_refresh":"now"}))
    );
    assert_eq!(codex_refreshed_document(&json!([]), &valid, "now"), None);
}
