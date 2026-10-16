use super::*;
use serde_json::json;

#[test]
fn completes_all_provider_refreshes() {
    for (provider, document, expected) in [
        (
            RefreshProvider::Claude,
            json!({"claudeAiOauth":{"accessToken":"old"}}),
            json!({"claudeAiOauth":{"accessToken":"new","expiresAt":61000}}),
        ),
        (
            RefreshProvider::Codex,
            json!({"tokens":{"access_token":"old"}}),
            json!({"tokens":{"access_token":"new"},"last_refresh":"now"}),
        ),
        (
            RefreshProvider::Gemini,
            json!({"access_token":"old"}),
            json!({"access_token":"new","expiry_date":61000}),
        ),
        (
            RefreshProvider::Antigravity,
            json!({"access_token":"old"}),
            json!({"access_token":"new","expiry":61}),
        ),
    ] {
        assert_eq!(
            complete_refresh(
                provider,
                &document,
                &json!({"access_token":"new","expires_in":60}),
                1000,
                "now"
            ),
            Ok(expected)
        );
    }
}

#[test]
fn reports_provider_response_failures() {
    for (provider, response, expected) in [
        (
            RefreshProvider::Claude,
            json!({}),
            RefreshCompletionError::MissingAccessToken(None),
        ),
        (
            RefreshProvider::Claude,
            json!({"access_token":"new","expires_in":0}),
            RefreshCompletionError::InvalidExpiry,
        ),
        (
            RefreshProvider::Codex,
            json!([]),
            RefreshCompletionError::MalformedResponse,
        ),
        (
            RefreshProvider::Codex,
            json!({"error":"invalid_grant"}),
            RefreshCompletionError::MissingAccessToken(Some("invalid_grant".into())),
        ),
        (
            RefreshProvider::Gemini,
            json!([]),
            RefreshCompletionError::MalformedResponse,
        ),
        (
            RefreshProvider::Gemini,
            json!({"access_token":"new","expires_in":0}),
            RefreshCompletionError::InvalidExpiry,
        ),
        (
            RefreshProvider::Gemini,
            json!({"error":"invalid_grant","error_description":"expired"}),
            RefreshCompletionError::MissingAccessToken(Some("expired".into())),
        ),
        (
            RefreshProvider::Antigravity,
            json!({"error":"invalid_grant"}),
            RefreshCompletionError::MissingAccessToken(Some("invalid_grant".into())),
        ),
    ] {
        assert_eq!(
            complete_refresh(provider, &json!({}), &response, 0, "now"),
            Err(expected)
        );
    }
}

#[test]
fn rejects_credentials_that_cannot_be_updated() {
    for provider in [
        RefreshProvider::Claude,
        RefreshProvider::Codex,
        RefreshProvider::Gemini,
        RefreshProvider::Antigravity,
    ] {
        assert_eq!(
            complete_refresh(provider, &json!([]), &json!({"access_token":"new"}), 0, "now"),
            Err(RefreshCompletionError::InvalidCredential)
        );
    }
}

#[test]
fn describes_completion_errors() {
    for (provider, expected) in [
        (RefreshProvider::Claude, "Claude"),
        (RefreshProvider::Codex, "Codex"),
        (RefreshProvider::Gemini, "Gemini"),
        (RefreshProvider::Antigravity, "Antigravity"),
    ] {
        assert_eq!(provider.name(), expected);
    }
    for (error, expected) in [
        (
            RefreshCompletionError::MalformedResponse,
            "Claude refresh returned an invalid payload",
        ),
        (
            RefreshCompletionError::MissingAccessToken(None),
            "Claude refresh returned no access token",
        ),
        (
            RefreshCompletionError::MissingAccessToken(Some("denied".into())),
            "denied",
        ),
        (
            RefreshCompletionError::InvalidExpiry,
            "Claude refresh returned an invalid expiry",
        ),
        (
            RefreshCompletionError::InvalidCredential,
            "Claude credentials cannot be updated",
        ),
    ] {
        assert_eq!(error.message(RefreshProvider::Claude), expected);
    }
}
