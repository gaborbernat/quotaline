use super::*;

#[test]
fn endpoint_uses_the_selected_code_assist_base() {
    assert_eq!(
        endpoint(BASE_URL, "loadCodeAssist"),
        "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist"
    );
    assert_eq!(TOKEN_URL, "https://oauth2.googleapis.com/token");
}

#[test]
fn user_agent_matches_each_frontend_platform() {
    assert_eq!(
        headers("secret", "darwin", Some("arm64")),
        [
            ("Authorization", "Bearer secret".into()),
            ("User-Agent", "GeminiCLI/quotaline (darwin; arm64; cli)".into()),
        ]
    );
    assert_eq!(
        headers("secret", "linux", None),
        [
            ("Authorization", "Bearer secret".into()),
            ("User-Agent", "GeminiCLI/quotaline (linux; cli)".into()),
        ]
    );
}

#[test]
fn request_bodies_cover_health_check_and_optional_project() {
    assert_eq!(
        assist_body(),
        json!({"mode":"HEALTH_CHECK","metadata":{"ideType":"GEMINI_CLI","pluginType":"GEMINI"}})
    );
    assert_eq!(quota_body(Some("project-id")), json!({"project":"project-id"}));
    assert_eq!(quota_body(None), json!({}));
}
