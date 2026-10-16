//! Defines Claude OAuth request endpoints and headers for every frontend.

use quotaline_credentials::CLAUDE_USER_AGENT;

pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
pub const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";
pub const TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
pub const BETA_HEADER: &str = "oauth-2025-04-20";

pub fn headers(token: &str) -> [(&'static str, String); 3] {
    [
        ("Authorization", format!("Bearer {token}")),
        ("anthropic-beta", BETA_HEADER.into()),
        ("User-Agent", CLAUDE_USER_AGENT.into()),
    ]
}

#[cfg(test)]
#[path = "../tests/unit/request/tests.rs"]
mod tests;
