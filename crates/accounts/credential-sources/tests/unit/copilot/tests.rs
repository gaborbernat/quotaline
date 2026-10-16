//! Checks Copilot file discovery and fallback policy.

use super::*;
use std::fs;

fn discovery_parts(discovery: CopilotDiscovery) -> (Option<CopilotFound>, Option<CopilotPending>) {
    match discovery {
        CopilotDiscovery::Found(found) => (Some(found), None),
        CopilotDiscovery::Pending(pending) => (None, Some(pending)),
    }
}

#[test]
fn cli_reads_accounts_and_first_plaintext_credential() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.json");
    fs::write(&path, br#"{"loggedInUsers":[{"user":"ada","keychainAccount":"shared","token":"first"},{"user":"bob","keychainAccount":"shared","token":"second"},{"token":"anonymous"}]}"#).unwrap();
    let result = read_copilot_cli(&path).unwrap();
    assert_eq!(result.accounts, ["shared"]);
    assert_eq!(result.credential.unwrap().token, "first");
}

#[test]
fn cli_distinguishes_missing_and_invalid_files() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.json");
    assert_eq!(read_copilot_cli(&path).unwrap(), CopilotCli::default());
    fs::write(&path, b"{").unwrap();
    assert!(matches!(read_copilot_cli(&path), Err(CopilotSourceError::Json { .. })));
    fs::write(&path, b"[]").unwrap();
    assert_eq!(read_copilot_cli(&path).unwrap(), CopilotCli::default());
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(matches!(read_copilot_cli(&path), Err(CopilotSourceError::File { .. })));
}

#[test]
fn legacy_prefers_first_usable_file() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("hosts.json");
    let second = root.path().join("apps.json");
    fs::write(&first, br#"{"github.com":{"oauth_token":"first"}}"#).unwrap();
    fs::write(&second, br#"{"github.com":{"oauth_token":"second"}}"#).unwrap();
    let result = discover_copilot_legacy(&[first.clone(), second], CopilotLegacyRecovery::Stop)
        .unwrap()
        .unwrap();
    assert_eq!(result.path, first);
    assert_eq!(result.credential.token, "first");
}

#[test]
fn legacy_recovery_matches_platform_error_policy() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("hosts.json");
    let second = root.path().join("apps.json");
    fs::write(&first, b"{").unwrap();
    fs::write(&second, br#"{"github.com":{"oauth_token":"fallback"}}"#).unwrap();
    let error = discover_copilot_legacy(&[first.clone(), second.clone()], CopilotLegacyRecovery::Stop).unwrap_err();
    assert!(matches!(error, CopilotSourceError::Json { .. }));
    assert!(error.to_string().contains(first.to_string_lossy().as_ref()));
    assert_eq!(
        discover_copilot_legacy(&[first.clone(), second], CopilotLegacyRecovery::TryNext)
            .unwrap()
            .unwrap()
            .credential
            .token,
        "fallback"
    );
    let error = discover_copilot_legacy(std::slice::from_ref(&first), CopilotLegacyRecovery::TryNext).unwrap_err();
    assert!(matches!(error, CopilotSourceError::Json { .. }));
    assert!(error.to_string().contains(first.to_string_lossy().as_ref()));
}

#[test]
fn legacy_handles_absent_and_nonobject_files() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("hosts.json");
    assert!(
        discover_copilot_legacy(std::slice::from_ref(&path), CopilotLegacyRecovery::Stop)
            .unwrap()
            .is_none()
    );
    fs::write(&path, b"[]").unwrap();
    assert!(matches!(
        discover_copilot_legacy(std::slice::from_ref(&path), CopilotLegacyRecovery::Stop),
        Err(CopilotSourceError::LegacyShape { .. })
    ));
    assert!(
        discover_copilot_legacy(std::slice::from_ref(&path), CopilotLegacyRecovery::TryNext)
            .unwrap()
            .is_none()
    );
    fs::write(&path, br#"{"github.com":{"user":"ada"}}"#).unwrap();
    assert!(
        discover_copilot_legacy(&[path], CopilotLegacyRecovery::Stop)
            .unwrap()
            .is_none()
    );
}

#[test]
fn legacy_reports_read_errors() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("hosts.json");
    fs::create_dir(&path).unwrap();
    for recovery in [CopilotLegacyRecovery::Stop, CopilotLegacyRecovery::TryNext] {
        assert!(matches!(
            discover_copilot_legacy(std::slice::from_ref(&path), recovery),
            Err(CopilotSourceError::File { .. })
        ));
    }
}

#[test]
fn legacy_retains_the_first_of_multiple_read_errors() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("hosts.json");
    let second = root.path().join("apps.json");
    fs::write(&first, b"{").unwrap();
    fs::create_dir(&second).unwrap();
    let error = discover_copilot_legacy(&[first.clone(), second], CopilotLegacyRecovery::TryNext).unwrap_err();
    assert!(matches!(error, CopilotSourceError::Json { .. }));
    assert!(error.to_string().contains(first.to_string_lossy().as_ref()));
}

#[test]
fn discovery_environment_skips_damaged_files() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.json");
    fs::write(&path, b"{").unwrap();
    let credential = CopilotCredential {
        token: "environment".into(),
        user: None,
        host: "github.com".into(),
    };
    let (found, pending) = discovery_parts(begin_copilot_discovery(Some(credential.clone()), &path));
    assert!(pending.is_none());
    let found = found.expect("environment credential should finish discovery");
    assert_eq!(
        found,
        CopilotFound {
            credential,
            origin: CopilotOrigin::Environment
        }
    );
}

#[test]
fn discovery_exposes_cli_accounts_to_the_platform_keyring() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.json");
    fs::write(&path, br#"{"loggedInUsers":[{"user":"ada","token":"cli"}]}"#).unwrap();
    let (found, pending) = discovery_parts(begin_copilot_discovery(None, &path));
    assert!(found.is_none());
    let pending = pending.expect("missing environment credential should query the keyring");
    assert_eq!(pending.keyring_accounts(), ["https://github.com:ada"]);
}

#[test]
fn discovery_prefers_keyring_credential_to_cli_token() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.json");
    fs::write(&path, br#"{"loggedInUsers":[{"user":"ada","token":"cli"}]}"#).unwrap();
    let pending = discovery_parts(begin_copilot_discovery(None, &path))
        .1
        .expect("missing environment credential should query the keyring");
    let credential = CopilotCredential {
        token: "keyring".into(),
        user: Some("ada".into()),
        host: "github.com".into(),
    };
    assert_eq!(
        pending
            .finish(
                Ok::<_, String>(Some(credential.clone())),
                &[],
                CopilotLegacyRecovery::Stop
            )
            .unwrap(),
        Some(CopilotFound {
            credential,
            origin: CopilotOrigin::Keyring
        })
    );
}

#[test]
fn discovery_cli_credential_recovers_from_keyring_failure() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.json");
    fs::write(&path, br#"{"loggedInUsers":[{"token":"cli"}]}"#).unwrap();
    let pending = discovery_parts(begin_copilot_discovery(None, &path))
        .1
        .expect("missing environment credential should query the keyring");
    let found = pending
        .finish(Err("keyring"), &[], CopilotLegacyRecovery::Stop)
        .unwrap()
        .unwrap();
    assert_eq!(found.credential.token, "cli");
    assert_eq!(found.origin, CopilotOrigin::Cli(path));
}

#[test]
fn discovery_legacy_credential_recovers_from_earlier_failures() {
    let root = tempfile::tempdir().unwrap();
    let cli = root.path().join("config.json");
    let legacy = root.path().join("hosts.json");
    fs::write(&cli, b"{").unwrap();
    fs::write(&legacy, br#"{"github.com":{"oauth_token":"legacy"}}"#).unwrap();
    let pending = discovery_parts(begin_copilot_discovery(None, &cli))
        .1
        .expect("missing environment credential should query the keyring");
    let found = pending
        .finish(
            Err("keyring"),
            std::slice::from_ref(&legacy),
            CopilotLegacyRecovery::Stop,
        )
        .unwrap()
        .unwrap();
    assert_eq!(found.credential.token, "legacy");
    assert_eq!(found.origin, CopilotOrigin::Legacy(legacy));
}

#[test]
fn discovery_error_priority_follows_keyring_then_cli_then_legacy() {
    for (keyring_failure, invalid_cli, expected) in
        [(true, true, "keyring"), (false, true, "cli"), (false, false, "legacy")]
    {
        let root = tempfile::tempdir().unwrap();
        let cli = root.path().join("config.json");
        let legacy = root.path().join("hosts.json");
        if invalid_cli {
            fs::write(&cli, b"{").unwrap();
        }
        fs::write(&legacy, b"{").unwrap();
        let pending = discovery_parts(begin_copilot_discovery(None, &cli))
            .1
            .expect("missing environment credential should query the keyring");
        let keyring = if keyring_failure { Err("keyring") } else { Ok(None) };
        let error = pending
            .finish(keyring, std::slice::from_ref(&legacy), CopilotLegacyRecovery::Stop)
            .unwrap_err();
        match expected {
            "keyring" => assert!(matches!(error, CopilotDiscoveryError::Keyring("keyring"))),
            "cli" => {
                assert!(matches!(
                    &error,
                    CopilotDiscoveryError::Source(CopilotSourceError::Json { .. })
                ));
                assert!(error.to_string().contains(cli.to_string_lossy().as_ref()));
            }
            _ => {
                assert!(matches!(
                    &error,
                    CopilotDiscoveryError::Source(CopilotSourceError::Json { .. })
                ));
                assert!(error.to_string().contains(legacy.to_string_lossy().as_ref()));
            }
        }
    }
}

#[test]
fn discovery_without_credentials_or_errors_returns_none() {
    let root = tempfile::tempdir().unwrap();
    let pending = discovery_parts(begin_copilot_discovery(None, &root.path().join("missing")))
        .1
        .expect("missing environment credential should query the keyring");
    assert!(
        pending
            .finish(Ok::<_, String>(None), &[], CopilotLegacyRecovery::Stop)
            .unwrap()
            .is_none()
    );
}
