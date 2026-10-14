use super::*;

#[test]
fn quota_url_uses_the_host_specific_api() {
    for (host, expected) in [
        ("github.com", "https://api.github.com/copilot_internal/user"),
        ("company.ghe.com", "https://api.company.ghe.com/copilot_internal/user"),
        (
            "api.company.ghe.com",
            "https://api.company.ghe.com/copilot_internal/user",
        ),
        (
            "github.company.test",
            "https://github.company.test/api/v3/copilot_internal/user",
        ),
    ] {
        assert_eq!(user_url(host), expected, "{host}");
    }
}

#[test]
fn quota_headers_contain_the_selected_token_and_client_versions() {
    assert_eq!(
        headers("secret"),
        [
            ("Authorization", "token secret".into()),
            ("Editor-Version", "vscode/1.96.2".into()),
            ("Editor-Plugin-Version", "copilot-chat/0.26.7".into()),
            ("User-Agent", "GitHubCopilotChat/0.26.7".into()),
            ("X-Github-Api-Version", "2025-04-01".into()),
        ]
    );
}
