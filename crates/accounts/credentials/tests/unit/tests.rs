//! Credential source and fingerprint cases.

use super::*;

fn token(claims: &str) -> String {
    format!(
        "header.{}.signature",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims)
    )
}

#[test]
fn jwt_claims_reject_malformed_segments_and_payloads() {
    assert_eq!(jwt_claims("opaque"), None);
    assert_eq!(jwt_claims("header.payload."), None);
    assert_eq!(jwt_claims("header.payload.signature.extra"), None);
    assert_eq!(jwt_claims("header.!.signature"), None);
    assert_eq!(jwt_claims(&token("invalid json")), None);
}

#[test]
fn jwt_claims_accept_padded_and_unpadded_payloads() {
    let claims = r#"{"exp":1700000000.5}"#;
    assert_eq!(jwt_expiry(&token(claims)), Some(1_700_000_000.5));
    let padded = base64::engine::general_purpose::URL_SAFE.encode(claims);
    assert_eq!(jwt_expiry(&format!("header.{padded}.signature")), Some(1_700_000_000.5));
    assert_eq!(jwt_expiry(&token("{}")), None);
}

#[test]
fn cursor_cookie_uses_the_user_id_from_a_valid_subject() {
    let access_token = token(r#"{"sub":"auth0|user_01ABC"}"#);
    assert_eq!(cursor_user_id(&access_token).as_deref(), Some("user_01ABC"));
    assert_eq!(
        cursor_session_cookie(&access_token),
        format!("WorkosCursorSessionToken=user_01ABC%3A%3A{access_token}")
    );
    assert_eq!(cursor_user_id(&token(r#"{"sub":"user"}"#)).as_deref(), Some("user"));
    assert_eq!(cursor_user_id(&token(r#"{"sub":"auth0|"}"#)).as_deref(), Some("auth0"));
    assert_eq!(cursor_user_id(&token(r#"{"sub":""}"#)), None);
    assert_eq!(cursor_session_cookie("opaque"), "WorkosCursorSessionToken=%3A%3Aopaque");
}

#[test]
fn credential_expiry_uses_the_shared_buffer() {
    assert_eq!(CREDENTIAL_EXPIRY_BUFFER, 120.0);
    assert!(!credential_expired(None, 1000.0));
    assert!(!credential_expired(Some(1120.0), 1000.0));
    assert!(credential_expired(Some(1119.0), 1000.0));
}

#[test]
fn codex_subscription_requires_valid_date() {
    let claims = r#"{"https://api.openai.com/auth":{"chatgpt_subscription_active_until":"2026-11-04T23:59:00-08:00"}}"#;
    assert_eq!(
        codex_subscription_until(&token(claims)).as_deref(),
        Some("2026-11-04T23:59:00-08:00")
    );
    assert_eq!(codex_subscription_until("bad"), None);
    assert_eq!(codex_subscription_until(&token("{}")), None);
    assert_eq!(
        codex_subscription_until(&token(
            r#"{"https://api.openai.com/auth":{"chatgpt_subscription_active_until":"bad"}}"#
        )),
        None
    );
}

#[test]
fn codex_fingerprint_prefers_account_then_email_then_token() {
    let claims =
        token(r#"{"email":"  User@Example.COM  ","https://api.openai.com/auth":{"chatgpt_account_id":" claim "}}"#);
    let account = codex_account_fingerprint("token", Some(" provided "), Some(&claims));
    assert_eq!(account, codex_account_fingerprint("other", Some("provided"), None));
    assert_eq!(
        codex_account_fingerprint("token", None, Some(&claims)),
        codex_account_fingerprint("other", Some("claim"), None)
    );
    assert_eq!(
        codex_account_fingerprint("token", Some(" "), Some(&claims)),
        codex_account_fingerprint("other", Some("claim"), None)
    );
    let email = token(r#"{"email":"  User@Example.COM  "}"#);
    assert_eq!(
        codex_account_fingerprint("token", None, Some(&email)),
        codex_account_fingerprint("other", None, Some(&token(r#"{"email":"user@example.com"}"#)))
    );
    assert_ne!(
        codex_account_fingerprint("token", None, None),
        codex_account_fingerprint("other", None, None)
    );
    assert_eq!(
        codex_account_fingerprint("token", None, Some(&token(r#"{"email":" "}"#))),
        codex_account_fingerprint("token", None, None)
    );
}

#[test]
fn copilot_host_accepts_only_hostname_characters() {
    assert_eq!(
        copilot_host(" HTTPS://GHE.Example.com/ ").as_deref(),
        Some("ghe.example.com")
    );
    assert_eq!(copilot_host("http://github.com").as_deref(), Some("github.com"));
    assert_eq!(copilot_host(""), None);
    assert_eq!(copilot_host("https://host/path"), None);
    assert_eq!(copilot_host("host:443"), None);
    assert_eq!(copilot_host("höst.example"), None);
}

#[test]
fn codex_home_key_uses_the_canonical_path_bytes() {
    assert_eq!(codex_home_key("/missing-codex-home"), "cli|827d2572d655e306");
    assert_ne!(codex_home_key("/one"), codex_home_key("/two"));
}

#[test]
fn claude_and_gemini_cache_keys_follow_claim_identity() {
    let claims = token(r#"{"sub":"subject","email":"user@example.com"}"#);
    let email = token(r#"{"email":"user@example.com"}"#);
    let subject = claude_cache_fingerprint(&claims, Some("refresh"));
    assert_eq!(subject, claude_cache_fingerprint(&claims, None));
    assert_ne!(subject, claude_cache_fingerprint(&email, Some("refresh")));
    assert_eq!(
        claude_cache_fingerprint("opaque", Some("refresh")),
        claude_cache_fingerprint("different", Some("refresh"))
    );
    assert_ne!(
        claude_cache_fingerprint("opaque", None),
        claude_cache_fingerprint("different", None)
    );
    assert_eq!(
        gemini_cache_fingerprint("access", Some("refresh"), Some(&claims)),
        gemini_cache_fingerprint("other", None, Some(&claims))
    );
    assert_eq!(
        gemini_cache_fingerprint("access", Some("refresh"), None),
        gemini_cache_fingerprint("other", Some("refresh"), None)
    );
    assert_ne!(
        gemini_cache_fingerprint("access", None, None),
        gemini_cache_fingerprint("other", None, None)
    );
}

#[test]
fn claude_keychain_service_scopes_nonempty_config_directory() {
    assert_eq!(claude_keychain_service(None), "Claude Code-credentials");
    assert_eq!(claude_keychain_service(Some("")), "Claude Code-credentials");
    assert_eq!(
        claude_keychain_service(Some("/tmp/claude")),
        "Claude Code-credentials-21493821"
    );
}

#[test]
fn antigravity_keyring_decodes_prefixed_and_plain_json() {
    let document =
        br#"{"token":{"access_token":"access","refresh_token":"refresh","expiry":"2026-08-29T20:00:00.5Z"}}"#;
    let encoded = base64::engine::general_purpose::STANDARD.encode(document);
    assert_eq!(
        antigravity_keyring_document(format!("go-keyring-base64:{encoded}").as_bytes()).unwrap(),
        antigravity_keyring_document(document).unwrap()
    );
    assert_eq!(
        antigravity_credential(&antigravity_keyring_document(document).unwrap()),
        Some(AntigravityCredential {
            access_token: "access".into(),
            refresh_token: Some("refresh".into()),
            expires_at: Some(1_788_033_600.5),
        })
    );
    assert_eq!(
        antigravity_keyring_document(b"go-keyring-base64:!"),
        Err(AntigravityDecodeError::InvalidBase64)
    );
    assert_eq!(
        antigravity_keyring_document(b"bad"),
        Err(AntigravityDecodeError::InvalidJson)
    );
}

#[test]
fn antigravity_credential_accepts_root_case_and_expiry_forms() {
    for (document, expected) in [
        (
            r#"{"accessToken":"a","refreshToken":"r","expiresAt":1788033600}"#,
            Some(1_788_033_600.0),
        ),
        (
            r#"{"access_token":"a","expires_at":"1788033600"}"#,
            Some(1_788_033_600.0),
        ),
        (r#"{"access_token":"a","expiry":"bad"}"#, None),
        (r#"{"access_token":"a"}"#, None),
    ] {
        let document = serde_json::from_str(document).unwrap();
        assert_eq!(antigravity_credential(&document).unwrap().expires_at, expected);
    }
    assert_eq!(antigravity_credential(&serde_json::json!({"token":{}})), None);
    assert_eq!(antigravity_credential(&serde_json::json!({"refresh_token":"r"})), None);
}
