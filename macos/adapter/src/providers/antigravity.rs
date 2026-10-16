//! Maps Antigravity responses through the shared projector.

mod cloud;
pub use cloud::{AntigravityCloudFetcher, AntigravityCloudOutcome, AntigravityCloudResult, AntigravityCloudSnapshot};
mod local;
mod local_callback;
pub use local::{
    AntigravityLocalInput, AntigravityLocalReading, AntigravityLocalResult, AntigravityLocalServer,
    AntigravityLocalSession, AntigravityLocalSnapshot, antigravity_local_probe,
};
pub use local_callback::AntigravityServerScanner;
#[cfg(target_os = "macos")]
mod native;
#[cfg(target_os = "macos")]
pub use native::AntigravityNativeScanner;

use crate::provider_record::ProviderRecord;
use quotaline_antigravity as core;
use quotaline_antigravity::request;

#[uniffi::export]
pub fn antigravity_server_headers(csrf_token: Option<String>) -> std::collections::HashMap<String, String> {
    request::server_headers(csrf_token.as_deref())
        .into_iter()
        .map(|(name, value)| (name.into(), value))
        .collect()
}

#[uniffi::export]
pub fn antigravity_server_quota_body() -> Vec<u8> {
    serde_json::to_vec(&request::server_quota_body()).expect("Antigravity quota request is serializable")
}

#[uniffi::export]
pub fn antigravity_server_status_body() -> Vec<u8> {
    serde_json::to_vec(&request::server_status_body()).expect("Antigravity status request is serializable")
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct AntigravityIdentity {
    pub plan_name: String,
    pub tier: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum AntigravityError {
    #[error("Invalid Antigravity response JSON")]
    InvalidJson,
    #[error("Antigravity session expired")]
    Unauthenticated,
    #[error("Antigravity quota request failed at the RPC layer")]
    RpcFailure,
    #[error("Antigravity response contains no quota groups")]
    MissingGroups,
}

impl From<core::QuotaError> for AntigravityError {
    fn from(value: core::QuotaError) -> Self {
        match value {
            core::QuotaError::Unauthenticated => Self::Unauthenticated,
            core::QuotaError::RpcFailure => Self::RpcFailure,
            core::QuotaError::MissingGroups => Self::MissingGroups,
        }
    }
}

#[derive(uniffi::Object)]
pub struct AntigravityQuotaResponse {
    document: serde_json::Value,
}

#[uniffi::export]
impl AntigravityQuotaResponse {
    #[uniffi::constructor]
    pub fn new(payload: Vec<u8>) -> Result<Self, AntigravityError> {
        let document = serde_json::from_slice(&payload).map_err(|_| AntigravityError::InvalidJson)?;
        core::validate(&document)?;
        Ok(Self { document })
    }

    /// The provider reading for this quota under the account's plan identity.
    pub fn provider(&self, identity: AntigravityIdentity) -> ProviderRecord {
        core::project_identified(&self.document, identity.into(), String::new())
            .expect("the quota was validated when it was read")
            .into()
    }
}

#[uniffi::export]
pub fn antigravity_identity(payload: Vec<u8>) -> Result<AntigravityIdentity, AntigravityError> {
    let document = serde_json::from_slice(&payload).map_err(|_| AntigravityError::InvalidJson)?;
    Ok(core::identity(&document).into())
}

impl From<AntigravityIdentity> for core::Identity {
    fn from(identity: AntigravityIdentity) -> Self {
        Self {
            plan_name: identity.plan_name,
            tier: identity.tier,
            email: identity.email,
        }
    }
}

impl From<core::Identity> for AntigravityIdentity {
    fn from(identity: core::Identity) -> Self {
        Self {
            plan_name: identity.plan_name,
            tier: identity.tier,
            email: identity.email,
        }
    }
}

#[uniffi::export]
pub fn antigravity_default_identity() -> AntigravityIdentity {
    let identity = core::identity(&serde_json::Value::Null);
    AntigravityIdentity {
        plan_name: identity.plan_name,
        tier: identity.tier,
        email: identity.email,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/providers/antigravity/tests.rs"]
mod tests;
