//! Projects Claude Code OAuth credentials into typed fields.

use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct ClaudeCredentialDocument {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<f64>,
    pub scopes: Vec<String>,
    pub subscription_type: Option<String>,
    pub rate_limit_tier: Option<String>,
    pub has_profile_scope: bool,
}

pub fn claude_credential_document(document: &Value) -> Option<ClaudeCredentialDocument> {
    let oauth = &document["claudeAiOauth"];
    let access_token = oauth["accessToken"].as_str()?.to_owned();
    let scopes: Vec<String> = oauth["scopes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|scope| scope.as_str().map(str::to_owned))
        .collect();
    let has_profile_scope = scopes.iter().any(|scope| scope == "user:profile");
    Some(ClaudeCredentialDocument {
        access_token,
        refresh_token: oauth["refreshToken"].as_str().map(str::to_owned),
        expires_at: oauth["expiresAt"].as_f64().map(|value| value / 1000.0),
        scopes,
        subscription_type: oauth["subscriptionType"].as_str().map(str::to_owned),
        rate_limit_tier: oauth["rateLimitTier"].as_str().map(str::to_owned),
        has_profile_scope,
    })
}

#[cfg(test)]
#[path = "../../tests/unit/claude/document/tests.rs"]
mod tests;
