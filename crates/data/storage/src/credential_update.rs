//! Keeps refreshed credential files from replacing a newer sign-in.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde_json::Value;
use thiserror::Error;

use crate::atomic_json;

pub struct CredentialFileGuard {
    path: PathBuf,
    original: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum CredentialFileError {
    #[error("cannot read credential file: {0}")]
    Read(io::Error),
    #[error("credential file is not JSON: {0}")]
    InvalidJson(serde_json::Error),
    #[error("credential file changed during refresh")]
    Changed,
    #[error("cannot save credential file: {0}")]
    Write(io::Error),
}

impl CredentialFileGuard {
    pub fn observe(path: &Path, expected: &Value) -> Result<Self, CredentialFileError> {
        let original = fs::read(path).map_err(CredentialFileError::Read)?;
        let current: Value = serde_json::from_slice(&original).map_err(CredentialFileError::InvalidJson)?;
        if &current != expected {
            return Err(CredentialFileError::Changed);
        }
        Ok(Self {
            path: path.into(),
            original,
        })
    }

    pub fn replace(self, updated: &Value) -> Result<(), CredentialFileError> {
        if fs::read(&self.path).map_err(CredentialFileError::Read)? != self.original {
            return Err(CredentialFileError::Changed);
        }
        atomic_json(&self.path, updated).map_err(CredentialFileError::Write)
    }
}

#[cfg(test)]
#[path = "../tests/unit/credential_update.rs"]
mod tests;
