//! Checks the public behavior of the adjacent module.

use super::*;

#[test]
fn login_flow_returns_only_completed_launches() {
    let flow = ProviderLoginFlow::new();
    assert!(flow.begin("claude".into()));
    assert!(!flow.begin("claude".into()));
    assert!(flow.begin("codex".into()));
    assert!(flow.take_returned_providers().is_empty());
    flow.opened("claude".into());
    assert_eq!(flow.take_returned_providers(), ["claude"]);
    assert!(flow.take_returned_providers().is_empty());
    assert!(!flow.begin("codex".into()));
    flow.failed("codex".into());
    assert!(flow.begin("codex".into()));
    let default_flow = ProviderLoginFlow::default();
    assert!(default_flow.begin("cursor".into()));
}

#[test]
fn provider_command_selection_preserves_credential_ownership() {
    for (provider, source, expected) in [
        ("claude", None, Some(LoginCommand::Claude)),
        ("codex", None, Some(LoginCommand::Codex)),
        ("cursor", Some("cursor.agent"), Some(LoginCommand::Cursor)),
        ("copilot", Some("copilot.keychain"), Some(LoginCommand::Copilot)),
        ("copilot", Some("copilot.file"), Some(LoginCommand::Copilot)),
        ("cursor", Some("cursor.app"), None),
        ("gemini", None, None),
    ] {
        assert_eq!(
            provider_login_command(provider.into(), source.map(str::to_owned)),
            expected,
            "{provider} {source:?}"
        );
    }
}

#[test]
fn configuration_directories_keep_only_supported_values() {
    assert_eq!(
        login_configuration_directories(Some("/claude".into()), None, Some("/copilot".into())),
        vec![
            ConfiguredLoginDirectory {
                command: LoginCommand::Claude,
                path: "/claude".into(),
            },
            ConfiguredLoginDirectory {
                command: LoginCommand::Copilot,
                path: "/copilot".into(),
            },
        ]
    );
    assert_eq!(
        login_configuration_directories(None, Some("/codex".into()), None)[0].command,
        LoginCommand::Codex
    );
}

#[test]
fn scripts_use_fixed_commands_and_quote_configuration_paths() {
    for (command, executable) in [
        (LoginCommand::Claude, "claude auth login"),
        (LoginCommand::Codex, "codex login"),
        (LoginCommand::Cursor, "cursor-agent login"),
        (LoginCommand::Copilot, "copilot login"),
    ] {
        let script = login_script(command, None);
        assert!(script.starts_with("#!/bin/zsh -l\n"));
        assert!(script.contains(&format!("\n{executable}\n")));
        assert!(!script.contains("fixture-secret"));
    }
    assert!(login_script(LoginCommand::Claude, None).contains("unset CLAUDE_CONFIG_DIR\nclaude auth login"));
    assert!(
        login_script(LoginCommand::Codex, Some("/team/O'Brien".into()))
            .contains("export CODEX_HOME='/team/O'\\''Brien'\ncodex login")
    );
    assert!(!login_script(LoginCommand::Cursor, Some("/unused".into())).contains("/unused"));
}

#[test]
fn login_script_is_written_atomically_with_private_permissions() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("Login");
    for command in [
        LoginCommand::Claude,
        LoginCommand::Codex,
        LoginCommand::Cursor,
        LoginCommand::Copilot,
    ] {
        let path = write_login_command(command, folder.to_string_lossy().into_owned(), None).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), login_script(command, None));
        assert_eq!(fs::metadata(&folder).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(
            write_login_command(command, folder.to_string_lossy().into_owned(), None).unwrap(),
            path
        );
    }
    assert_eq!(fs::read_dir(folder).unwrap().count(), 4);
}

#[test]
fn login_file_write_reports_invalid_directory() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("not-a-directory");
    fs::write(&file, []).unwrap();
    let LoginFileError::Io { message } =
        write_login_command(LoginCommand::Claude, file.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!message.is_empty());
}

#[test]
fn login_file_write_reports_failed_atomic_replace() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("Login");
    fs::create_dir_all(folder.join("claude.command")).unwrap();
    let LoginFileError::Io { message } =
        write_login_command(LoginCommand::Claude, folder.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!message.is_empty());
}
