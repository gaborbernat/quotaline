//! Fetches the Copilot user document through a desktop's transport.

use serde_json::Value;

use super::request;
use quotaline_http::HttpError;
use quotaline_http::transport::{Request, Transport, header_list, json_document};

pub async fn fetch_user(transport: &impl Transport, url: &str, token: &str) -> Result<Value, HttpError> {
    let payload = transport
        .send(Request::get(url, header_list(request::headers(token)), "copilot.user"))
        .await?;
    json_document(&payload, "copilot.user")
}

#[cfg(test)]
#[path = "../tests/unit/fetch/tests.rs"]
mod tests;
