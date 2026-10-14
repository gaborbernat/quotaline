use super::*;

#[test]
fn initial_credentials_bridge_serializes_claude_document() {
    let payload = claude_initial_credentials(
        "access".into(),
        Some("refresh".into()),
        Some(1_500),
        vec!["user:profile".into()],
        Some("pro".into()),
        None,
    );
    let document: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(
        document,
        serde_json::json!({"claudeAiOauth":{
            "accessToken":"access","refreshToken":"refresh","expiresAt":1500,
            "scopes":["user:profile"],"subscriptionType":"pro"
        }})
    );
}

#[test]
fn credential_record_exposes_scope_and_expiry() {
    let record = claude_credential_record(
        br#"{"claudeAiOauth":{"accessToken":"access","refreshToken":"refresh","expiresAt":1500,"scopes":["user:profile"],"subscriptionType":"pro","rateLimitTier":"default"}}"#.to_vec(),
    )
    .unwrap();
    assert_eq!(record.access_token, "access");
    assert_eq!(record.refresh_token.as_deref(), Some("refresh"));
    assert_eq!(record.expires_at, Some(1.5));
    assert!(record.has_profile_scope);
    assert_eq!(record.scopes, ["user:profile"]);
    assert_eq!(record.subscription_type.as_deref(), Some("pro"));
    assert_eq!(record.rate_limit_tier.as_deref(), Some("default"));
    assert_eq!(claude_credential_record(b"invalid".to_vec()), None);
}
