//! Exposes shared OAuth refresh completion to the macOS client.

use quotaline_credentials as credentials;
use quotaline_credentials::RefreshCompletionError;
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum RefreshProvider {
    Claude,
    Codex,
    Gemini,
    Antigravity,
}

impl From<RefreshProvider> for credentials::RefreshProvider {
    fn from(provider: RefreshProvider) -> Self {
        match provider {
            RefreshProvider::Claude => Self::Claude,
            RefreshProvider::Codex => Self::Codex,
            RefreshProvider::Gemini => Self::Gemini,
            RefreshProvider::Antigravity => Self::Antigravity,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefreshProblem {
    MalformedResponse,
    MissingAccessToken,
    InvalidExpiry,
    InvalidCredential,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefreshCompletion {
    pub document: Option<Vec<u8>>,
    pub problem: Option<RefreshProblem>,
    pub detail: Option<String>,
}

impl RefreshCompletion {
    fn success(document: Value) -> Self {
        Self {
            document: Some(serde_json::to_vec(&document).expect("credential document is serializable")),
            problem: None,
            detail: None,
        }
    }

    fn failure(provider: credentials::RefreshProvider, error: RefreshCompletionError) -> Self {
        let problem = match error {
            RefreshCompletionError::MalformedResponse => RefreshProblem::MalformedResponse,
            RefreshCompletionError::MissingAccessToken(_) => RefreshProblem::MissingAccessToken,
            RefreshCompletionError::InvalidExpiry => RefreshProblem::InvalidExpiry,
            RefreshCompletionError::InvalidCredential => RefreshProblem::InvalidCredential,
        };
        Self {
            document: None,
            problem: Some(problem),
            detail: Some(match error {
                RefreshCompletionError::MissingAccessToken(None) => "refresh returned no access token".into(),
                _ => error.message(provider),
            }),
        }
    }
}

pub fn complete_credential_refresh(
    provider: RefreshProvider,
    document: Vec<u8>,
    response: Vec<u8>,
    now_millis: i64,
    refreshed_at: String,
) -> RefreshCompletion {
    let provider = provider.into();
    let Ok(document) = serde_json::from_slice(&document) else {
        return RefreshCompletion::failure(provider, RefreshCompletionError::InvalidCredential);
    };
    let Ok(response) = serde_json::from_slice(&response) else {
        return RefreshCompletion::failure(provider, RefreshCompletionError::MalformedResponse);
    };
    match credentials::complete_refresh(provider, &document, &response, now_millis, &refreshed_at) {
        Ok(document) => RefreshCompletion::success(document),
        Err(error) => RefreshCompletion::failure(provider, error),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AntigravityRefreshCompletion {
    pub access_token: Option<String>,
    pub expires_at: Option<f64>,
    pub problem: Option<RefreshProblem>,
    pub detail: Option<String>,
}

pub fn complete_antigravity_refresh(
    stored_access_token: String,
    response: Vec<u8>,
    now_millis: i64,
) -> AntigravityRefreshCompletion {
    let provider = RefreshProvider::Antigravity.into();
    let response = serde_json::from_slice(&response);
    let completed = response
        .map_err(|_| RefreshCompletionError::MalformedResponse)
        .and_then(|response| {
            credentials::complete_refresh(
                provider,
                &json!({"access_token":stored_access_token}),
                &response,
                now_millis,
                "",
            )
        });
    match completed {
        Ok(document) => AntigravityRefreshCompletion {
            access_token: document["access_token"].as_str().map(str::to_owned),
            expires_at: document["expiry"].as_f64(),
            problem: None,
            detail: None,
        },
        Err(error) => {
            let failure = RefreshCompletion::failure(provider, error);
            AntigravityRefreshCompletion {
                access_token: None,
                expires_at: None,
                problem: failure.problem,
                detail: failure.detail,
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/providers/refresh_completion/tests.rs"]
mod tests;
