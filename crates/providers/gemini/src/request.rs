//! Defines Gemini CLI OAuth and Code Assist requests.

use serde_json::{Value, json};

pub const BASE_URL: &str = "https://cloudcode-pa.googleapis.com";
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

pub fn endpoint(base: &str, method: &str) -> String {
    format!("{base}/v1internal:{method}")
}

pub fn headers(token: &str, platform: &str, architecture: Option<&str>) -> [(&'static str, String); 2] {
    let client = architecture.map_or_else(
        || format!("GeminiCLI/quotaline ({platform}; cli)"),
        |architecture| format!("GeminiCLI/quotaline ({platform}; {architecture}; cli)"),
    );
    [("Authorization", format!("Bearer {token}")), ("User-Agent", client)]
}

pub fn assist_body() -> Value {
    json!({"mode":"HEALTH_CHECK","metadata":{"ideType":"GEMINI_CLI","pluginType":"GEMINI"}})
}

pub fn quota_body(project: Option<&str>) -> Value {
    project.map_or_else(|| json!({}), |project| json!({"project":project}))
}

#[cfg(test)]
#[path = "../tests/unit/request/tests.rs"]
mod tests;
