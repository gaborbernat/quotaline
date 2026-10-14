//! Maps shared sign-in actions into macOS launch commands.

use quotaline_setup::action;
use quotaline_setup::login::LoginFlow;
use std::{fs, io::Write, os::unix::fs::PermissionsExt, path::PathBuf, sync::Mutex};

#[derive(uniffi::Object)]
pub struct ProviderLoginFlow {
    state: Mutex<LoginFlow>,
}

#[uniffi::export]
impl ProviderLoginFlow {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(LoginFlow::default()),
        }
    }

    pub fn begin(&self, provider: String) -> bool {
        self.state.lock().unwrap().begin(&provider)
    }

    pub fn opened(&self, provider: String) {
        self.state.lock().unwrap().opened(&provider);
    }

    pub fn failed(&self, provider: String) {
        self.state.lock().unwrap().failed(&provider);
    }

    pub fn take_returned_providers(&self) -> Vec<String> {
        self.state.lock().unwrap().take_returned()
    }
}

impl Default for ProviderLoginFlow {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum LoginCommand {
    Claude,
    Codex,
    Cursor,
    Copilot,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ConfiguredLoginDirectory {
    pub command: LoginCommand,
    pub path: String,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum LoginFileError {
    #[error("Could not write provider login command: {message}")]
    Io { message: String },
}

fn io_error(error: std::io::Error) -> LoginFileError {
    LoginFileError::Io {
        message: error.to_string(),
    }
}

impl LoginCommand {
    fn name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::Copilot => "copilot",
        }
    }

    fn executable(self) -> &'static str {
        match self {
            Self::Claude => "claude auth login",
            Self::Codex => "codex login",
            Self::Cursor => "cursor-agent login",
            Self::Copilot => "copilot login",
        }
    }

    fn configuration_variable(self) -> Option<&'static str> {
        match self {
            Self::Claude => Some("CLAUDE_CONFIG_DIR"),
            Self::Codex => Some("CODEX_HOME"),
            Self::Copilot => Some("COPILOT_HOME"),
            Self::Cursor => None,
        }
    }
}

#[uniffi::export]
pub fn provider_login_command(provider: String, source: Option<String>) -> Option<LoginCommand> {
    action::login_command(&provider, source.as_deref()).map(|command| match command {
        action::LoginCommand::Claude => LoginCommand::Claude,
        action::LoginCommand::Codex => LoginCommand::Codex,
        action::LoginCommand::Cursor => LoginCommand::Cursor,
        action::LoginCommand::Copilot => LoginCommand::Copilot,
    })
}

#[uniffi::export]
pub fn login_configuration_directories(
    claude: Option<String>,
    codex: Option<String>,
    copilot: Option<String>,
) -> Vec<ConfiguredLoginDirectory> {
    [
        (LoginCommand::Claude, claude),
        (LoginCommand::Codex, codex),
        (LoginCommand::Copilot, copilot),
    ]
    .into_iter()
    .filter_map(|(command, path)| path.map(|path| ConfiguredLoginDirectory { command, path }))
    .collect()
}

#[uniffi::export]
pub fn login_script(command: LoginCommand, configuration_directory: Option<String>) -> String {
    let configuration = match (command.configuration_variable(), configuration_directory) {
        (Some(variable), Some(path)) => format!("export {variable}='{}'\n", path.replace("'", "'\\''")),
        (Some(variable), None) => format!("unset {variable}\n"),
        (None, _) => String::new(),
    };
    format!(
        "#!/bin/zsh -l\ncd || exit 1\nexport PATH=\"$HOME/.local/bin:/opt/homebrew/bin:/usr/local/bin:$PATH\"\n{}{}\nresult=$?\nif (( result == 0 )); then\n  print 'Sign-in finished. Return to Quotaline to refresh usage.'\nelse\n  print 'Sign-in did not finish. Resolve the error above and try again.'\nfi\nexit \"$result\"\n\n",
        configuration,
        command.executable()
    )
}

#[uniffi::export]
pub fn write_login_command(
    command: LoginCommand,
    directory: String,
    configuration_directory: Option<String>,
) -> Result<String, LoginFileError> {
    let folder = PathBuf::from(directory);
    if !folder.is_dir() {
        fs::create_dir_all(&folder).map_err(io_error)?;
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).map_err(io_error)?;
    }
    let path = folder.join(format!("{}.command", command.name()));
    let mut file = tempfile::NamedTempFile::new_in(&folder).map_err(io_error)?;
    file.write_all(login_script(command, configuration_directory).as_bytes())
        .map_err(io_error)?;
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(0o700))
        .map_err(io_error)?;
    file.persist(&path).map_err(|error| io_error(error.error))?;
    Ok(path.to_string_lossy().into_owned())
}

#[cfg(test)]
#[path = "../tests/unit/login/tests.rs"]
mod tests;
