//! Resolves Codex credential storage and account homes.

use std::path::{Path, PathBuf};

mod account_discovery;
pub mod rollouts;
pub use account_discovery::{AccountDiscovery, AccountScan};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodexCredentialStoreMode {
    Automatic,
    File,
    Keyring,
    Unknown(String),
}

pub fn codex_credential_store_mode(config: &str) -> Option<CodexCredentialStoreMode> {
    let document: toml::Value = toml::from_str(config).ok()?;
    document
        .get("cli_auth_credentials_store")?
        .as_str()
        .map(|mode| match mode {
            "auto" => CodexCredentialStoreMode::Automatic,
            "file" => CodexCredentialStoreMode::File,
            "keyring" => CodexCredentialStoreMode::Keyring,
            value => CodexCredentialStoreMode::Unknown(value.into()),
        })
}

pub fn read_codex_credential_store_mode(path: &Path) -> Option<CodexCredentialStoreMode> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|config| codex_credential_store_mode(&config))
}

pub fn codex_homes(configured: &Path, home: &Path) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    [
        configured.to_path_buf(),
        home.join(".codex"),
        home.join(".config/codex"),
    ]
    .into_iter()
    .filter(|path| {
        let normalized = path.canonicalize().unwrap_or_else(|_| path.clone());
        seen.insert(normalized) && (path == configured || path.is_dir())
    })
    .collect()
}

pub fn select_codex_home(selected: Option<&Path>, available: &[PathBuf]) -> Option<PathBuf> {
    match selected {
        Some(selected) => available.iter().find(|home| home.as_path() == selected).cloned(),
        None => available.first().cloned(),
    }
}

#[cfg(test)]
#[path = "../tests/unit/codex/tests.rs"]
mod tests;
