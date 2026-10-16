use super::*;

#[test]
fn storage_mode_reads_root_toml() {
    for (config, expected) in [
        (
            "cli_auth_credentials_store = 'auto'",
            Some(CodexCredentialStoreMode::Automatic),
        ),
        (
            "cli_auth_credentials_store = 'file'",
            Some(CodexCredentialStoreMode::File),
        ),
        (
            "cli_auth_credentials_store = 'keyring' # secure",
            Some(CodexCredentialStoreMode::Keyring),
        ),
        (
            "cli_auth_credentials_store = 'future'",
            Some(CodexCredentialStoreMode::Unknown("future".into())),
        ),
        ("[profile.work]\ncli_auth_credentials_store = 'file'", None),
        ("other = 1", None),
        ("invalid = [", None),
    ] {
        assert_eq!(codex_credential_store_mode(config), expected, "{config}");
    }
}

#[test]
fn storage_mode_reads_file_and_defaults_when_missing() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    assert_eq!(read_codex_credential_store_mode(&path), None);
    std::fs::write(&path, "cli_auth_credentials_store = 'keyring'").unwrap();
    assert_eq!(
        read_codex_credential_store_mode(&path),
        Some(CodexCredentialStoreMode::Keyring)
    );
}

#[test]
fn home_discovery_keeps_configured_and_existing_defaults_once() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    std::fs::create_dir_all(home.join(".codex")).unwrap();
    std::fs::create_dir_all(home.join(".config/codex")).unwrap();
    let configured = root.path().join("other");
    assert_eq!(
        codex_homes(&configured, &home),
        [configured, home.join(".codex"), home.join(".config/codex")]
    );
    assert_eq!(
        codex_homes(&home.join(".codex"), &home),
        [home.join(".codex"), home.join(".config/codex")]
    );
}

#[test]
fn home_selection_sticks_to_selected_account() {
    let homes = vec![PathBuf::from("first"), PathBuf::from("second")];
    assert_eq!(select_codex_home(None, &homes), Some(PathBuf::from("first")));
    assert_eq!(
        select_codex_home(Some(Path::new("second")), &homes),
        Some(PathBuf::from("second"))
    );
    assert_eq!(select_codex_home(Some(Path::new("missing")), &homes), None);
    assert_eq!(select_codex_home(None, &[]), None);
}
