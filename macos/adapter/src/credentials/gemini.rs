//! Exposes the shared Gemini credential projection to Swift.

use quotaline_credentials::GeminiCredentialDocument;
use quotaline_credentials::GeminiStorage as CoreGeminiStorage;
use quotaline_credentials::gemini_credential_document;
use quotaline_credentials::gemini_initial_document;
use quotaline_credentials::gemini_keychain_document;
use quotaline_credentials::gemini_storage_mode;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct GeminiCredentialRecord {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_at: Option<f64>,
    pub email: Option<String>,
    pub hosted_domain: Option<String>,
}

impl From<GeminiCredentialDocument> for GeminiCredentialRecord {
    fn from(value: GeminiCredentialDocument) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            id_token: value.id_token,
            expires_at: value.expires_at,
            email: value.email,
            hosted_domain: value.hosted_domain,
        }
    }
}

#[uniffi::export]
pub fn gemini_credential_record(payload: Vec<u8>) -> Option<GeminiCredentialRecord> {
    let document = serde_json::from_slice(&payload).ok()?;
    gemini_credential_document(&document).map(Into::into)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum GeminiStorage {
    File,
    Keychain,
}

impl From<CoreGeminiStorage> for GeminiStorage {
    fn from(value: CoreGeminiStorage) -> Self {
        match value {
            CoreGeminiStorage::File => Self::File,
            CoreGeminiStorage::Keychain => Self::Keychain,
        }
    }
}

#[uniffi::export]
pub fn gemini_credential_storage(value: Option<String>) -> GeminiStorage {
    gemini_storage_mode(value.as_deref()).into()
}

#[uniffi::export]
pub fn gemini_initial_credentials(
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_at_millis: Option<f64>,
) -> Vec<u8> {
    serde_json::to_vec(&gemini_initial_document(
        &access_token,
        refresh_token.as_deref(),
        id_token.as_deref(),
        expires_at_millis,
    ))
    .expect("Gemini credential document is serializable")
}

#[uniffi::export]
pub fn gemini_keychain_credentials(document: Vec<u8>, updated_at_millis: f64) -> Option<Vec<u8>> {
    let document = serde_json::from_slice(&document).ok()?;
    Some(
        serde_json::to_vec(&gemini_keychain_document(&document, updated_at_millis)?)
            .expect("Gemini Keychain document is serializable"),
    )
}

#[cfg(test)]
#[path = "../../tests/unit/credentials/gemini/tests.rs"]
mod tests;
