//! Reads Copilot CLI and legacy credential files for both platforms.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use quotaline_credentials::CopilotCredential;
use quotaline_credentials::copilot_cli_accounts;
use quotaline_credentials::copilot_legacy_credential;
use quotaline_storage::read_optional_file;
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CopilotCli {
    pub accounts: Vec<String>,
    pub credential: Option<CopilotCredential>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopilotLegacy {
    pub credential: CopilotCredential,
    pub path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopilotLegacyRecovery {
    Stop,
    TryNext,
}

#[derive(Debug, Error)]
pub enum CopilotSourceError {
    #[error("cannot read {}: {source}", path.display())]
    File { path: PathBuf, source: std::io::Error },
    #[error("{} is not valid JSON: {source}", path.display())]
    Json { path: PathBuf, source: serde_json::Error },
    #[error("{} is not a JSON object", path.display())]
    LegacyShape { path: PathBuf },
}

#[derive(Debug, PartialEq, Eq)]
pub enum CopilotOrigin {
    Environment,
    Keyring,
    Cli(PathBuf),
    Legacy(PathBuf),
}

#[derive(Debug, PartialEq, Eq)]
pub struct CopilotFound {
    pub credential: CopilotCredential,
    pub origin: CopilotOrigin,
}

pub enum CopilotDiscovery {
    Found(CopilotFound),
    Pending(CopilotPending),
}

pub struct CopilotPending {
    cli: CopilotCli,
    cli_path: PathBuf,
    cli_error: Option<CopilotSourceError>,
}

impl CopilotPending {
    pub fn keyring_accounts(&self) -> &[String] {
        &self.cli.accounts
    }

    pub fn finish<E>(
        self,
        keyring: Result<Option<CopilotCredential>, E>,
        legacy_paths: &[PathBuf],
        recovery: CopilotLegacyRecovery,
    ) -> Result<Option<CopilotFound>, CopilotDiscoveryError<E>> {
        let keyring_error = match keyring {
            Ok(Some(credential)) => {
                return Ok(Some(CopilotFound {
                    credential,
                    origin: CopilotOrigin::Keyring,
                }));
            }
            Ok(None) => None,
            Err(error) => Some(error),
        };
        if let Some(credential) = self.cli.credential {
            return Ok(Some(CopilotFound {
                credential,
                origin: CopilotOrigin::Cli(self.cli_path),
            }));
        }
        let legacy = match discover_copilot_legacy(legacy_paths, recovery) {
            Ok(Some(found)) => {
                return Ok(Some(CopilotFound {
                    credential: found.credential,
                    origin: CopilotOrigin::Legacy(found.path),
                }));
            }
            other => other,
        };
        if let Some(error) = keyring_error {
            return Err(CopilotDiscoveryError::Keyring(error));
        }
        if let Some(error) = self.cli_error {
            return Err(CopilotDiscoveryError::Source(error));
        }
        legacy.map(|_| None).map_err(CopilotDiscoveryError::Source)
    }
}

#[derive(Debug, Error)]
pub enum CopilotDiscoveryError<E> {
    #[error("{0}")]
    Keyring(E),
    #[error(transparent)]
    Source(CopilotSourceError),
}

pub fn begin_copilot_discovery(environment: Option<CopilotCredential>, cli_path: &Path) -> CopilotDiscovery {
    if let Some(credential) = environment {
        return CopilotDiscovery::Found(CopilotFound {
            credential,
            origin: CopilotOrigin::Environment,
        });
    }
    let (cli, cli_error) = match read_copilot_cli(cli_path) {
        Ok(cli) => (cli, None),
        Err(error) => (CopilotCli::default(), Some(error)),
    };
    CopilotDiscovery::Pending(CopilotPending {
        cli,
        cli_path: cli_path.into(),
        cli_error,
    })
}

pub fn read_copilot_cli(path: &Path) -> Result<CopilotCli, CopilotSourceError> {
    let Some(document) = read_document(path)? else {
        return Ok(CopilotCli::default());
    };
    let mut seen = HashSet::new();
    let mut result = CopilotCli::default();
    for entry in copilot_cli_accounts(&document) {
        if let Some(account) = entry.account.filter(|account| seen.insert(account.clone())) {
            result.accounts.push(account);
        }
        if result.credential.is_none() {
            result.credential = entry.credential;
        }
    }
    Ok(result)
}

pub fn discover_copilot_legacy(
    paths: &[PathBuf],
    recovery: CopilotLegacyRecovery,
) -> Result<Option<CopilotLegacy>, CopilotSourceError> {
    let mut first_error = None;
    for path in paths {
        match read_document(path) {
            Ok(Some(document)) if document.is_object() => {
                if let Some(credential) = copilot_legacy_credential(&document) {
                    return Ok(Some(CopilotLegacy {
                        credential,
                        path: path.clone(),
                    }));
                }
            }
            Ok(Some(_)) if recovery == CopilotLegacyRecovery::Stop => {
                return Err(CopilotSourceError::LegacyShape { path: path.clone() });
            }
            Ok(_) => {}
            Err(error) if recovery == CopilotLegacyRecovery::Stop => return Err(error),
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }
    first_error.map_or(Ok(None), Err)
}

fn read_document(path: &Path) -> Result<Option<Value>, CopilotSourceError> {
    let Some(bytes) = read_optional_file(path).map_err(|source| CopilotSourceError::File {
        path: path.into(),
        source,
    })?
    else {
        return Ok(None);
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|source| CopilotSourceError::Json {
            path: path.into(),
            source,
        })
}

#[cfg(test)]
#[path = "../tests/unit/copilot/tests.rs"]
mod tests;
