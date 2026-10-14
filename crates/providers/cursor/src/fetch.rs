//! Fetches Cursor usage through a desktop's transport, falling back to the current-period endpoint.

use serde_json::Value;

use super::{UsageResponse, profile_email, request};
use quotaline_http::HttpError;
use quotaline_http::transport::{Request, Transport, header_list, json_document};

pub struct Endpoints<'a> {
    pub summary: &'a str,
    pub period: &'a str,
}

impl Default for Endpoints<'_> {
    fn default() -> Self {
        Self {
            summary: request::SUMMARY_URL,
            period: request::PERIOD_URL,
        }
    }
}

pub async fn fetch_usage(
    transport: &impl Transport,
    endpoints: &Endpoints<'_>,
    cookie: &str,
    token: &str,
    home: Option<&str>,
) -> Result<UsageResponse, HttpError> {
    let summary = transport
        .send(Request::get(
            endpoints.summary,
            header_list(request::cookie_headers(cookie)),
            "cursor.usage-summary",
        ))
        .await
        .and_then(|payload| json_document(&payload, "cursor.usage-summary"));
    let summary_error = match summary {
        Ok(summary) => return Ok(UsageResponse::summary(summary)),
        Err(error) => error,
    };
    let payload = transport
        .send(Request::post_json(
            endpoints.period,
            header_list(request::bearer_headers(token)),
            "cursor.period-usage",
            b"{}".to_vec(),
        ))
        .await?;
    let period = json_document(&payload, "cursor.period-usage")?;
    Ok(UsageResponse::period(&period, &summary_error.message(home)))
}

/// Looks up the signed-in email; identity is optional, so any failure reads as unknown.
pub async fn fetch_email(transport: &impl Transport, url: &str, cookie: &str) -> Option<String> {
    let payload = transport
        .send(Request::get(
            url,
            header_list(request::cookie_headers(cookie)),
            "cursor.me",
        ))
        .await
        .ok()?;
    profile_email(&serde_json::from_slice::<Value>(&payload).ok()?).map(str::to_owned)
}

#[cfg(test)]
#[path = "../tests/unit/fetch/tests.rs"]
mod tests;
