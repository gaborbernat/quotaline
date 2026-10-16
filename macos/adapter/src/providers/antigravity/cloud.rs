//! Fetches Antigravity cloud quotas and account tiers.

use std::sync::{Arc, Mutex};

use quotaline_antigravity::TierCache;
use quotaline_antigravity::request;

use super::AntigravityIdentity;
use crate::http::{NativeHttpTransport, NativeTransport, ProviderFailure};
use crate::provider_record::ProviderRecord;
use quotaline_antigravity as core;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AntigravityCloudSnapshot {
    pub provider: ProviderRecord,
    pub warning: Option<String>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum AntigravityCloudOutcome {
    Success(Box<AntigravityCloudSnapshot>),
    Failure(ProviderFailure),
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct AntigravityCloudResult {
    pub outcome: AntigravityCloudOutcome,
    pub debug_messages: Vec<String>,
}

#[derive(Default, uniffi::Object)]
pub struct AntigravityCloudFetcher(Mutex<TierCache<AntigravityIdentity>>);

#[uniffi::export]
impl AntigravityCloudFetcher {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn fetch(
        &self,
        token: String,
        now: f64,
        hint: String,
        home: String,
        transport: Arc<dyn NativeHttpTransport>,
    ) -> AntigravityCloudResult {
        let native = NativeTransport(transport);
        let quota = core::fetch::fetch_quota(&native, &request::CLOUD_BASES, &token, &hint, Some(&home)).await;
        let outcome = match quota.result {
            Ok((document, base)) => {
                let (identity, warning) = self.tier(&native, &token, now, &base, &home).await;
                AntigravityCloudOutcome::Success(Box::new(AntigravityCloudSnapshot {
                    provider: core::project_identified(
                        &document,
                        identity.unwrap_or_else(super::antigravity_default_identity).into(),
                        String::new(),
                    )
                    .expect("the cloud fetch validated the quota")
                    .into(),
                    warning,
                }))
            }
            Err(error) => AntigravityCloudOutcome::Failure(error.provider_failure(&hint, Some(&home))),
        };
        AntigravityCloudResult {
            outcome,
            debug_messages: quota.debug_messages,
        }
    }
}

impl AntigravityCloudFetcher {
    async fn tier(
        &self,
        transport: &NativeTransport,
        token: &str,
        now: f64,
        base: &str,
        home: &str,
    ) -> (Option<AntigravityIdentity>, Option<String>) {
        if let Some(identity) = self.0.lock().unwrap().read(token, now) {
            return (Some(identity), None);
        }
        match core::fetch::fetch_plan(transport, base, token, Some(home)).await {
            Ok(plan) => {
                let identity = AntigravityIdentity::from(core::identity(&plan));
                self.0.lock().unwrap().record(token, identity.clone(), now);
                (Some(identity), None)
            }
            Err(message) => (None, Some(format!("Plan details unavailable: {message}"))),
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/providers/antigravity/cloud/tests.rs"]
mod tests;
