//! Fetches Claude usage and the OAuth profile through a desktop's transport.

use serde_json::Value;

use super::{profile_valid, request};
use quotaline_http::HttpError;
use quotaline_http::transport::{Request, Transport, header_list, json_document};

pub async fn fetch_usage(transport: &impl Transport, url: &str, token: &str) -> Result<Value, HttpError> {
    let payload = transport
        .send(Request::get(url, header_list(request::headers(token)), "claude.usage"))
        .await?;
    json_document(&payload, "claude.usage")
}

/// Returns the profile, or the warning to show while the account details are unavailable.
pub async fn fetch_profile(
    transport: &impl Transport,
    url: &str,
    token: &str,
    home: Option<&str>,
) -> Result<Value, String> {
    let payload = transport
        .send(Request::get(
            url,
            header_list(request::headers(token)),
            "claude.profile",
        ))
        .await
        .map_err(|error| error.message(home))?;
    serde_json::from_slice(&payload)
        .ok()
        .filter(profile_valid)
        .ok_or_else(|| "Provider returned an invalid profile".into())
}

#[cfg(test)]
#[path = "../tests/unit/fetch/tests.rs"]
mod tests;
