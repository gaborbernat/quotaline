//! Bridges provider credential document reads to Swift.

use quotaline_credential_sources::{
    CredentialFileError, CredentialProvider, read_credential_file as read_file,
    usable_credential_payload as usable_payload,
};

use crate::storage::StorageFileError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CredentialFileProvider {
    Claude,
    Codex,
    Gemini,
}

impl From<CredentialFileProvider> for CredentialProvider {
    fn from(provider: CredentialFileProvider) -> Self {
        match provider {
            CredentialFileProvider::Claude => Self::Claude,
            CredentialFileProvider::Codex => Self::Codex,
            CredentialFileProvider::Gemini => Self::Gemini,
        }
    }
}

#[uniffi::export]
pub fn credential_payload_usable(provider: CredentialFileProvider, payload: Vec<u8>) -> Result<bool, StorageFileError> {
    usable_payload(provider.into(), &payload).map_err(|_| StorageFileError::Malformed)
}

#[uniffi::export]
pub fn read_provider_credential_file(
    provider: CredentialFileProvider,
    path: String,
) -> Result<Option<Vec<u8>>, StorageFileError> {
    read_file(provider.into(), std::path::Path::new(&path)).map_err(|error| match error {
        CredentialFileError::Io { source, .. } => StorageFileError::Io {
            message: source.to_string(),
        },
        CredentialFileError::Json { .. } => StorageFileError::Malformed,
    })
}

#[cfg(test)]
#[path = "../../tests/unit/credentials/file.rs"]
mod tests;
