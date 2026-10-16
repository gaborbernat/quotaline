use super::*;
use serde_json::json;

#[test]
fn completes_persisted_credentials() {
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
    ] {
        let completion = complete_credential_refresh(
            provider,
            serde_json::to_vec(&document).unwrap(),
            br#"{"access_token":"new","expires_in":60}"#.to_vec(),
            1000,
            "now".into(),
        );
        assert_eq!(completion.problem, None);
        assert_eq!(
            serde_json::from_slice::<Value>(&completion.document.unwrap()).unwrap(),
            expected
        );
    }
}

#[test]
fn rejects_invalid_refresh_payloads() {
    let completion = complete_credential_refresh(
        RefreshProvider::Claude,
        b"{}".to_vec(),
        b"invalid".to_vec(),
        0,
        "".into(),
    );
    assert_eq!(completion.problem, Some(RefreshProblem::MalformedResponse));
    assert_eq!(
        completion.detail.as_deref(),
        Some("Claude refresh returned an invalid payload")
    );
    assert_eq!(completion.document, None);
    let completion = complete_credential_refresh(
        RefreshProvider::Claude,
        b"invalid".to_vec(),
        b"{}".to_vec(),
        0,
        "".into(),
    );
    assert_eq!(completion.problem, Some(RefreshProblem::InvalidCredential));
    let completion = complete_credential_refresh(
        RefreshProvider::Gemini,
        b"{}".to_vec(),
        br#"{"error":"invalid_grant"}"#.to_vec(),
        0,
        "".into(),
    );
    assert_eq!(completion.problem, Some(RefreshProblem::MissingAccessToken));
    assert_eq!(completion.detail.as_deref(), Some("invalid_grant"));
    let completion = complete_credential_refresh(RefreshProvider::Codex, b"{}".to_vec(), b"{}".to_vec(), 0, "".into());
    assert_eq!(completion.detail.as_deref(), Some("refresh returned no access token"));
}

#[test]
fn maps_expiry_and_credential_errors() {
    let invalid_expiry = complete_credential_refresh(
        RefreshProvider::Gemini,
        b"{}".to_vec(),
        br#"{"access_token":"a","expires_in":0}"#.to_vec(),
        0,
        "".into(),
    );
    assert_eq!(invalid_expiry.problem, Some(RefreshProblem::InvalidExpiry));
    let invalid_credential = complete_credential_refresh(
        RefreshProvider::Codex,
        b"[]".to_vec(),
        br#"{"access_token":"a"}"#.to_vec(),
        0,
        "".into(),
    );
    assert_eq!(invalid_credential.problem, Some(RefreshProblem::InvalidCredential));
}

#[test]
fn completes_antigravity_in_memory() {
    let result = complete_antigravity_refresh(
        "old".into(),
        br#"{"access_token":"new","expires_in":60}"#.to_vec(),
        1000,
    );
    assert_eq!(result.access_token.as_deref(), Some("new"));
    assert_eq!(result.expires_at, Some(61.0));
    assert_eq!(result.problem, None);
}

#[test]
fn rejects_antigravity_invalid_response() {
    let malformed = complete_antigravity_refresh("old".into(), b"invalid".to_vec(), 0);
    assert_eq!(malformed.problem, Some(RefreshProblem::MalformedResponse));
    let missing = complete_antigravity_refresh("old".into(), br#"{"error":"invalid_grant"}"#.to_vec(), 0);
    assert_eq!(missing.problem, Some(RefreshProblem::MissingAccessToken));
    assert_eq!(missing.detail.as_deref(), Some("invalid_grant"));
}
