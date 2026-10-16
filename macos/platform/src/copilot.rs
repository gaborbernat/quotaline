//! Discovers macOS Copilot credentials across environment, Keychain, and files.

use std::{collections::HashMap, path::PathBuf};

use crate::keychain::Reader;
use quotaline_credential_sources::{
    CopilotDiscovery, CopilotDiscoveryError, CopilotLegacyRecovery, CopilotOrigin, CopilotSourceError,
    begin_copilot_discovery,
};
use quotaline_credentials::CopilotCredential;
use quotaline_credentials::copilot_environment_credential;
use quotaline_credentials::copilot_keyring_credential;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Environment,
    Keychain,
    Cli,
    Legacy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub credential: CopilotCredential,
    pub origin: Origin,
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("Copilot Keychain read failed with status {0}")]
    Keychain(i32),
    #[error("Copilot Keychain item is not a token")]
    KeychainMalformed,
    #[error(transparent)]
    File(#[from] CopilotSourceError),
}

pub fn discover(
    environment: &HashMap<String, String>,
    cli_path: &std::path::Path,
    legacy_paths: &[PathBuf],
    keychain: &impl Reader,
) -> Result<Option<Found>, DiscoveryError> {
    let found = match begin_copilot_discovery(copilot_environment_credential(environment), cli_path) {
        CopilotDiscovery::Found(found) => Some(found),
        CopilotDiscovery::Pending(pending) => {
            let keyring = read_keychain(pending.keyring_accounts(), keychain);
            pending
                .finish(keyring, legacy_paths, CopilotLegacyRecovery::Stop)
                .map_err(|error| match error {
                    CopilotDiscoveryError::Keyring(error) => error,
                    CopilotDiscoveryError::Source(error) => DiscoveryError::File(error),
                })?
        }
    };
    Ok(found.map(|found| Found {
        credential: found.credential,
        origin: match found.origin {
            CopilotOrigin::Environment => Origin::Environment,
            CopilotOrigin::Keyring => Origin::Keychain,
            CopilotOrigin::Cli(_) => Origin::Cli,
            CopilotOrigin::Legacy(_) => Origin::Legacy,
        },
    }))
}

fn read_keychain(accounts: &[String], keychain: &impl Reader) -> Result<Option<CopilotCredential>, DiscoveryError> {
    let mut first_error = None;
    let accounts = if accounts.is_empty() {
        vec![None]
    } else {
        accounts.iter().cloned().map(Some).collect()
    };
    for account in accounts {
        match keychain.load(crate::keychain::COPILOT_SERVICE, account) {
            Ok(Some(item)) => match copilot_keyring_credential(&item.data, item.account.as_deref()) {
                Ok(Some(credential)) => return Ok(Some(credential)),
                Ok(None) => {}
                Err(_) => {
                    first_error.get_or_insert(DiscoveryError::KeychainMalformed);
                }
            },
            Ok(None) => {}
            Err(status) => {
                first_error.get_or_insert(DiscoveryError::Keychain(status));
            }
        }
    }
    first_error.map_or(Ok(None), Err)
}
