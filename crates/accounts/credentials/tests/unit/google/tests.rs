use super::*;

#[test]
fn refresh_request_contains_client_and_token() {
    assert_eq!(
        google_refresh_request("client", "secret", "refresh"),
        [
            ("client_id", "client".into()),
            ("client_secret", "secret".into()),
            ("grant_type", "refresh_token".into()),
            ("refresh_token", "refresh".into()),
        ]
    );
}

#[test]
fn refresh_response_reads_optional_fields_and_default_expiry() {
    assert_eq!(
        google_refresh_response(
            &json!({"access_token":"access","refresh_token":"refresh","id_token":"id","error":"error","error_description":"detail"})
        ),
        Ok(GoogleRefresh {
            access_token: Some("access".into()),
            refresh_token: Some("refresh".into()),
            id_token: Some("id".into()),
            expires_in: 3600.0,
            error: Some("error".into()),
            error_description: Some("detail".into()),
        })
    );
}

#[test]
fn refresh_response_rejects_bad_expiry_and_document() {
    for value in [json!(0), json!(-1), json!("NaN")] {
        assert_eq!(
            google_refresh_response(&json!({"access_token":"a","expires_in":value})),
            Err(GoogleRefreshError::InvalidExpiry)
        );
    }
    assert_eq!(
        google_refresh_response(&json!([])),
        Err(GoogleRefreshError::InvalidResponse)
    );
    assert_eq!(
        google_refresh_response(&json!({"expires_in":"60"})).unwrap().expires_in,
        60.0
    );
}

#[test]
fn refresh_errors_explain_the_failed_field() {
    assert_eq!(
        GoogleRefreshError::InvalidResponse.message("Gemini"),
        "Gemini refresh returned no access token"
    );
    assert_eq!(
        GoogleRefreshError::InvalidExpiry.message("Antigravity"),
        "Antigravity refresh returned an invalid expiry"
    );
}

#[test]
fn refreshed_document_updates_flat_and_wrapped_credentials() {
    let token = google_refresh_response(
        &json!({"access_token":"new","refresh_token":"rotated","id_token":"id","expires_in":60}),
    )
    .unwrap();
    assert_eq!(
        gemini_refreshed_document(&json!({"access_token":"old","refresh_token":"original"}), &token, 1000),
        Some(json!({"access_token":"new","refresh_token":"rotated","id_token":"id","expiry_date":61000}))
    );
    assert_eq!(
        gemini_refreshed_document(&json!({"token":{"accessToken":"old"},"other":1}), &token, 1000),
        Some(
            json!({"token":{"accessToken":"new","refreshToken":"rotated","idToken":"id","expiresAt":61000},"other":1})
        )
    );
}

#[test]
fn refreshed_document_preserves_existing_optional_tokens() {
    let token = google_refresh_response(&json!({"access_token":"new"})).unwrap();
    assert_eq!(
        gemini_refreshed_document(&json!({"token":{"refreshToken":"old","idToken":"id"}}), &token, 0),
        Some(json!({"token":{"accessToken":"new","refreshToken":"old","idToken":"id","expiresAt":3600000}}))
    );
}

#[test]
fn refreshed_document_requires_access_token_and_object() {
    let missing = google_refresh_response(&json!({"error":"invalid_grant"})).unwrap();
    assert_eq!(gemini_refreshed_document(&json!({}), &missing, 0), None);
    let token = google_refresh_response(&json!({"access_token":"new"})).unwrap();
    assert_eq!(gemini_refreshed_document(&json!([]), &token, 0), None);
}

#[test]
fn antigravity_document_keeps_access_token_shape_and_expiry() {
    let token = google_refresh_response(&json!({"access_token":"new","expires_in":60})).unwrap();
    assert_eq!(
        antigravity_refreshed_document(&json!({"token":{"accessToken":"old","refreshToken":"r"}}), &token, 100),
        Some(json!({"token":{"accessToken":"new","refreshToken":"r","expiry":160}}))
    );
    assert_eq!(
        antigravity_refreshed_document(&json!({"access_token":"old"}), &token, 100),
        Some(json!({"access_token":"new","expiry":160}))
    );
    assert_eq!(
        antigravity_refreshed_document(&json!({"token":{}}), &token, 100),
        Some(json!({"token":{"access_token":"new","expiry":160}}))
    );
}

#[test]
fn antigravity_document_requires_access_and_object() {
    let missing = google_refresh_response(&json!({"error":"invalid_grant"})).unwrap();
    assert_eq!(antigravity_refreshed_document(&json!({}), &missing, 0), None);
    let token = google_refresh_response(&json!({"access_token":"new"})).unwrap();
    assert_eq!(antigravity_refreshed_document(&json!([]), &token, 0), None);
}
