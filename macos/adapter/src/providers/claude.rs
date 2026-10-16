//! Exposes the shared Claude response projector to macOS.

use crate::claude::ClaudeProfileCache;
use crate::http::{NativeHttpTransport, NativeTransport, ProviderFailure};
use crate::provider_record::ProviderRecord;
use futures_util::future::join;
use quotaline_claude as claude;
use quotaline_claude::request;
use quotaline_credentials::ClaudeRefresh;
use quotaline_credentials::claude_refreshed_document;
use serde_json::Value;
use std::sync::Arc;

#[uniffi::export]
pub fn claude_updated_credentials(
    document: Vec<u8>,
    access_token: String,
    refresh_token: Option<String>,
    expires_in: f64,
    now_millis: i64,
) -> Option<Vec<u8>> {
    let document: Value = serde_json::from_slice(&document).ok()?;
    let token = ClaudeRefresh {
        access_token,
        refresh_token,
        expires_in,
    };
    Some(
        serde_json::to_vec(&claude_refreshed_document(&document, &token, now_millis)?)
            .expect("Claude document is serializable"),
    )
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum ClaudeFetchOutcome {
    Provider(Box<ProviderRecord>),
    Failure(ProviderFailure),
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ClaudeFetchResult {
    pub outcome: ClaudeFetchOutcome,
    pub profile_warning: Option<String>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ClaudeFetchInput {
    pub token: String,
    pub auth: Vec<u8>,
    pub fingerprint: String,
    pub now: f64,
    pub hint: String,
    pub home: String,
}

#[uniffi::export]
pub async fn claude_fetch(
    input: ClaudeFetchInput,
    cache: Arc<ClaudeProfileCache>,
    transport: Arc<dyn NativeHttpTransport>,
) -> ClaudeFetchResult {
    let native = NativeTransport(transport.clone());
    let usage = claude::fetch::fetch_usage(&native, request::USAGE_URL, &input.token);
    let profile = cache.fetch(
        input.token.clone(),
        input.fingerprint,
        input.now,
        input.home.clone(),
        transport,
    );
    let (usage, profile) = join(usage, profile).await;
    let outcome = match usage {
        Ok(usage) => match claude_projection(
            serde_json::to_vec(&usage).expect("a JSON value serializes"),
            input.auth,
            profile.value,
        ) {
            Ok(provider) => ClaudeFetchOutcome::Provider(Box::new(provider)),
            Err(_) => ClaudeFetchOutcome::Failure(crate::http::http_provider_failure(
                crate::http::HttpErrorRecord::Decoding {
                    detail: "claude.usage: invalid payload".into(),
                },
                input.hint,
                Some(input.home),
            )),
        },
        Err(error) => ClaudeFetchOutcome::Failure(error.provider_failure(&input.hint, Some(&input.home))),
    };
    ClaudeFetchResult {
        outcome,
        profile_warning: profile.warning,
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum ClaudeProjectionError {
    #[error("Invalid Claude response JSON")]
    InvalidJson,
    #[error("No readable Claude usage fields")]
    NoReadableQuotas,
}

#[uniffi::export]
pub fn claude_projection(
    usage: Vec<u8>,
    auth: Vec<u8>,
    profile: Option<Vec<u8>>,
) -> Result<ProviderRecord, ClaudeProjectionError> {
    let usage: Value = serde_json::from_slice(&usage).map_err(|_| ClaudeProjectionError::InvalidJson)?;
    let auth: Value = serde_json::from_slice(&auth).map_err(|_| ClaudeProjectionError::InvalidJson)?;
    let profile: Option<Value> = profile
        .as_deref()
        .map(serde_json::from_slice)
        .transpose()
        .map_err(|_| ClaudeProjectionError::InvalidJson)?;
    if !usage.is_object() || usage.get("error").is_some() || !auth.is_object() {
        return Err(ClaudeProjectionError::InvalidJson);
    }
    claude::project(claude::Input {
        data: &usage,
        auth: &auth["claudeAiOauth"],
        profile: profile.as_ref(),
        source: "network".into(),
    })
    .map(Into::into)
    .map_err(|_| ClaudeProjectionError::NoReadableQuotas)
}

#[cfg(test)]
#[path = "../../tests/unit/providers/claude/tests.rs"]
mod tests;
