//! Finds installed OAuth clients in macOS application and CLI locations.

use quotaline_credential_sources::google_oauth::{self, OAuthClient};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

const ANTIGRAVITY_APPLICATIONS: [&str; 2] = ["Antigravity.app", "Gemini.app"];
const ANTIGRAVITY_PATHS: [&str; 3] = [
    "Contents/Resources/bin/language_server_macos_arm",
    "Contents/Resources/app/out/main.js",
    "Contents/MacOS/Gemini",
];
const GEMINI_PATHS: [&str; 4] = [
    "node_modules/@google/gemini-cli-core/dist/src/code_assist/oauth2.js",
    "lib/node_modules/@google/gemini-cli-core/dist/src/code_assist/oauth2.js",
    "lib/node_modules/@google/gemini-cli/node_modules/@google/gemini-cli-core/dist/src/code_assist/oauth2.js",
    "lib/node_modules/@google/gemini-cli/dist/gemini.js",
];

pub fn antigravity_client(
    home: &Path,
    applications_root: &Path,
    environment: &HashMap<String, String>,
) -> Option<OAuthClient> {
    if let (Some(id), Some(secret)) = (
        environment.get("ANTIGRAVITY_OAUTH_CLIENT_ID"),
        environment.get("ANTIGRAVITY_OAUTH_CLIENT_SECRET"),
    ) {
        return Some(OAuthClient {
            id: id.clone(),
            secret: secret.clone(),
        });
    }
    [home.join("Applications"), applications_root.to_path_buf()]
        .into_iter()
        .flat_map(|root| ANTIGRAVITY_APPLICATIONS.map(|app| root.join(app)))
        .flat_map(|app| ANTIGRAVITY_PATHS.map(|path| app.join(path)))
        .find_map(|path| google_oauth::antigravity_binary(&path))
}

pub fn gemini_client(
    home: &Path,
    system_prefixes: &[PathBuf],
    environment: &HashMap<String, String>,
) -> Option<OAuthClient> {
    if let (Some(id), Some(secret)) = (
        environment.get("GEMINI_OAUTH_CLIENT_ID"),
        environment.get("GEMINI_OAUTH_CLIENT_SECRET"),
    ) {
        return Some(OAuthClient {
            id: id.clone(),
            secret: secret.clone(),
        });
    }
    environment
        .get("NPM_CONFIG_PREFIX")
        .map(PathBuf::from)
        .into_iter()
        .chain(std::iter::once(home.join(".npm-global")))
        .chain(system_prefixes.iter().cloned())
        .flat_map(|root| GEMINI_PATHS.map(|path| root.join(path)))
        .find_map(|path| {
            fs::read_to_string(path)
                .ok()
                .and_then(|source| google_oauth::gemini_source(&source))
        })
}

#[cfg(test)]
#[path = "../tests/unit/google_oauth/tests.rs"]
mod tests;
