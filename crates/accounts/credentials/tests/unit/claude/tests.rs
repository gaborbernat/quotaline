//! Checks Claude refresh parsing and credential preservation.

use super::*;

#[test]
fn initial_document_contains_supplied_claude_fields() {
    assert_eq!(
        claude_initial_document(
            "access",
            Some("refresh"),
            Some(1_500),
            &["user:profile".into()],
            Some("pro"),
            Some("tier")
        ),
        json!({"claudeAiOauth":{
            "accessToken":"access","refreshToken":"refresh","expiresAt":1500,
            "scopes":["user:profile"],"subscriptionType":"pro","rateLimitTier":"tier"
        }})
    );
}

#[test]
fn initial_document_omits_absent_claude_fields() {
    assert_eq!(
        claude_initial_document("access", None, None, &[], None, None),
        json!({"claudeAiOauth":{"accessToken":"access","scopes":[]}})
    );
}

#[test]
fn refresh_request_uses_the_shared_client_id() {
    assert_eq!(
        claude_refresh_request("secret"),
        json!({"grant_type":"refresh_token","refresh_token":"secret","client_id":CLAUDE_CLIENT_ID})
    );
}

#[test]
fn response_parses_rotation_and_default_expiry() {
    assert_eq!(
        claude_refresh_response(&json!({"access_token":"new","refresh_token":"rotated"})),
        Ok(ClaudeRefresh {
            access_token: "new".into(),
            refresh_token: Some("rotated".into()),
            expires_in: 3600.0
        })
    );
    assert_eq!(
        claude_refresh_response(&json!({"access_token":"new","expires_in":"120"}))
            .unwrap()
            .expires_in,
        120.0
    );
}

#[test]
fn response_rejects_missing_token_or_invalid_expiry() {
    for (response, expected) in [
        (json!({}), ClaudeRefreshError::MissingAccessToken),
        (json!({"access_token":""}), ClaudeRefreshError::MissingAccessToken),
        (
            json!({"access_token":"new","expires_in":0}),
            ClaudeRefreshError::InvalidExpiry,
        ),
        (
            json!({"access_token":"new","expires_in":-1}),
            ClaudeRefreshError::InvalidExpiry,
        ),
        (
            json!({"access_token":"new","expires_in":"NaN"}),
            ClaudeRefreshError::InvalidExpiry,
        ),
    ] {
        assert_eq!(claude_refresh_response(&response), Err(expected));
    }
}

#[test]
fn refreshed_document_preserves_other_fields_and_existing_refresh_token() {
    let token = claude_refresh_response(&json!({"access_token":"new","expires_in":1})).unwrap();
    assert_eq!(
        claude_refreshed_document(
            &json!({"other":true,"claudeAiOauth":{"accessToken":"old","refreshToken":"keep"}}),
            &token,
            1000
        ),
        Some(json!({"other":true,"claudeAiOauth":{"accessToken":"new","refreshToken":"keep","expiresAt":2000}}))
    );
}

#[test]
fn refreshed_document_rotates_token_and_saturates_expiry() {
    let token =
        claude_refresh_response(&json!({"access_token":"new","refresh_token":"rotate","expires_in":3600})).unwrap();
    assert_eq!(
        claude_refreshed_document(&json!({"claudeAiOauth":{}}), &token, i64::MAX).unwrap()["claudeAiOauth"],
        json!({"accessToken":"new","refreshToken":"rotate","expiresAt":i64::MAX})
    );
    assert!(claude_refreshed_document(&json!({}), &token, 0).is_none());
    assert!(claude_refreshed_document(&json!({"claudeAiOauth":null}), &token, 0).is_none());
}
