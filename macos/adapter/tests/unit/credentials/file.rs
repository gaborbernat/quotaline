use super::*;

#[test]
fn provider_payload_bridge_validates_each_provider() {
    for (provider, payload) in [
        (
            CredentialFileProvider::Claude,
            br#"{"claudeAiOauth":{"accessToken":"a"}}"#.as_slice(),
        ),
        (
            CredentialFileProvider::Codex,
            br#"{"tokens":{"access_token":"a"}}"#.as_slice(),
        ),
        (CredentialFileProvider::Gemini, br#"{"access_token":"a"}"#.as_slice()),
    ] {
        assert!(credential_payload_usable(provider, payload.to_vec()).unwrap());
        assert!(!credential_payload_usable(provider, br#"{}"#.to_vec()).unwrap());
        assert!(matches!(
            credential_payload_usable(provider, b"{".to_vec()),
            Err(StorageFileError::Malformed)
        ));
    }
}

#[test]
fn provider_file_bridge_returns_valid_document_and_maps_errors() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("auth.json");
    assert_eq!(
        read_provider_credential_file(CredentialFileProvider::Codex, path.to_string_lossy().into()).unwrap(),
        None
    );
    std::fs::write(&path, br#"{"tokens":{"access_token":"a"}}"#).unwrap();
    assert_eq!(
        read_provider_credential_file(CredentialFileProvider::Codex, path.to_string_lossy().into()).unwrap(),
        Some(br#"{"tokens":{"access_token":"a"}}"#.to_vec())
    );
    std::fs::write(&path, b"{").unwrap();
    assert!(matches!(
        read_provider_credential_file(CredentialFileProvider::Codex, path.to_string_lossy().into()),
        Err(StorageFileError::Malformed)
    ));
    assert!(matches!(
        read_provider_credential_file(CredentialFileProvider::Codex, directory.path().to_string_lossy().into()),
        Err(StorageFileError::Io { .. })
    ));
}
