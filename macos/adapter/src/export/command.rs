//! Exposes macOS export command policy to Swift.

use quotaline_macos_platform::export::{self, Command, Invocation};

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum ExportCommandRecord {
    Icons,
    MenuBar,
    Popover,
}

impl From<Command> for ExportCommandRecord {
    fn from(command: Command) -> Self {
        match command {
            Command::Icons => Self::Icons,
            Command::MenuBar => Self::MenuBar,
            Command::Popover => Self::Popover,
        }
    }
}

impl From<ExportCommandRecord> for Command {
    fn from(command: ExportCommandRecord) -> Self {
        match command {
            ExportCommandRecord::Icons => Self::Icons,
            ExportCommandRecord::MenuBar => Self::MenuBar,
            ExportCommandRecord::Popover => Self::Popover,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ParsedExportCommand {
    pub command: ExportCommandRecord,
    pub directory: String,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum ParsedExportInvocation {
    UsageJson,
    Files {
        command: ExportCommandRecord,
        directory: String,
    },
}

#[uniffi::export]
pub fn parse_export_command(arguments: Vec<String>) -> Option<ParsedExportCommand> {
    export::parse_command(&arguments).map(|(command, directory)| ParsedExportCommand {
        command: command.into(),
        directory,
    })
}

#[uniffi::export]
pub fn parse_export_invocation(arguments: Vec<String>) -> Option<ParsedExportInvocation> {
    export::parse_invocation(&arguments).map(|invocation| match invocation {
        Invocation::UsageJson => ParsedExportInvocation::UsageJson,
        Invocation::Files { command, directory } => ParsedExportInvocation::Files {
            command: command.into(),
            directory,
        },
    })
}

#[uniffi::export]
pub fn export_failure_message(command: Option<ExportCommandRecord>) -> String {
    command
        .map_or("usage export failed", |command| {
            Command::from(command).failure_message()
        })
        .into()
}

#[cfg(test)]
#[path = "../../tests/unit/export/command.rs"]
mod tests;
