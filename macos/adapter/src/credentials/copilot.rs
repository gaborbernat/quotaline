//! Bridges Copilot credential discovery to Swift.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use quotaline_credential_sources::CopilotSourceError;
use quotaline_credentials::CopilotCredential;
use quotaline_macos_platform::{
    copilot::{self, DiscoveryError, Origin},
    keychain,
};

use super::{keychain::KeychainBridge, keychain_callback::KeychainReader};

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CopilotCredentialRecord {
    pub token: String,
    pub user: Option<String>,
    pub host: String,
}

impl From<CopilotCredential> for CopilotCredentialRecord {
    fn from(value: CopilotCredential) -> Self {
        Self {
            token: value.token,
            user: value.user,
            host: value.host,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CopilotDiscoveredCredential {
    pub credential: CopilotCredentialRecord,
    pub source_id: String,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum CopilotDiscoveryError {
    #[error("{detail}")]
    Failure { source_id: String, detail: String },
}

fn discovery_error(value: DiscoveryError, cli_path: &Path) -> CopilotDiscoveryError {
    let (source_id, detail) = match value {
        DiscoveryError::Keychain(status) => (
            source_id(Origin::Keychain),
            if keychain::is_denial(status) {
                "Keychain access denied; allow it in Keychain Access or sign in again.".into()
            } else {
                "The Keychain item could not be read.".into()
            },
        ),
        DiscoveryError::KeychainMalformed => (
            source_id(Origin::Keychain),
            "GitHub Copilot Keychain item is not a token".into(),
        ),
        DiscoveryError::File(CopilotSourceError::File { path, .. }) => (
            source_id(if path == cli_path { Origin::Cli } else { Origin::Legacy }),
            "The credential file could not be read.".into(),
        ),
        DiscoveryError::File(CopilotSourceError::Json { path, .. }) => {
            let cli = path == cli_path;
            (
                source_id(if cli { Origin::Cli } else { Origin::Legacy }),
                format!(
                    "{} is not {}",
                    path_name(&path),
                    if cli { "JSON" } else { "a JSON object" }
                ),
            )
        }
        DiscoveryError::File(CopilotSourceError::LegacyShape { path }) => (
            source_id(Origin::Legacy),
            format!("{} is not a JSON object", path_name(&path)),
        ),
    };
    CopilotDiscoveryError::Failure {
        source_id: source_id.into(),
        detail,
    }
}

fn source_id(origin: Origin) -> &'static str {
    match origin {
        Origin::Environment => "copilot.environment",
        Origin::Keychain => "copilot.keychain",
        Origin::Cli => "copilot.file",
        Origin::Legacy => "copilot.legacy-file",
    }
}

fn path_name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into()
}

#[uniffi::export]
pub fn discover_copilot_credentials(
    environment: HashMap<String, String>,
    cli_path: String,
    legacy_paths: Vec<String>,
    keychain: Arc<dyn KeychainReader>,
) -> Result<Option<CopilotDiscoveredCredential>, CopilotDiscoveryError> {
    let paths = legacy_paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    copilot::discover(&environment, Path::new(&cli_path), &paths, &KeychainBridge(keychain))
        .map(|found| {
            found.map(|found| CopilotDiscoveredCredential {
                credential: found.credential.into(),
                source_id: source_id(found.origin).into(),
            })
        })
        .map_err(|error| discovery_error(error, Path::new(&cli_path)))
}
