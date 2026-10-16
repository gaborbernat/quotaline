//! Reads and validates local provider credential documents.

use std::{
    io,
    path::{Path, PathBuf},
};

use quotaline_credentials::claude_credential_document;
use quotaline_credentials::codex_credential_document;
use quotaline_credentials::gemini_credential_document;
use quotaline_storage::read_optional_file;
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialProvider {
    Claude,
    Codex,
    Gemini,
}

#[derive(Debug, Error)]
pub enum CredentialFileError {
    #[error("Cannot read {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{path} is not JSON: {source}")]
    Json { path: PathBuf, source: serde_json::Error },
}

pub fn usable_credential_payload(provider: CredentialProvider, data: &[u8]) -> Result<bool, serde_json::Error> {
    let document: Value = serde_json::from_slice(data)?;
    Ok(match provider {
        CredentialProvider::Claude => claude_credential_document(&document).is_some(),
        CredentialProvider::Codex => codex_credential_document(&document).is_some(),
        CredentialProvider::Gemini => gemini_credential_document(&document).is_some(),
    })
}

pub fn read_credential_file(provider: CredentialProvider, path: &Path) -> Result<Option<Vec<u8>>, CredentialFileError> {
    let Some(data) = read_optional_file(path).map_err(|source| CredentialFileError::Io {
        path: path.into(),
        source,
    })?
    else {
        return Ok(None);
    };
    usable_credential_payload(provider, &data)
        .map_err(|source| CredentialFileError::Json {
            path: path.into(),
            source,
        })
        .map(|usable| usable.then_some(data))
}

#[cfg(test)]
#[path = "../tests/unit/credential_file.rs"]
mod tests;
