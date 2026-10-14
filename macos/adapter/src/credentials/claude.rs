//! Exposes the shared Claude credential projection to Swift.

use quotaline_credentials::ClaudeCredentialDocument;
use quotaline_credentials::claude_credential_document;
use quotaline_credentials::claude_initial_document;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ClaudeCredentialRecord {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<f64>,
    pub scopes: Vec<String>,
    pub subscription_type: Option<String>,
    pub rate_limit_tier: Option<String>,
    pub has_profile_scope: bool,
}

impl From<ClaudeCredentialDocument> for ClaudeCredentialRecord {
    fn from(value: ClaudeCredentialDocument) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            expires_at: value.expires_at,
            scopes: value.scopes,
            subscription_type: value.subscription_type,
            rate_limit_tier: value.rate_limit_tier,
            has_profile_scope: value.has_profile_scope,
        }
    }
}

#[uniffi::export]
pub fn claude_credential_record(payload: Vec<u8>) -> Option<ClaudeCredentialRecord> {
    let document = serde_json::from_slice(&payload).ok()?;
    claude_credential_document(&document).map(Into::into)
}

#[uniffi::export]
pub fn claude_initial_credentials(
    access_token: String,
    refresh_token: Option<String>,
    expires_at_millis: Option<i64>,
    scopes: Vec<String>,
    subscription_type: Option<String>,
    rate_limit_tier: Option<String>,
) -> Vec<u8> {
    serde_json::to_vec(&claude_initial_document(
        &access_token,
        refresh_token.as_deref(),
        expires_at_millis,
        &scopes,
        subscription_type.as_deref(),
        rate_limit_tier.as_deref(),
    ))
    .expect("Claude credential document is serializable")
}

#[cfg(test)]
#[path = "../../tests/unit/credentials/claude/tests.rs"]
mod tests;
