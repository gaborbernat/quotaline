use super::*;

#[test]
fn endpoints_target_local_language_server_and_cloud_fallbacks() {
    assert_eq!(
        server_status_url("https", 1234),
        "https://127.0.0.1:1234/exa.language_server_pb.LanguageServerService/GetUserStatus"
    );
    assert_eq!(
        server_quota_url("http", 4321),
        "http://127.0.0.1:4321/exa.language_server_pb.LanguageServerService/RetrieveUserQuotaSummary"
    );
    assert_eq!(
        cloud_assist_url(CLOUD_BASES[0]),
        "https://daily-cloudcode-pa.googleapis.com/v1internal:loadCodeAssist"
    );
    assert_eq!(
        cloud_quota_url(CLOUD_BASES[1]),
        "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary"
    );
    assert_eq!(CLOUD_BASES[1], "https://cloudcode-pa.googleapis.com");
}

#[test]
fn local_headers_include_csrf_only_when_supplied() {
    assert_eq!(server_headers(None), [("Connect-Protocol-Version", "1".into())]);
    assert_eq!(
        server_headers(Some("secret")),
        [
            ("Connect-Protocol-Version", "1".into()),
            ("X-Codeium-Csrf-Token", "secret".into()),
        ]
    );
}

#[test]
fn cloud_headers_require_the_selected_token() {
    assert_eq!(
        cloud_headers("secret"),
        [
            ("Authorization", "Bearer secret".into()),
            ("Accept", "application/json".into()),
            ("User-Agent", "antigravity".into()),
        ]
    );
}

#[test]
fn bodies_match_language_server_and_cloud_contracts() {
    assert_eq!(server_quota_body(), json!({"forceRefresh":true}));
    assert_eq!(
        server_status_body(),
        json!({"metadata":{"ideName":"antigravity","extensionName":"antigravity","ideVersion":"unknown","locale":"en"}})
    );
    assert_eq!(cloud_quota_body(), json!({}));
    assert_eq!(
        cloud_assist_body(),
        json!({"metadata":{"ideType":"ANTIGRAVITY","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}})
    );
}
