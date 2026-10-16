//! macOS credential bridge cases.

use super::*;
use base64::Engine;

#[test]
fn jwt_bridge_returns_decoded_claims_and_expiry() {
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"exp":1700000000}"#);
    let token = format!("header.{payload}.signature");
    assert_eq!(jwt_claims_json(token.clone()), Some(br#"{"exp":1700000000}"#.to_vec()));
    assert_eq!(jwt_expiry(token), Some(1_700_000_000.0));
    assert_eq!(jwt_claims_json("bad".into()), None);
    assert_eq!(jwt_expiry("bad".into()), None);
}

#[test]
fn cursor_and_expiry_bridge_use_shared_policy() {
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"sub":"auth0|user"}"#);
    let token = format!("header.{payload}.signature");
    assert_eq!(cursor_user_id(token.clone()).as_deref(), Some("user"));
    assert_eq!(
        cursor_session_cookie(token.clone()),
        format!("WorkosCursorSessionToken=user%3A%3A{token}")
    );
    assert_eq!(cursor_user_id("opaque".into()), None);
    assert_eq!(credential_expiry_buffer(), 120.0);
    assert!(!credential_expired(Some(1120.0), 1000.0));
    assert!(credential_expired(Some(1119.0), 1000.0));
}

#[test]
fn codex_bridge_uses_shared_credential_policy() {
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        br#"{"https://api.openai.com/auth":{"chatgpt_subscription_active_until":"2026-11-04T23:59:00-08:00"}}"#,
    );
    let token = format!("header.{payload}.signature");
    assert_eq!(
        codex_account_fingerprint("token".into(), Some("account".into()), Some(token.clone())),
        credentials::codex_account_fingerprint("token", Some("account"), Some(&token))
    );
}

#[test]
fn copilot_bridge_rejects_unsafe_hosts() {
    assert_eq!(
        copilot_host("HTTPS://GHE.Example.com/".into()).as_deref(),
        Some("ghe.example.com")
    );
    assert_eq!(copilot_host("ghe.example.com/path".into()), None);
}

#[test]
fn codex_home_bridge_keeps_the_existing_key_format() {
    assert_eq!(codex_home_key("/missing-codex-home".into()), "cli|827d2572d655e306");
}

#[test]
fn cache_fingerprint_bridges_keep_provider_identities_separate() {
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"sub":"subject"}"#);
    let token = format!("header.{payload}.signature");
    assert_eq!(
        claude_cache_fingerprint(token.clone(), Some("refresh".into())),
        claude_cache_fingerprint(token.clone(), None)
    );
    assert_eq!(
        gemini_cache_fingerprint("access".into(), Some("refresh".into()), Some(token.clone())),
        gemini_cache_fingerprint("other".into(), None, Some(token.clone()))
    );
    assert_ne!(
        claude_cache_fingerprint(token.clone(), None),
        gemini_cache_fingerprint("access".into(), None, Some(token))
    );
}

#[test]
fn claude_keychain_bridge_scopes_nondefault_directory() {
    assert_eq!(claude_keychain_service(None), "Claude Code-credentials");
    assert_eq!(
        claude_keychain_service(Some("/tmp/claude".into())),
        "Claude Code-credentials-21493821"
    );
}

#[test]
fn antigravity_bridge_decodes_keyring_and_optional_document() {
    let document = br#"{"token":{"access_token":"access","refresh_token":"refresh","expiry":1788033600}}"#;
    let expected = AntigravityCredential {
        access_token: "access".into(),
        refresh_token: Some("refresh".into()),
        expires_at: Some(1_788_033_600.0),
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(document);
    assert_eq!(
        antigravity_keyring_credential(format!("go-keyring-base64:{encoded}").into_bytes()),
        AntigravityKeyringResult {
            credential: Some(expected),
            problem: None,
        }
    );
    assert_eq!(
        antigravity_keyring_credential(br#"{"token":{}}"#.to_vec()),
        AntigravityKeyringResult {
            credential: None,
            problem: None,
        }
    );
    assert_eq!(
        antigravity_keyring_credential(b"go-keyring-base64:!".to_vec()),
        AntigravityKeyringResult {
            credential: None,
            problem: Some(AntigravityCredentialProblem::InvalidBase64),
        }
    );
    assert_eq!(
        antigravity_keyring_credential(b"bad".to_vec()),
        AntigravityKeyringResult {
            credential: None,
            problem: Some(AntigravityCredentialProblem::InvalidJson),
        }
    );
}
