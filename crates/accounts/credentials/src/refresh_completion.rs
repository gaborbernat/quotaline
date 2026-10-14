//! Applies provider refresh responses to stored credentials.

use serde_json::Value;

use super::{
    ClaudeRefreshError, GoogleRefreshError, antigravity_refreshed_document, claude_refresh_response,
    claude_refreshed_document, codex_refresh_response, codex_refreshed_document, gemini_refreshed_document,
    google_refresh_response,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshProvider {
    Claude,
    Codex,
    Gemini,
    Antigravity,
}

impl RefreshProvider {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
            Self::Gemini => "Gemini",
            Self::Antigravity => "Antigravity",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefreshCompletionError {
    MalformedResponse,
    MissingAccessToken(Option<String>),
    InvalidExpiry,
    InvalidCredential,
}

impl RefreshCompletionError {
    pub fn message(&self, provider: RefreshProvider) -> String {
        match self {
            Self::MalformedResponse => format!("{} refresh returned an invalid payload", provider.name()),
            Self::MissingAccessToken(Some(detail)) => detail.clone(),
            Self::MissingAccessToken(None) => format!("{} refresh returned no access token", provider.name()),
            Self::InvalidExpiry => format!("{} refresh returned an invalid expiry", provider.name()),
            Self::InvalidCredential => format!("{} credentials cannot be updated", provider.name()),
        }
    }
}

pub fn complete_refresh(
    provider: RefreshProvider,
    document: &Value,
    response: &Value,
    now_millis: i64,
    refreshed_at: &str,
) -> Result<Value, RefreshCompletionError> {
    use RefreshCompletionError as Error;
    match provider {
        RefreshProvider::Claude => {
            let token = claude_refresh_response(response).map_err(|error| match error {
                ClaudeRefreshError::MissingAccessToken => Error::MissingAccessToken(None),
                ClaudeRefreshError::InvalidExpiry => Error::InvalidExpiry,
            })?;
            claude_refreshed_document(document, &token, now_millis).ok_or(Error::InvalidCredential)
        }
        RefreshProvider::Codex => {
            let token = codex_refresh_response(response).ok_or(Error::MalformedResponse)?;
            if token.access_token.is_none() {
                return Err(Error::MissingAccessToken(token.error));
            }
            codex_refreshed_document(document, &token, refreshed_at).ok_or(Error::InvalidCredential)
        }
        RefreshProvider::Gemini | RefreshProvider::Antigravity => {
            let token = google_refresh_response(response).map_err(|error| match error {
                GoogleRefreshError::InvalidResponse => Error::MalformedResponse,
                GoogleRefreshError::InvalidExpiry => Error::InvalidExpiry,
            })?;
            if token.access_token.is_none() {
                return Err(Error::MissingAccessToken(token.error_description.or(token.error)));
            }
            let refreshed = if provider == RefreshProvider::Gemini {
                gemini_refreshed_document(document, &token, now_millis)
            } else {
                antigravity_refreshed_document(document, &token, now_millis.div_euclid(1000))
            };
            refreshed.ok_or(Error::InvalidCredential)
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/refresh_completion/tests.rs"]
mod tests;
