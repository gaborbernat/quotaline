//! Defines Antigravity language server and cloud quota requests.

use serde_json::{Value, json};

pub const CLOUD_BASES: [&str; 2] = [
    "https://daily-cloudcode-pa.googleapis.com",
    "https://cloudcode-pa.googleapis.com",
];
const SERVER_SERVICE: &str = "exa.language_server_pb.LanguageServerService";

fn server_url(scheme: &str, port: u16, method: &str) -> String {
    format!("{scheme}://127.0.0.1:{port}/{SERVER_SERVICE}/{method}")
}

fn cloud_url(base: &str, method: &str) -> String {
    format!("{base}/v1internal:{method}")
}

/// The language server answers on loopback, so a slow reply means it is busy or gone rather than far away.
pub const SERVER_TIMEOUT_MILLIS: u32 = 500;
/// Newer language servers serve TLS on the same port; older ones speak plain HTTP.
pub const SERVER_SCHEMES: [&str; 2] = ["https", "http"];

pub fn server_quota_url(scheme: &str, port: u16) -> String {
    server_url(scheme, port, "RetrieveUserQuotaSummary")
}

pub fn server_status_url(scheme: &str, port: u16) -> String {
    server_url(scheme, port, "GetUserStatus")
}

pub fn cloud_quota_url(base: &str) -> String {
    cloud_url(base, "retrieveUserQuotaSummary")
}

pub fn cloud_assist_url(base: &str) -> String {
    cloud_url(base, "loadCodeAssist")
}

pub fn server_headers(csrf_token: Option<&str>) -> Vec<(&'static str, String)> {
    let mut headers = vec![("Connect-Protocol-Version", "1".into())];
    if let Some(csrf_token) = csrf_token {
        headers.push(("X-Codeium-Csrf-Token", csrf_token.into()));
    }
    headers
}

pub fn cloud_headers(token: &str) -> [(&'static str, String); 3] {
    [
        ("Authorization", format!("Bearer {token}")),
        ("Accept", "application/json".into()),
        ("User-Agent", "antigravity".into()),
    ]
}

pub fn server_quota_body() -> Value {
    json!({"forceRefresh":true})
}

pub fn server_status_body() -> Value {
    json!({"metadata":{"ideName":"antigravity","extensionName":"antigravity","ideVersion":"unknown","locale":"en"}})
}

pub fn cloud_quota_body() -> Value {
    json!({})
}

pub fn cloud_assist_body() -> Value {
    json!({"metadata":{"ideType":"ANTIGRAVITY","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}})
}

#[cfg(test)]
#[path = "../tests/unit/request/tests.rs"]
mod tests;
