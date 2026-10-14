//! Selects macOS command-line exports before the app starts.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Icons,
    MenuBar,
    Popover,
}

impl Command {
    pub fn failure_message(self) -> &'static str {
        match self {
            Self::Icons => "icon export failed",
            Self::MenuBar => "menu bar export failed",
            Self::Popover => "popover export failed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Invocation {
    UsageJson,
    Files { command: Command, directory: String },
}

pub fn parse_command(arguments: &[String]) -> Option<(Command, String)> {
    for (flag, command) in [
        ("--export-icon", Command::Icons),
        ("--export-menubar", Command::MenuBar),
        ("--export-popover", Command::Popover),
    ] {
        if let Some(index) = arguments.iter().position(|argument| argument == flag)
            && let Some(directory) = arguments.get(index + 1)
        {
            return Some((command, directory.clone()));
        }
    }
    None
}

pub fn parse_invocation(arguments: &[String]) -> Option<Invocation> {
    if arguments.iter().any(|argument| argument == "--usage-json") {
        return Some(Invocation::UsageJson);
    }
    parse_command(arguments).map(|(command, directory)| Invocation::Files { command, directory })
}

#[cfg(test)]
#[path = "../tests/unit/export/tests.rs"]
mod tests;
