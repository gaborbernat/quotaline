use super::*;

#[test]
fn provider_payloads_require_their_own_credentials() {
    for (provider, usable) in [
        (
            CredentialProvider::Claude,
            br#"{"claudeAiOauth":{"accessToken":"claude"}}"#.as_slice(),
        ),
        (
            CredentialProvider::Codex,
            br#"{"tokens":{"access_token":"codex"}}"#.as_slice(),
        ),
        (CredentialProvider::Gemini, br#"{"access_token":"gemini"}"#.as_slice()),
    ] {
        assert!(usable_credential_payload(provider, usable).unwrap());
        assert!(!usable_credential_payload(provider, br#"{}"#).unwrap());
    }
}

#[test]
fn malformed_payload_is_distinct_from_missing_token() {
    assert!(usable_credential_payload(CredentialProvider::Claude, b"{").is_err());
    assert!(!usable_credential_payload(CredentialProvider::Claude, br#"{"mcpOAuth":{"accessToken":"mcp"}}"#).unwrap());
}

#[test]
fn credential_file_keeps_original_bytes_for_a_usable_document() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("auth.json");
    let data = br#"{ "tokens": { "access_token": "codex" } }"#;
    std::fs::write(&path, data).unwrap();
    assert_eq!(
        read_credential_file(CredentialProvider::Codex, &path).unwrap(),
        Some(data.to_vec())
    );
}

#[test]
fn credential_file_distinguishes_missing_empty_and_malformed() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("auth.json");
    assert_eq!(read_credential_file(CredentialProvider::Codex, &path).unwrap(), None);
    std::fs::write(&path, br#"{}"#).unwrap();
    assert_eq!(read_credential_file(CredentialProvider::Codex, &path).unwrap(), None);
    std::fs::write(&path, b"{").unwrap();
    assert!(matches!(
        read_credential_file(CredentialProvider::Codex, &path),
        Err(CredentialFileError::Json { .. })
    ));
}

#[test]
fn credential_file_reports_read_failure() {
    let directory = tempfile::tempdir().unwrap();
    assert!(matches!(
        read_credential_file(CredentialProvider::Gemini, directory.path()),
        Err(CredentialFileError::Io { .. })
    ));
}
