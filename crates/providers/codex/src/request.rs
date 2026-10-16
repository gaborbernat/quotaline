//! Defines Codex usage and OAuth endpoints.

pub const BASE_URL: &str = "https://chatgpt.com/backend-api";
pub const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
pub const USAGE_PATH: &str = "wham/usage";
pub const RESET_CREDITS_PATH: &str = "wham/rate-limit-reset-credits";
pub const CREDIT_EVENTS_PATH: &str = "wham/usage/credit-usage-events";

pub fn endpoint(path: &str) -> String {
    format!("{BASE_URL}/{path}")
}

#[cfg(test)]
#[path = "../tests/unit/request/tests.rs"]
mod tests;
