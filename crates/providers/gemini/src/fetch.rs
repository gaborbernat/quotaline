//! Fetches Gemini plan details and quota through a desktop's transport.

use std::sync::Mutex;

use serde_json::Value;

use super::{AssistCache, UNSUPPORTED_CLIENT_MESSAGE, has_credit_balances, project_id, request, subscription_required};
use quotaline_http::transport::{Request, Transport, json_document};
use quotaline_http::{HttpError, ProviderFailureKind};

pub struct Context<'a> {
    pub base: &'a str,
    pub headers: Vec<(String, String)>,
    pub fingerprint: &'a str,
    pub now: f64,
    pub hint: &'a str,
    pub home: Option<&'a str>,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Quota { quota: Value, assist: Value },
    Unsupported(String),
    Failure(HttpError),
}

#[derive(Debug, PartialEq)]
pub struct Fetch {
    pub outcome: Outcome,
    pub warnings: Vec<String>,
}

pub async fn fetch(transport: &impl Transport, cache: &Mutex<AssistCache<Value>>, context: Context<'_>) -> Fetch {
    let mut warnings = Vec::new();
    let assist = match assist(transport, cache, &context).await {
        Ok(assist) => assist,
        Err(error) => {
            // Signing in again or waiting out a rate limit is required; other plan failures still allow quota.
            let kind = error.provider_failure(context.hint, context.home).kind;
            if matches!(
                kind,
                ProviderFailureKind::NotAuthenticated | ProviderFailureKind::RateLimited
            ) {
                return Fetch {
                    outcome: Outcome::Failure(error),
                    warnings,
                };
            }
            warnings.push(format!("Plan details unavailable: {}", error.message(context.home)));
            Value::Null
        }
    };
    if let Some(reason) = super::unsupported_reason(&assist) {
        return Fetch {
            outcome: Outcome::Unsupported(reason),
            warnings,
        };
    }
    let body = serde_json::to_vec(&request::quota_body(project_id(&assist))).expect("Gemini quota body serializes");
    let quota = transport
        .send(Request::post_json(
            request::endpoint(context.base, "retrieveUserQuota"),
            context.headers.clone(),
            "gemini.quota",
            body,
        ))
        .await
        .and_then(|payload| json_document(&payload, "gemini.quota"));
    let outcome = match quota {
        Ok(quota) => Outcome::Quota { quota, assist },
        Err(HttpError::Response {
            status: 403, ref body, ..
        }) if subscription_required(body) => Outcome::Unsupported(UNSUPPORTED_CLIENT_MESSAGE.into()),
        Err(error) => Outcome::Failure(error),
    };
    Fetch { outcome, warnings }
}

async fn assist(
    transport: &impl Transport,
    cache: &Mutex<AssistCache<Value>>,
    context: &Context<'_>,
) -> Result<Value, HttpError> {
    if let Some(cached) = cache.lock().unwrap().read(context.fingerprint, context.now) {
        return Ok(cached);
    }
    let body = serde_json::to_vec(&request::assist_body()).expect("Gemini assist body serializes");
    let payload = transport
        .send(Request::post_json(
            request::endpoint(context.base, "loadCodeAssist"),
            context.headers.clone(),
            "gemini.load-code-assist",
            body,
        ))
        .await?;
    let assist = json_document(&payload, "gemini.load-code-assist")?;
    cache.lock().unwrap().record(
        context.fingerprint,
        assist.clone(),
        has_credit_balances(&assist),
        context.now,
    );
    Ok(assist)
}

#[cfg(test)]
#[path = "../tests/unit/fetch/tests.rs"]
mod tests;
