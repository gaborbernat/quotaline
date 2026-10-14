//! Checks macOS Copilot source precedence through the public discovery API.

use quotaline_macos_platform::copilot::*;
use quotaline_macos_platform::keychain::KeychainValue;
use serde_json::json;
use std::collections::HashMap;
use std::fs;

#[path = "support/keychain.rs"]
mod keychain_mock;
use keychain_mock::MockCalls;

#[test]
fn environment_skips_keychain_and_files() {
    let keychain = MockCalls::new();
    let found = discover(
        &HashMap::from([("COPILOT_GITHUB_TOKEN".into(), "environment-token".into())]),
        std::path::Path::new("missing"),
        &[],
        &keychain,
    )
    .unwrap()
    .unwrap();
    assert_eq!(found.origin, Origin::Environment);
    assert_eq!(found.credential.token, "environment-token");
}

#[test]
fn keychain_follows_cli_account_order_before_file_token() {
    let directory = tempfile::tempdir().unwrap();
    let cli = directory.path().join("config.json");
    fs::write(
        &cli,
        json!({"loggedInUsers":[
            {"user":"ada","host":"github.com","token":"file-token"},
            {"user":"bob","host":"github.com"}
        ]})
        .to_string(),
    )
    .unwrap();
    let mut keychain = MockCalls::new();
    keychain.expect_load().times(2).returning(|service, account| {
        assert_eq!(service, "copilot-cli");
        Ok(account.and_then(|account| {
            (account == "https://github.com:bob").then(|| KeychainValue {
                data: b"keychain-token".to_vec(),
                account: Some(account),
            })
        }))
    });
    let found = discover(&HashMap::new(), &cli, &[], &keychain).unwrap().unwrap();
    assert_eq!(found.origin, Origin::Keychain);
    assert_eq!(found.credential.token, "keychain-token");
    assert_eq!(found.credential.user.as_deref(), Some("bob"));
}

#[test]
fn cli_token_recovers_from_keychain_error() {
    let directory = tempfile::tempdir().unwrap();
    let cli = directory.path().join("config.json");
    fs::write(&cli, json!({"loggedInUsers":[{"token":"file-token"}]}).to_string()).unwrap();
    let mut keychain = MockCalls::new();
    keychain.expect_load().returning(|_, _| Err(-25293));
    let found = discover(&HashMap::new(), &cli, &[], &keychain).unwrap().unwrap();
    assert_eq!(found.origin, Origin::Cli);
    assert_eq!(found.credential.token, "file-token");
}

#[test]
fn legacy_token_recovers_from_damaged_cli_and_keychain() {
    let directory = tempfile::tempdir().unwrap();
    let cli = directory.path().join("config.json");
    let legacy = directory.path().join("hosts.json");
    fs::write(&cli, "{").unwrap();
    fs::write(
        &legacy,
        json!({"github.com":{"oauth_token":"legacy-token"}}).to_string(),
    )
    .unwrap();
    let mut keychain = MockCalls::new();
    keychain.expect_load().returning(|_, _| Err(-25293));
    let found = discover(&HashMap::new(), &cli, std::slice::from_ref(&legacy), &keychain)
        .unwrap()
        .unwrap();
    assert_eq!(found.origin, Origin::Legacy);
    assert_eq!(found.credential.token, "legacy-token");
}

#[test]
fn keychain_error_precedes_file_errors_without_usable_credential() {
    let directory = tempfile::tempdir().unwrap();
    let cli = directory.path().join("config.json");
    fs::write(&cli, "{").unwrap();
    let mut keychain = MockCalls::new();
    keychain.expect_load().returning(|_, _| Err(-25293));
    let error = discover(&HashMap::new(), &cli, &[], &keychain).unwrap_err();
    assert!(matches!(error, DiscoveryError::Keychain(-25293)));
}

#[test]
fn keychain_malformed_error_survives_empty_fallbacks() {
    let directory = tempfile::tempdir().unwrap();
    let cli = directory.path().join("config.json");
    let mut keychain = MockCalls::new();
    keychain.expect_load().returning(|_, _| {
        Ok(Some(KeychainValue {
            data: b"{".to_vec(),
            account: None,
        }))
    });
    let error = discover(&HashMap::new(), &cli, &[], &keychain).unwrap_err();
    assert!(matches!(error, DiscoveryError::KeychainMalformed));
}

#[test]
fn missing_sources_return_no_credential() {
    let directory = tempfile::tempdir().unwrap();
    let mut keychain = MockCalls::new();
    keychain.expect_load().returning(|_, _| Ok(None));
    let found = discover(&HashMap::new(), &directory.path().join("missing"), &[], &keychain).unwrap();
    assert!(found.is_none());
}

#[test]
fn empty_keychain_item_has_no_credential() {
    let directory = tempfile::tempdir().unwrap();
    let mut keychain = MockCalls::new();
    keychain.expect_load().returning(|_, _| {
        Ok(Some(KeychainValue {
            data: Vec::new(),
            account: None,
        }))
    });
    assert!(
        discover(&HashMap::new(), &directory.path().join("missing"), &[], &keychain)
            .unwrap()
            .is_none()
    );
}

#[test]
fn cli_error_survives_missing_legacy_fallback() {
    let directory = tempfile::tempdir().unwrap();
    let cli = directory.path().join("config.json");
    fs::write(&cli, "{").unwrap();
    let mut keychain = MockCalls::new();
    keychain.expect_load().returning(|_, _| Ok(None));
    let error = discover(&HashMap::new(), &cli, &[], &keychain).unwrap_err();
    assert!(
        error
            .to_string()
            .starts_with(&format!("{} is not valid JSON:", cli.display()))
    );
}

#[test]
fn legacy_error_is_reported_when_other_sources_are_missing() {
    let directory = tempfile::tempdir().unwrap();
    let cli = directory.path().join("config.json");
    let legacy = directory.path().join("hosts.json");
    fs::write(&legacy, "[]").unwrap();
    let mut keychain = MockCalls::new();
    keychain.expect_load().returning(|_, _| Ok(None));
    let error = discover(&HashMap::new(), &cli, std::slice::from_ref(&legacy), &keychain).unwrap_err();
    assert_eq!(error.to_string(), format!("{} is not a JSON object", legacy.display()));
}

#[test]
fn cli_error_precedes_legacy_error() {
    let directory = tempfile::tempdir().unwrap();
    let cli = directory.path().join("config.json");
    let legacy = directory.path().join("hosts.json");
    fs::write(&cli, "{").unwrap();
    fs::write(&legacy, "[]").unwrap();
    let mut keychain = MockCalls::new();
    keychain.expect_load().returning(|_, _| Ok(None));
    let error = discover(&HashMap::new(), &cli, &[legacy], &keychain).unwrap_err();
    assert!(
        error
            .to_string()
            .starts_with(&format!("{} is not valid JSON:", cli.display()))
    );
}
