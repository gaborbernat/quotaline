use super::*;

#[test]
fn initial_credentials_bridge_serializes_codex_document() {
    let payload = codex_initial_credentials(
        "access".into(),
        Some("refresh".into()),
        None,
        Some("account".into()),
        Some("now".into()),
    );
    let document: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(
        document,
        serde_json::json!({
            "tokens":{"access_token":"access","refresh_token":"refresh","account_id":"account"},
            "auth_mode":"chatgpt","OPENAI_API_KEY":null,"last_refresh":"now"
        })
    );
}

#[test]
fn credential_record_projects_auth_document() {
    let record = codex_credential_record(
        br#"{"tokens":{"access_token":"access","refresh_token":"refresh","account_id":"account"},"last_refresh":"now"}"#.to_vec(),
    )
    .unwrap();
    assert_eq!(record.access_token, "access");
    assert_eq!(record.refresh_token.as_deref(), Some("refresh"));
    assert_eq!(record.account_id.as_deref(), Some("account"));
    assert_eq!(record.last_refresh.as_deref(), Some("now"));
    assert!(record.supports_usage_api);
    assert_eq!(codex_credential_record(b"invalid".to_vec()), None);
}

#[test]
fn mode_maps_known_unknown_and_missing_values() {
    for (config, expected) in [
        (
            "cli_auth_credentials_store = 'auto'",
            CodexCredentialStoreMode::Automatic,
        ),
        ("cli_auth_credentials_store = 'file'", CodexCredentialStoreMode::File),
        (
            "cli_auth_credentials_store = 'keyring'",
            CodexCredentialStoreMode::Keyring,
        ),
        (
            "cli_auth_credentials_store = 'future'",
            CodexCredentialStoreMode::Unknown("future".into()),
        ),
        ("", CodexCredentialStoreMode::Automatic),
    ] {
        assert_eq!(codex_credential_mode(config.into()), expected, "{config}");
    }
}

#[test]
fn mode_at_path_reads_file_and_defaults_when_missing() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    assert_eq!(
        codex_credential_mode_at_path(path.display().to_string()),
        CodexCredentialStoreMode::Automatic
    );
    std::fs::write(&path, "cli_auth_credentials_store = 'file'").unwrap();
    assert_eq!(
        codex_credential_mode_at_path(path.display().to_string()),
        CodexCredentialStoreMode::File
    );
}

#[test]
fn homes_return_existing_default_directories() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    std::fs::create_dir_all(home.join(".codex")).unwrap();
    assert_eq!(
        codex_credential_homes(
            root.path().join("other").display().to_string(),
            home.display().to_string()
        ),
        [root.path().join("other"), home.join(".codex")].map(|path| path.display().to_string())
    );
}

#[test]
fn api_key_usage_message_is_the_shared_text() {
    assert_eq!(
        codex_api_key_usage_message(),
        quotaline_credentials::CODEX_API_KEY_USAGE_UNAVAILABLE
    );
}
