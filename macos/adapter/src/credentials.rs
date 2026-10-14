//! Exposes shared credential parsing to the macOS renderer.

mod claude;
mod claude_discovery;
mod codex;
mod codex_discovery;
mod copilot;
mod cursor;
mod file;
mod gemini;
mod keychain;
pub mod keychain_callback;
mod save_recovery;
mod source_chain;
pub mod source_chain_callback;

pub use claude::*;
pub use claude_discovery::*;
pub use codex::*;
pub use codex_discovery::*;
pub use copilot::*;
pub use cursor::*;
pub use file::*;
pub use gemini::*;
pub use keychain::*;
pub use keychain_callback::*;
use quotaline_credentials as credentials;
pub use save_recovery::{
    CredentialRecoveryError, CredentialRecoveryLedger, CredentialRecoveryLoadedProbe, CredentialRecoveryPending,
    CredentialRecoveryReadProbe, CredentialRecoveryResult, CredentialRecoverySave, CredentialRecoverySource,
    CredentialRecoveryStored,
};
pub use source_chain::*;
pub use source_chain_callback::*;

#[uniffi::export]
pub fn jwt_claims_json(token: String) -> Option<Vec<u8>> {
    serde_json::to_vec(&credentials::jwt_claims(&token)?).ok()
}

#[uniffi::export]
pub fn jwt_expiry(token: String) -> Option<f64> {
    credentials::jwt_expiry(&token)
}

#[uniffi::export]
pub fn credential_expiry_buffer() -> f64 {
    credentials::CREDENTIAL_EXPIRY_BUFFER
}

#[uniffi::export]
pub fn credential_expired(expires_at: Option<f64>, now: f64) -> bool {
    credentials::credential_expired(expires_at, now)
}

#[uniffi::export]
pub fn cursor_user_id(token: String) -> Option<String> {
    credentials::cursor_user_id(&token)
}

#[uniffi::export]
pub fn cursor_session_cookie(token: String) -> String {
    credentials::cursor_session_cookie(&token)
}

#[uniffi::export]
pub fn codex_account_fingerprint(access_token: String, account_id: Option<String>, id_token: Option<String>) -> String {
    credentials::codex_account_fingerprint(&access_token, account_id.as_deref(), id_token.as_deref())
}

#[uniffi::export]
pub fn copilot_host(value: String) -> Option<String> {
    credentials::copilot_host(&value)
}

#[uniffi::export]
pub fn codex_home_key(canonical_path: String) -> String {
    credentials::codex_home_key(&canonical_path)
}

#[uniffi::export]
pub fn claude_cache_fingerprint(access_token: String, refresh_token: Option<String>) -> String {
    hex::encode(credentials::claude_cache_fingerprint(
        &access_token,
        refresh_token.as_deref(),
    ))
}

#[uniffi::export]
pub fn gemini_cache_fingerprint(
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
) -> String {
    hex::encode(credentials::gemini_cache_fingerprint(
        &access_token,
        refresh_token.as_deref(),
        id_token.as_deref(),
    ))
}

#[uniffi::export]
pub fn claude_keychain_service(config_dir: Option<String>) -> String {
    credentials::claude_keychain_service(config_dir.as_deref())
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AntigravityCredential {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<f64>,
}

impl From<credentials::AntigravityCredential> for AntigravityCredential {
    fn from(value: credentials::AntigravityCredential) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            expires_at: value.expires_at,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum AntigravityCredentialProblem {
    InvalidBase64,
    InvalidJson,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AntigravityKeyringResult {
    pub credential: Option<AntigravityCredential>,
    pub problem: Option<AntigravityCredentialProblem>,
}

#[uniffi::export]
pub fn antigravity_keyring_credential(payload: Vec<u8>) -> AntigravityKeyringResult {
    match credentials::antigravity_keyring_document(&payload) {
        Ok(document) => AntigravityKeyringResult {
            credential: credentials::antigravity_credential(&document).map(Into::into),
            problem: None,
        },
        Err(error) => AntigravityKeyringResult {
            credential: None,
            problem: Some(match error {
                credentials::AntigravityDecodeError::InvalidBase64 => AntigravityCredentialProblem::InvalidBase64,
                credentials::AntigravityDecodeError::InvalidJson => AntigravityCredentialProblem::InvalidJson,
            }),
        },
    }
}

#[cfg(test)]
#[path = "../tests/unit/credentials/tests.rs"]
mod tests;
