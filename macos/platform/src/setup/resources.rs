//! Defines macOS sandbox resource locations and grant descriptions.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Need {
    Required,
    Optional,
    OneOf(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Override {
    Path(&'static str),
    Home(&'static str),
    Prefix(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resource {
    pub id: &'static str,
    pub provider: &'static str,
    pub relative_path: &'static str,
    pub need: Need,
    pub explanation: &'static str,
    override_path: Option<Override>,
}

const RESOURCES: [Resource; 7] = [
    Resource {
        id: "claude.home",
        provider: "claude",
        relative_path: ".claude",
        need: Need::Optional,
        explanation: concat!(
            "Claude Code keeps its transcripts here. They give History its token counts and API-equivalent ",
            "costs. Quota, plan and account come from Claude Code's Keychain sign-in, so Claude works without ",
            "this folder. It also holds the credentials file Claude Code writes only when the Keychain refuses ",
            "its sign-in."
        ),
        override_path: Some(Override::Path("CLAUDE_CONFIG_DIR")),
    },
    Resource {
        id: "codex.home",
        provider: "codex",
        relative_path: ".codex",
        need: Need::Required,
        explanation: concat!(
            "Codex keeps its sign-in (auth.json) and its session logs here. The sign-in lets the app ask OpenAI ",
            "for your limits; the logs give History its token counts."
        ),
        override_path: Some(Override::Path("CODEX_HOME")),
    },
    Resource {
        id: "gemini.home",
        provider: "gemini",
        relative_path: ".gemini",
        need: Need::Required,
        explanation: concat!(
            "Gemini CLI keeps its sign-in (oauth_creds.json) here. The app uses it to ask Google for your ",
            "limits. Not needed if you store the sign-in in the Keychain with GEMINI_FORCE_ENCRYPTED_FILE_STORAGE."
        ),
        override_path: Some(Override::Home("GEMINI_CLI_HOME")),
    },
    Resource {
        id: "cursor.support",
        provider: "cursor",
        relative_path: "Library/Application Support/Cursor",
        need: Need::OneOf("cursor"),
        explanation: concat!(
            "The Cursor app keeps its session in a database here (state.vscdb). The app reads it, read-only, to ",
            "ask Cursor for your usage. Grant this folder if you sign in through the Cursor app; ~/.cursor is ",
            "then not needed."
        ),
        override_path: None,
    },
    Resource {
        id: "cursor.home",
        provider: "cursor",
        relative_path: ".cursor",
        need: Need::OneOf("cursor"),
        explanation: concat!(
            "cursor-agent saves its sign-in (auth.json) here. Grant this folder if you sign in with cursor-agent; ",
            "the Cursor app's folder is then not needed."
        ),
        override_path: None,
    },
    Resource {
        id: "copilot.home",
        provider: "copilot",
        relative_path: ".copilot",
        need: Need::OneOf("copilot"),
        explanation: concat!(
            "The GitHub Copilot CLI saves its settings and sign-in (config.json) here. Grant this folder if you ",
            "sign in with copilot login and the sign-in is not in the Keychain; ~/.config/github-copilot is ",
            "then not needed."
        ),
        override_path: Some(Override::Path("COPILOT_HOME")),
    },
    Resource {
        id: "copilot.config",
        provider: "copilot",
        relative_path: ".config/github-copilot",
        need: Need::OneOf("copilot"),
        explanation: concat!(
            "GitHub Copilot's editor extensions save hosts.json and apps.json here. Grant this folder if you use ",
            "Copilot only through an editor extension; ~/.copilot is then not needed."
        ),
        override_path: Some(Override::Prefix("XDG_CONFIG_HOME")),
    },
];

pub fn all() -> &'static [Resource] {
    &RESOURCES
}

pub fn configured_providers(configured: &[String], intervals: &[String], bookmarked: &[String]) -> Vec<&'static str> {
    let configured = configured.iter().map(String::as_str).collect::<HashSet<_>>();
    let intervals = intervals.iter().map(String::as_str).collect::<HashSet<_>>();
    let bookmarked = bookmarked.iter().map(String::as_str).collect::<HashSet<_>>();
    quotaline_catalog::IDS
        .into_iter()
        .filter(|id| {
            configured.contains(id)
                || intervals.contains(id)
                || RESOURCES
                    .iter()
                    .any(|resource| resource.provider == *id && bookmarked.contains(resource.id))
        })
        .collect()
}

pub fn missing_ids(provider: &str, bookmarked: &[String]) -> Vec<&'static str> {
    let bookmarked = bookmarked.iter().map(String::as_str).collect::<HashSet<_>>();
    RESOURCES
        .iter()
        .filter(|resource| resource.provider == provider && !bookmarked.contains(resource.id))
        .map(|resource| resource.id)
        .collect()
}

pub fn needs_access(provider: &str, credential_source: Option<&str>) -> bool {
    provider != "copilot" || credential_source != Some("copilot.environment")
}

pub fn required_ids(environment: &HashMap<String, String>) -> Vec<&'static str> {
    let copilot_source =
        quotaline_credentials::copilot_environment_credential(environment).map(|_| "copilot.environment");
    all()
        .iter()
        .filter(|resource| needs_access(resource.provider, copilot_source))
        .map(|resource| resource.id)
        .collect()
}

pub fn configured_path(resource: &Resource, environment: &HashMap<String, String>, home: &Path) -> PathBuf {
    match resource.override_path.and_then(|mode| match mode {
        Override::Path(key) => environment.get(key).map(|value| PathBuf::from(value.as_str())),
        Override::Home(key) => environment
            .get(key)
            .map(|value| Path::new(value).join(resource.relative_path)),
        Override::Prefix(key) => environment.get(key).map(|value| {
            Path::new(value).join(
                Path::new(resource.relative_path)
                    .file_name()
                    .expect("resource path has a name"),
            )
        }),
    }) {
        Some(path) => path,
        None => home.join(resource.relative_path),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/setup/resources.rs"]
mod tests;
