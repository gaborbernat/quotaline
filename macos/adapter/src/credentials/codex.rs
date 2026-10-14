//! Exposes shared Codex credential source selection to Swift.

use quotaline_credential_sources::{CodexCredentialStoreMode as CoreMode, codex_credential_store_mode, codex_homes};
use quotaline_credentials::CodexCredentialDocument;
use quotaline_credentials::codex_credential_document;
use quotaline_credentials::codex_initial_document;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CodexCredentialRecord {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub account_id: Option<String>,
    pub api_key: Option<String>,
    pub last_refresh: Option<String>,
    pub email: Option<String>,
    pub plan_type: Option<String>,
    pub subscription_active_until: Option<String>,
    pub supports_usage_api: bool,
}

impl From<CodexCredentialDocument> for CodexCredentialRecord {
    fn from(value: CodexCredentialDocument) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            id_token: value.id_token,
            account_id: value.account_id,
            api_key: value.api_key,
            last_refresh: value.last_refresh,
            email: value.email,
            plan_type: value.plan_type,
            subscription_active_until: value.subscription_active_until,
            supports_usage_api: value.supports_usage_api,
        }
    }
}

#[uniffi::export]
pub fn codex_api_key_usage_message() -> String {
    quotaline_credentials::CODEX_API_KEY_USAGE_UNAVAILABLE.into()
}

#[uniffi::export]
pub fn codex_credential_record(payload: Vec<u8>) -> Option<CodexCredentialRecord> {
    let document = serde_json::from_slice(&payload).ok()?;
    codex_credential_document(&document).map(Into::into)
}

#[uniffi::export]
pub fn codex_initial_credentials(
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    account_id: Option<String>,
    last_refresh: Option<String>,
) -> Vec<u8> {
    serde_json::to_vec(&codex_initial_document(
        &access_token,
        refresh_token.as_deref(),
        id_token.as_deref(),
        account_id.as_deref(),
        last_refresh.as_deref(),
    ))
    .expect("Codex credential document is serializable")
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CodexCredentialStoreMode {
    Automatic,
    File,
    Keyring,
    Unknown(String),
}

impl From<CoreMode> for CodexCredentialStoreMode {
    fn from(value: CoreMode) -> Self {
        match value {
            CoreMode::Automatic => Self::Automatic,
            CoreMode::File => Self::File,
            CoreMode::Keyring => Self::Keyring,
            CoreMode::Unknown(value) => Self::Unknown(value),
        }
    }
}

#[uniffi::export]
pub fn codex_credential_mode(config: String) -> CodexCredentialStoreMode {
    codex_credential_store_mode(&config)
        .unwrap_or(CoreMode::Automatic)
        .into()
}

#[uniffi::export]
pub fn codex_credential_mode_at_path(path: String) -> CodexCredentialStoreMode {
    quotaline_credential_sources::read_codex_credential_store_mode(Path::new(&path))
        .unwrap_or(CoreMode::Automatic)
        .into()
}

#[uniffi::export]
pub fn codex_credential_homes(configured: String, home: String) -> Vec<String> {
    codex_homes(Path::new(&configured), Path::new(&home))
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/credentials/codex/tests.rs"]
mod tests;
