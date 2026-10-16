use super::*;
use serde_json::json;

#[test]
fn document_projects_oauth_fields_and_profile_scope() {
    assert_eq!(
        claude_credential_document(&json!({"claudeAiOauth":{
            "accessToken":"access","refreshToken":"refresh","expiresAt":1500,
            "scopes":["user:profile",1,"user:inference"],
            "subscriptionType":"pro","rateLimitTier":"default"
        }})),
        Some(ClaudeCredentialDocument {
            access_token: "access".into(),
            refresh_token: Some("refresh".into()),
            expires_at: Some(1.5),
            scopes: vec!["user:profile".into(), "user:inference".into()],
            subscription_type: Some("pro".into()),
            rate_limit_tier: Some("default".into()),
            has_profile_scope: true,
        })
    );
}

#[test]
fn document_defaults_optional_fields_and_requires_access() {
    assert_eq!(
        claude_credential_document(&json!({"claudeAiOauth":{"accessToken":"access"}})),
        Some(ClaudeCredentialDocument {
            access_token: "access".into(),
            refresh_token: None,
            expires_at: None,
            scopes: vec![],
            subscription_type: None,
            rate_limit_tier: None,
            has_profile_scope: false,
        })
    );
    assert_eq!(claude_credential_document(&json!({"claudeAiOauth":{}})), None);
}
