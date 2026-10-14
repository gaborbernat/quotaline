//! Defines Claude OAuth refresh payloads and credential updates.

use serde_json::{Value, json};

mod document;
pub use document::{ClaudeCredentialDocument, claude_credential_document};

pub const CLAUDE_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
pub const CLAUDE_USER_AGENT: &str = "claude-code/2.1.251";

pub fn claude_initial_document(
    access_token: &str,
    refresh_token: Option<&str>,
    expires_at_millis: Option<i64>,
    scopes: &[String],
    subscription_type: Option<&str>,
    rate_limit_tier: Option<&str>,
) -> Value {
    let mut oauth = json!({"accessToken":access_token,"scopes":scopes});
    for (key, value) in [
        ("refreshToken", refresh_token.map(str::to_owned)),
        ("subscriptionType", subscription_type.map(str::to_owned)),
        ("rateLimitTier", rate_limit_tier.map(str::to_owned)),
    ] {
        if let Some(value) = value {
            oauth[key] = Value::String(value);
        }
    }
    if let Some(expires_at_millis) = expires_at_millis {
        oauth["expiresAt"] = json!(expires_at_millis);
    }
    json!({"claudeAiOauth":oauth})
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClaudeRefresh {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaudeRefreshError {
    MissingAccessToken,
    InvalidExpiry,
}

pub fn claude_refresh_request(refresh_token: &str) -> Value {
    json!({"grant_type":"refresh_token","refresh_token":refresh_token,"client_id":CLAUDE_CLIENT_ID})
}

pub fn claude_refresh_response(response: &Value) -> Result<ClaudeRefresh, ClaudeRefreshError> {
    let access_token = response["access_token"]
        .as_str()
        .filter(|token| !token.is_empty())
        .ok_or(ClaudeRefreshError::MissingAccessToken)?;
    let expires_in = response["expires_in"]
        .as_f64()
        .or_else(|| response["expires_in"].as_str().and_then(|value| value.parse().ok()))
        .unwrap_or(3600.0);
    if !expires_in.is_finite() || expires_in <= 0.0 {
        return Err(ClaudeRefreshError::InvalidExpiry);
    }
    Ok(ClaudeRefresh {
        access_token: access_token.into(),
        refresh_token: response["refresh_token"]
            .as_str()
            .filter(|token| !token.is_empty())
            .map(str::to_owned),
        expires_in,
    })
}

pub fn claude_refreshed_document(document: &Value, token: &ClaudeRefresh, now_ms: i64) -> Option<Value> {
    let mut updated = document.clone();
    let oauth = updated.get_mut("claudeAiOauth")?.as_object_mut()?;
    oauth.insert("accessToken".into(), Value::String(token.access_token.clone()));
    if let Some(refresh_token) = &token.refresh_token {
        oauth.insert("refreshToken".into(), Value::String(refresh_token.clone()));
    }
    oauth.insert(
        "expiresAt".into(),
        json!(now_ms.saturating_add((token.expires_in * 1000.0) as i64)),
    );
    Some(updated)
}

#[cfg(test)]
#[path = "../tests/unit/claude/tests.rs"]
mod tests;
