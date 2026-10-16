//! Bridges shared Cursor credential sources to macOS.

use quotaline_credential_sources::{CursorAuth, CursorOrigin, CursorSourceError};

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CursorCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub email: Option<String>,
    pub membership_type: Option<String>,
}

impl From<CursorAuth> for CursorCredentials {
    fn from(auth: CursorAuth) -> Self {
        Self {
            access_token: auth.access_token,
            refresh_token: auth.refresh_token,
            email: auth.email,
            membership_type: auth.membership_type,
        }
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CursorCredentialError {
    #[error("Cannot open Cursor state: {message}")]
    StateOpen { message: String },
    #[error("Cannot query Cursor state: {message}")]
    StateQuery { message: String },
    #[error("Cannot read Cursor state: {message}")]
    StateRead { message: String },
    #[error("Cannot read Cursor agent credentials: {message}")]
    AgentFile { message: String },
    #[error("Cursor agent credentials are not JSON: {message}")]
    AgentJson { message: String },
}

impl From<CursorSourceError> for CursorCredentialError {
    fn from(error: CursorSourceError) -> Self {
        match error {
            CursorSourceError::Open(error) => Self::StateOpen {
                message: error.to_string(),
            },
            CursorSourceError::Query(error) => Self::StateQuery {
                message: error.to_string(),
            },
            CursorSourceError::Read(error) => Self::StateRead {
                message: error.to_string(),
            },
            CursorSourceError::File(error) => Self::AgentFile {
                message: error.to_string(),
            },
            CursorSourceError::Json(error) => Self::AgentJson {
                message: error.to_string(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CursorCredentialOrigin {
    App,
    Agent,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CursorDiscoveredCredentials {
    pub credentials: CursorCredentials,
    pub origin: CursorCredentialOrigin,
}

#[uniffi::export]
pub fn discover_cursor_credentials(
    state_path: String,
    agent_path: String,
) -> Result<Option<CursorDiscoveredCredentials>, CursorCredentialError> {
    quotaline_credential_sources::discover_cursor(std::path::Path::new(&state_path), std::path::Path::new(&agent_path))
        .map(|found| {
            found.map(|found| CursorDiscoveredCredentials {
                credentials: found.auth.into(),
                origin: match found.origin {
                    CursorOrigin::App => CursorCredentialOrigin::App,
                    CursorOrigin::Agent => CursorCredentialOrigin::Agent,
                },
            })
        })
        .map_err(Into::into)
}

#[cfg(test)]
#[path = "../../tests/unit/credentials/cursor/discovery.rs"]
mod tests;
