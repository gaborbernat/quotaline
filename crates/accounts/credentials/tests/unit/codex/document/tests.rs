use super::*;
use base64::Engine;
use serde_json::json;

fn token(claims: Value) -> String {
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string());
    format!("e30.{encoded}.signature")
}

#[test]
fn document_prefers_oauth_and_projects_claims() {
    let id = token(json!({
        "email":"user@example.com",
        "https://api.openai.com/auth": {
            "chatgpt_account_id":"account",
            "chatgpt_plan_type":"pro",
            "chatgpt_subscription_active_until":"2026-10-13T00:00:00Z"
        }
    }));
    assert_eq!(
        codex_credential_document(&json!({
            "OPENAI_API_KEY":"key",
            "tokens":{"access_token":"access","refresh_token":"refresh","id_token":id},
            "last_refresh":"2026-10-12T00:00:00Z"
        })),
        Some(CodexCredentialDocument {
            access_token: "access".into(),
            refresh_token: Some("refresh".into()),
            id_token: Some(id),
            account_id: Some("account".into()),
            api_key: Some("key".into()),
            last_refresh: Some("2026-10-12T00:00:00Z".into()),
            email: Some("user@example.com".into()),
            plan_type: Some("pro".into()),
            subscription_active_until: Some("2026-10-13T00:00:00+00:00".into()),
            supports_usage_api: true,
        })
    );
}

#[test]
fn document_falls_back_to_api_key_and_explicit_account() {
    assert_eq!(
        codex_credential_document(&json!({"OPENAI_API_KEY":"key","tokens":{"account_id":"explicit"}})),
        Some(CodexCredentialDocument {
            access_token: "key".into(),
            refresh_token: None,
            id_token: None,
            account_id: Some("explicit".into()),
            api_key: Some("key".into()),
            last_refresh: None,
            email: None,
            plan_type: None,
            subscription_active_until: None,
            supports_usage_api: false,
        })
    );
}

#[test]
fn document_requires_a_string_access_token_or_api_key() {
    for value in [json!({"tokens":{}}), json!({"OPENAI_API_KEY":null}), json!([])] {
        assert_eq!(codex_credential_document(&value), None);
        assert!(!codex_credential_available(&value));
    }
    assert!(codex_credential_available(&json!({"tokens":{"access_token":"access"}})));
    assert!(codex_credential_available(&json!({"OPENAI_API_KEY":"key"})));
}
