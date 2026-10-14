//! Exposes shared Codex refresh rules to the macOS client.

use quotaline_credentials::CodexRefresh;
use quotaline_credentials::codex_refreshed_document;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CodexRefreshToken {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
}

impl From<CodexRefreshToken> for CodexRefresh {
    fn from(value: CodexRefreshToken) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            id_token: value.id_token,
            error: None,
        }
    }
}

#[uniffi::export]
pub fn codex_updated_credentials(document: Vec<u8>, token: CodexRefreshToken, refreshed_at: String) -> Option<Vec<u8>> {
    let document: Value = serde_json::from_slice(&document).ok()?;
    Some(
        serde_json::to_vec(&codex_refreshed_document(&document, &token.into(), &refreshed_at)?)
            .expect("Codex document is serializable"),
    )
}

#[cfg(test)]
#[path = "../../tests/unit/providers/codex_credentials/tests.rs"]
mod tests;
