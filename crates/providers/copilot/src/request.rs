//! Defines the GitHub Copilot quota request for every frontend.

pub fn user_url(host: &str) -> String {
    if host == "github.com" {
        return "https://api.github.com/copilot_internal/user".into();
    }
    if host.ends_with(".ghe.com") {
        let prefix = if host.starts_with("api.") { "" } else { "api." };
        return format!("https://{prefix}{host}/copilot_internal/user");
    }
    format!("https://{host}/api/v3/copilot_internal/user")
}

pub fn headers(token: &str) -> [(&'static str, String); 5] {
    [
        ("Authorization", format!("token {token}")),
        ("Editor-Version", "vscode/1.96.2".into()),
        ("Editor-Plugin-Version", "copilot-chat/0.26.7".into()),
        ("User-Agent", "GitHubCopilotChat/0.26.7".into()),
        ("X-Github-Api-Version", "2025-04-01".into()),
    ]
}

#[cfg(test)]
#[path = "../tests/unit/request/tests.rs"]
mod tests;
