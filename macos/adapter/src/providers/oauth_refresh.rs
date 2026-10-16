//! Owns OAuth refresh requests, rejection state, and response completion for macOS providers.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use quotaline_claude as claude;
use quotaline_codex as codex;
use quotaline_credentials::CLAUDE_USER_AGENT;
use quotaline_credentials::RefreshRejectionLedger;
use quotaline_credentials::claude_refresh_request;
use quotaline_credentials::codex_refresh_request;
use quotaline_credentials::google_refresh_request;
use quotaline_gemini as gemini;
use serde_json::Value;

use super::oauth_client::OAuthClientRecord;
use super::refresh_completion::{
    RefreshProblem, RefreshProvider, complete_antigravity_refresh, complete_credential_refresh,
};
use crate::http::{
    HttpErrorRecord, NativeHttpPostForm, NativeHttpPostJson, NativeHttpTransport, native_http_post_form,
    native_http_post_json,
};

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct OAuthRefreshInput {
    pub provider: RefreshProvider,
    pub credential: Vec<u8>,
    pub document: Vec<u8>,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub client: Option<OAuthClientRecord>,
    pub now_millis: i64,
    pub refreshed_at: String,
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum OAuthRefreshOutcome {
    Credential { document: Vec<u8> },
    Antigravity { access_token: String, expires_at: f64 },
    MissingRefreshToken,
    OAuthClientUnavailable,
    SignInRequired { code: String },
    Http { error: HttpErrorRecord },
    InvalidResponse { detail: String },
}

#[derive(Default, uniffi::Object)]
pub struct OAuthRefreshSession(Mutex<RefreshRejectionLedger>);

#[uniffi::export]
impl OAuthRefreshSession {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn refresh(
        &self,
        input: OAuthRefreshInput,
        transport: Arc<dyn NativeHttpTransport>,
    ) -> OAuthRefreshOutcome {
        let Some(refresh_token) = input.refresh_token.as_deref() else {
            return OAuthRefreshOutcome::MissingRefreshToken;
        };
        let client = match input.provider {
            RefreshProvider::Gemini | RefreshProvider::Antigravity => match input.client.as_ref() {
                Some(client) => Some(client),
                None => return OAuthRefreshOutcome::OAuthClientUnavailable,
            },
            _ => None,
        };
        let Ok(credential) = serde_json::from_slice::<Value>(&input.credential) else {
            return OAuthRefreshOutcome::InvalidResponse {
                detail: "stored credential is not JSON".into(),
            };
        };
        if let Some(code) = self.0.lock().unwrap().rejected_code("oauth", &credential) {
            return OAuthRefreshOutcome::SignInRequired { code: code.into() };
        }
        let operation = format!("{}.refresh", provider_name(input.provider));
        let response = match input.provider {
            RefreshProvider::Claude | RefreshProvider::Codex => {
                let (url, headers, body) = if input.provider == RefreshProvider::Claude {
                    (
                        claude::request::TOKEN_URL,
                        HashMap::from([("User-Agent".into(), CLAUDE_USER_AGENT.into())]),
                        claude_refresh_request(refresh_token),
                    )
                } else {
                    (
                        codex::request::TOKEN_URL,
                        HashMap::new(),
                        codex_refresh_request(refresh_token),
                    )
                };
                native_http_post_json(
                    transport,
                    NativeHttpPostJson {
                        url: url.into(),
                        headers,
                        operation,
                        body: serde_json::to_vec(&body).expect("OAuth refresh request is serializable"),
                        timeout_millis: None,
                    },
                )
                .await
            }
            RefreshProvider::Gemini | RefreshProvider::Antigravity => {
                let client = client.expect("Google refresh requires an OAuth client");
                native_http_post_form(
                    transport,
                    NativeHttpPostForm {
                        url: gemini::request::TOKEN_URL.into(),
                        headers: HashMap::new(),
                        operation,
                        fields: google_refresh_request(&client.id, &client.secret, refresh_token)
                            .into_iter()
                            .map(|(key, value)| (key.into(), value))
                            .collect(),
                    },
                )
                .await
            }
        };
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                if let HttpErrorRecord::Response { status, body, .. } = &error
                    && let Ok(body) = serde_json::from_str(body)
                    && let Some(code) =
                        self.0
                            .lock()
                            .unwrap()
                            .record_failure("oauth", &credential, *status as u16, &body)
                {
                    return OAuthRefreshOutcome::SignInRequired { code };
                }
                return OAuthRefreshOutcome::Http { error };
            }
        };
        if input.provider == RefreshProvider::Antigravity {
            let completion = complete_antigravity_refresh(input.access_token, response, input.now_millis);
            return match (completion.access_token, completion.expires_at) {
                (Some(access_token), Some(expires_at)) => OAuthRefreshOutcome::Antigravity {
                    access_token,
                    expires_at,
                },
                _ => invalid_completion(input.provider, completion.problem, completion.detail),
            };
        }
        let completion = complete_credential_refresh(
            input.provider,
            input.document,
            response,
            input.now_millis,
            input.refreshed_at,
        );
        match completion.document {
            Some(document) => OAuthRefreshOutcome::Credential { document },
            None => invalid_completion(input.provider, completion.problem, completion.detail),
        }
    }
}

fn invalid_completion(
    provider: RefreshProvider,
    problem: Option<RefreshProblem>,
    detail: Option<String>,
) -> OAuthRefreshOutcome {
    if matches!(
        problem,
        Some(RefreshProblem::MalformedResponse | RefreshProblem::InvalidExpiry)
    ) || provider == RefreshProvider::Claude && problem == Some(RefreshProblem::MissingAccessToken)
    {
        return OAuthRefreshOutcome::Http {
            error: HttpErrorRecord::Decoding {
                detail: format!("{}.refresh: invalid payload", provider_name(provider)),
            },
        };
    }
    OAuthRefreshOutcome::InvalidResponse {
        detail: detail.expect("refresh completion provides a failure detail"),
    }
}

fn provider_name(provider: RefreshProvider) -> &'static str {
    match provider {
        RefreshProvider::Claude => "claude",
        RefreshProvider::Codex => "codex",
        RefreshProvider::Gemini => "gemini",
        RefreshProvider::Antigravity => "antigravity",
    }
}

#[uniffi::export]
pub fn refresh_rejected_message(code: String) -> String {
    quotaline_credentials::refresh_rejected_message(&code)
}

#[uniffi::export]
pub fn refresh_failed_message(provider: String, detail: String) -> String {
    quotaline_credentials::refresh_failed_message(&provider, &detail)
}

#[cfg(test)]
#[path = "../../tests/unit/providers/oauth_refresh/tests.rs"]
mod tests;
