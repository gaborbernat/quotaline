//! Defines macOS sign-in commands and credential source identifiers.

pub mod resources;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderSetup {
    pub sign_in_command: &'static str,
    pub credential_sources: &'static [&'static str],
}

pub fn metadata(id: &str) -> Option<ProviderSetup> {
    let (sign_in_command, credential_sources): (&'static str, &'static [&'static str]) = match id {
        "claude" => ("claude auth login", &["claude.keychain", "claude.file"]),
        "codex" => ("codex login", &["codex.keyring", "codex.file"]),
        "gemini" => ("gemini", &["gemini.keychain", "gemini.file"]),
        "antigravity" => ("agy", &["antigravity.keychain"]),
        "cursor" => ("cursor-agent login", &["cursor.app", "cursor.agent"]),
        "copilot" => (
            "copilot login",
            &[
                "copilot.environment",
                "copilot.keychain",
                "copilot.file",
                "copilot.legacy-file",
            ],
        ),
        _ => return None,
    };
    Some(ProviderSetup {
        sign_in_command,
        credential_sources,
    })
}

#[cfg(test)]
#[path = "../tests/unit/setup/tests.rs"]
mod tests;
