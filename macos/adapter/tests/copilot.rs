use quotaline_macos_ffi::credentials::*;
use std::fs;
use std::{collections::HashMap, sync::Arc};

#[path = "support/keychain.rs"]
mod keychain_mock;
use keychain_mock::MockCalls;

fn keychain(lookup: KeychainLookup) -> Arc<dyn KeychainReader> {
    let mut calls = MockCalls::new();
    calls.expect_load().return_const(lookup);
    Arc::new(calls)
}

#[test]
fn ffi_discovers_environment_without_keychain() {
    let calls = Arc::new(MockCalls::new());
    let found = discover_copilot_credentials(
        HashMap::from([("GH_TOKEN".into(), "environment".into())]),
        "missing".into(),
        Vec::new(),
        calls,
    )
    .unwrap()
    .unwrap();
    assert_eq!(found.source_id, "copilot.environment");
    assert_eq!(found.credential.token, "environment");
}

#[test]
fn ffi_discovers_keychain_item() {
    let directory = tempfile::tempdir().unwrap();
    let found = discover_copilot_credentials(
        HashMap::new(),
        directory.path().join("missing").to_string_lossy().into(),
        Vec::new(),
        keychain(KeychainLookup::Item {
            data: b"secret".to_vec(),
            account: Some("https://ghe.example:ada".into()),
        }),
    )
    .unwrap()
    .unwrap();
    assert_eq!(found.source_id, "copilot.keychain");
    assert_eq!(found.credential.host, "ghe.example");
}

#[test]
fn ffi_discovers_cli_after_missing_keychain() {
    let directory = tempfile::tempdir().unwrap();
    let cli = directory.path().join("config.json");
    fs::write(&cli, r#"{"loggedInUsers":[{"token":"file"}]}"#).unwrap();
    let found = discover_copilot_credentials(
        HashMap::new(),
        cli.to_string_lossy().into(),
        Vec::new(),
        keychain(KeychainLookup::Missing),
    )
    .unwrap()
    .unwrap();
    assert_eq!(found.source_id, "copilot.file");
    assert_eq!(found.credential.token, "file");
}

#[test]
fn ffi_discovers_legacy_after_missing_keychain() {
    let directory = tempfile::tempdir().unwrap();
    let legacy = directory.path().join("hosts.json");
    fs::write(&legacy, r#"{"github.com":{"oauth_token":"legacy"}}"#).unwrap();
    let found = discover_copilot_credentials(
        HashMap::new(),
        directory.path().join("missing").to_string_lossy().into(),
        vec![legacy.to_string_lossy().into()],
        keychain(KeychainLookup::Missing),
    )
    .unwrap()
    .unwrap();
    assert_eq!(found.source_id, "copilot.legacy-file");
    assert_eq!(found.credential.token, "legacy");
}

#[test]
fn ffi_maps_keychain_status() {
    let directory = tempfile::tempdir().unwrap();
    let result = discover_copilot_credentials(
        HashMap::new(),
        directory.path().join("missing").to_string_lossy().into(),
        Vec::new(),
        keychain(KeychainLookup::Error { status: -25293 }),
    );
    assert_eq!(
        result.unwrap_err(),
        CopilotDiscoveryError::Failure {
            source_id: "copilot.keychain".into(),
            detail: "Keychain access denied; allow it in Keychain Access or sign in again.".into(),
        }
    );
}

#[test]
fn ffi_maps_unexpected_keychain_status() {
    let directory = tempfile::tempdir().unwrap();
    let error = discover_copilot_credentials(
        HashMap::new(),
        directory.path().join("missing").to_string_lossy().into(),
        Vec::new(),
        keychain(KeychainLookup::Error { status: -1 }),
    )
    .unwrap_err();
    assert_eq!(
        error,
        CopilotDiscoveryError::Failure {
            source_id: "copilot.keychain".into(),
            detail: "The Keychain item could not be read.".into(),
        }
    );
}

#[test]
fn ffi_maps_invalid_keychain_item() {
    let directory = tempfile::tempdir().unwrap();
    let result = discover_copilot_credentials(
        HashMap::new(),
        directory.path().join("missing").to_string_lossy().into(),
        Vec::new(),
        keychain(KeychainLookup::Item {
            data: vec![0xff],
            account: None,
        }),
    );
    assert_eq!(
        result.unwrap_err(),
        CopilotDiscoveryError::Failure {
            source_id: "copilot.keychain".into(),
            detail: "GitHub Copilot Keychain item is not a token".into(),
        }
    );
}

#[test]
fn ffi_maps_json_file_error() {
    let directory = tempfile::tempdir().unwrap();
    let cli = directory.path().join("config.json");
    fs::write(&cli, "{").unwrap();
    let result = discover_copilot_credentials(
        HashMap::new(),
        cli.to_string_lossy().into(),
        Vec::new(),
        keychain(KeychainLookup::Missing),
    );
    assert_eq!(
        result.unwrap_err(),
        CopilotDiscoveryError::Failure {
            source_id: "copilot.file".into(),
            detail: "config.json is not JSON".into(),
        }
    );
}

#[test]
fn ffi_maps_unreadable_file_error() {
    let directory = tempfile::tempdir().unwrap();
    let result = discover_copilot_credentials(
        HashMap::new(),
        directory.path().to_string_lossy().into(),
        Vec::new(),
        keychain(KeychainLookup::Missing),
    );
    assert_eq!(
        result.unwrap_err(),
        CopilotDiscoveryError::Failure {
            source_id: "copilot.file".into(),
            detail: "The credential file could not be read.".into(),
        }
    );
}

#[test]
fn ffi_maps_unreadable_legacy_file_to_its_source() {
    let directory = tempfile::tempdir().unwrap();
    let legacy = directory.path().join("hosts.json");
    fs::create_dir(&legacy).unwrap();
    let error = discover_copilot_credentials(
        HashMap::new(),
        directory.path().join("missing").to_string_lossy().into(),
        vec![legacy.to_string_lossy().into()],
        keychain(KeychainLookup::Missing),
    )
    .unwrap_err();
    assert_eq!(
        error,
        CopilotDiscoveryError::Failure {
            source_id: "copilot.legacy-file".into(),
            detail: "The credential file could not be read.".into(),
        }
    );
}

#[test]
fn ffi_maps_legacy_shape_error() {
    let directory = tempfile::tempdir().unwrap();
    let legacy = directory.path().join("hosts.json");
    fs::write(&legacy, "[]").unwrap();
    let result = discover_copilot_credentials(
        HashMap::new(),
        directory.path().join("missing").to_string_lossy().into(),
        vec![legacy.to_string_lossy().into()],
        keychain(KeychainLookup::Missing),
    );
    assert_eq!(
        result.unwrap_err(),
        CopilotDiscoveryError::Failure {
            source_id: "copilot.legacy-file".into(),
            detail: "hosts.json is not a JSON object".into(),
        }
    );
}

#[test]
fn ffi_maps_invalid_legacy_json_to_its_source() {
    let directory = tempfile::tempdir().unwrap();
    let legacy = directory.path().join("hosts.json");
    fs::write(&legacy, "{").unwrap();
    let error = discover_copilot_credentials(
        HashMap::new(),
        directory.path().join("missing").to_string_lossy().into(),
        vec![legacy.to_string_lossy().into()],
        keychain(KeychainLookup::Missing),
    )
    .unwrap_err();
    assert_eq!(
        error,
        CopilotDiscoveryError::Failure {
            source_id: "copilot.legacy-file".into(),
            detail: "hosts.json is not a JSON object".into(),
        }
    );
}
