use super::*;

#[test]
fn oauth_endpoints_match_the_provider_contract() {
    assert_eq!(USAGE_URL, "https://api.anthropic.com/api/oauth/usage");
    assert_eq!(PROFILE_URL, "https://api.anthropic.com/api/oauth/profile");
    assert_eq!(TOKEN_URL, "https://platform.claude.com/v1/oauth/token");
}

#[test]
fn oauth_headers_include_the_current_beta_and_client_version() {
    assert_eq!(
        headers("secret"),
        [
            ("Authorization", "Bearer secret".into()),
            ("anthropic-beta", "oauth-2025-04-20".into()),
            ("User-Agent", "claude-code/2.1.251".into()),
        ]
    );
}
