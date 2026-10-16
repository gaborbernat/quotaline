//! Maps Gemini responses through the shared projector.

use crate::http::{NativeHttpTransport, NativeTransport, ProviderFailure};
use crate::provider_record::ProviderRecord;
use quotaline_gemini as gemini;
use quotaline_gemini::AssistCache;
use quotaline_gemini::request;
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Keeps a Gemini account's plan details between refreshes.
#[derive(Default, uniffi::Object)]
pub struct GeminiAssistCache(Mutex<AssistCache<Value>>);

#[uniffi::export]
impl GeminiAssistCache {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }
}

#[uniffi::export]
pub fn gemini_request_headers(token: String, architecture: String) -> std::collections::HashMap<String, String> {
    request::headers(&token, "darwin", Some(&architecture))
        .into_iter()
        .map(|(name, value)| (name.into(), value))
        .collect()
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct GeminiFetchInput {
    pub token: String,
    pub fingerprint: String,
    pub architecture: String,
    pub hosted_domain: Option<String>,
    pub email: Option<String>,
    pub now: f64,
    pub hint: String,
    pub home: String,
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum GeminiFetchOutcome {
    Provider(Box<ProviderRecord>),
    Unsupported(String),
    Failure(ProviderFailure),
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct GeminiFetchResult {
    pub outcome: GeminiFetchOutcome,
    pub warnings: Vec<String>,
}

#[uniffi::export]
pub async fn gemini_fetch(
    input: GeminiFetchInput,
    cache: Arc<GeminiAssistCache>,
    transport: Arc<dyn NativeHttpTransport>,
) -> GeminiFetchResult {
    let fetch = gemini::fetch::fetch(
        &NativeTransport(transport),
        &cache.0,
        gemini::fetch::Context {
            base: request::BASE_URL,
            headers: gemini_request_headers(input.token, input.architecture)
                .into_iter()
                .collect(),
            fingerprint: &input.fingerprint,
            now: input.now,
            hint: &input.hint,
            home: Some(&input.home),
        },
    )
    .await;
    let outcome = match fetch.outcome {
        gemini::fetch::Outcome::Quota { quota, assist } => {
            GeminiFetchOutcome::Provider(Box::new(project(&quota, &assist, input.hosted_domain, input.email)))
        }
        gemini::fetch::Outcome::Unsupported(reason) => GeminiFetchOutcome::Unsupported(reason),
        gemini::fetch::Outcome::Failure(error) => {
            GeminiFetchOutcome::Failure(error.provider_failure(&input.hint, Some(&input.home)))
        }
    };
    GeminiFetchResult {
        outcome,
        warnings: fetch.warnings,
    }
}

fn project(quota: &Value, assist: &Value, hosted_domain: Option<String>, email: Option<String>) -> ProviderRecord {
    gemini::project(
        quota,
        assist,
        hosted_domain.as_deref(),
        email.unwrap_or_default(),
        String::new(),
    )
    .into()
}

#[uniffi::export]
pub fn gemini_subscription_required(body: String) -> bool {
    gemini::subscription_required(&body)
}

#[uniffi::export]
pub fn gemini_unsupported_client_message() -> String {
    gemini::UNSUPPORTED_CLIENT_MESSAGE.into()
}

#[cfg(test)]
#[path = "../../tests/unit/providers/gemini/tests.rs"]
mod tests;
