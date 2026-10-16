//! Fetches Codex usage, reset credits, and daily analytics through a desktop's transport.

use std::sync::Mutex;

use futures_util::future::{join, join_all};
use serde_json::Value;
use url::Url;

use super::request;
use crate::codex_usage::{self, ResetCredits};
use quotaline_analytics::watermark::Watermarks;
use quotaline_analytics::{CodexBatchPlan, CodexBatchResult, EndpointResponse, codex_batch};
use quotaline_credentials::codex_request_headers;
use quotaline_http::HttpError;
use quotaline_http::transport::{Request, Transport, json_document};

pub struct Account<'a> {
    pub base: &'a str,
    pub token: &'a str,
    pub account_id: Option<&'a str>,
}

pub async fn fetch_usage(transport: &impl Transport, account: &Account<'_>) -> Result<Value, HttpError> {
    Some(get(transport, account, request::USAGE_PATH, "codex.usage").await?)
        .filter(Value::is_object)
        .ok_or_else(|| invalid("codex.usage"))
}

pub async fn fetch_reset_credits(transport: &impl Transport, account: &Account<'_>) -> Result<ResetCredits, HttpError> {
    codex_usage::reset_credits(&get(transport, account, request::RESET_CREDITS_PATH, "codex.reset-credits").await?)
        .ok_or_else(|| invalid("codex.reset-credits"))
}

pub fn reset_credits_warning(error: &HttpError, home: Option<&str>) -> String {
    format!("Reset credits unavailable: {}", error.message(home))
}

pub struct AnalyticsContext<'a> {
    pub fingerprint: &'a str,
    pub now: f64,
    pub retention_days: u32,
    pub home: Option<&'a str>,
}

/// Fetches each analytics endpoint from its watermark and credit events in parallel, then advances the watermarks.
pub async fn fetch_analytics(
    transport: &impl Transport,
    account: &Account<'_>,
    watermarks: &Mutex<Watermarks>,
    context: &AnalyticsContext<'_>,
) -> CodexBatchResult {
    let plan = CodexBatchPlan::new(&mut watermarks.lock().unwrap(), context.now, context.retention_days);
    let send = |url: String, operation: String| async move {
        transport
            .send(Request::get(url, headers(account), operation))
            .await
            .map_err(|error| error.message(context.home))
    };
    let endpoints = join_all(plan.starts.iter().map(|(endpoint, start)| {
        let mut url =
            Url::parse(&format!("{}/{}", account.base, endpoint.path())).expect("Codex endpoint URL is valid");
        url.query_pairs_mut().extend_pairs(endpoint.query(start, &plan.end));
        let response = send(url.into(), format!("codex.{}", endpoint.name()));
        async move {
            EndpointResponse {
                endpoint: *endpoint,
                result: response.await,
            }
        }
    }));
    let credits = send(
        format!("{}/{}", account.base, request::CREDIT_EVENTS_PATH),
        "codex.credit-events".into(),
    );
    let (endpoints, credits) = join(endpoints, credits).await;
    codex_batch(
        &mut watermarks.lock().unwrap(),
        &plan,
        endpoints,
        credits,
        context.fingerprint.into(),
        context.now,
        context.retention_days,
    )
}

fn headers(account: &Account<'_>) -> Vec<(String, String)> {
    codex_request_headers(account.token, account.account_id)
        .into_iter()
        .map(|(name, value)| (name.into(), value))
        .collect()
}

async fn get(
    transport: &impl Transport,
    account: &Account<'_>,
    path: &str,
    operation: &str,
) -> Result<Value, HttpError> {
    let payload = transport
        .send(Request::get(
            format!("{}/{path}", account.base),
            headers(account),
            operation,
        ))
        .await?;
    json_document(&payload, operation)
}

fn invalid(operation: &str) -> HttpError {
    HttpError::Decoding(format!("{operation}: invalid payload"))
}

#[cfg(test)]
#[path = "../tests/unit/fetch/tests.rs"]
mod tests;
