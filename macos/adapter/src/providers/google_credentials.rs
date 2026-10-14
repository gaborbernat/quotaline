//! Exposes shared Google OAuth refresh rules to macOS.

use quotaline_credentials::GoogleRefresh;
use quotaline_credentials::gemini_refreshed_document;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct GoogleRefreshToken {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_in: f64,
}

impl From<GoogleRefreshToken> for GoogleRefresh {
    fn from(value: GoogleRefreshToken) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            id_token: value.id_token,
            expires_in: value.expires_in,
            error: None,
            error_description: None,
        }
    }
}

#[uniffi::export]
pub fn gemini_updated_credentials(document: Vec<u8>, token: GoogleRefreshToken, now_millis: i64) -> Option<Vec<u8>> {
    let document: Value = serde_json::from_slice(&document).ok()?;
    Some(
        serde_json::to_vec(&gemini_refreshed_document(&document, &token.into(), now_millis)?)
            .expect("Gemini document is serializable"),
    )
}

#[cfg(test)]
#[path = "../../tests/unit/providers/google_credentials/tests.rs"]
mod tests;
