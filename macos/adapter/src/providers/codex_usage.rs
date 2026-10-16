//! Turns Codex usage, live or from local rollouts, into the shared provider reading.

use std::sync::Arc;

use crate::provider_record::ProviderRecord;
use quotaline_codex::codex_usage::{self, ResetCredits, Usage};
use quotaline_quota::Provider;

/// What the stored sign-in knows about the account when the response leaves it out.
#[derive(Clone, Debug, Default, PartialEq, uniffi::Record)]
pub struct CodexAccountFacts {
    #[uniffi(default = None)]
    pub fingerprint: Option<String>,
    #[uniffi(default = None)]
    pub plan_type: Option<String>,
    #[uniffi(default = None)]
    pub email: Option<String>,
    #[uniffi(default = None)]
    pub subscription_active_until: Option<String>,
}

pub(crate) fn account_provider(
    mut usage: Usage,
    reset_credits: Option<ResetCredits>,
    account: &CodexAccountFacts,
    now: f64,
) -> Provider {
    usage.plan_type = usage.plan_type.or_else(|| account.plan_type.clone());
    usage.email = usage.email.or_else(|| account.email.clone());
    let mut provider = usage.into_provider(reset_credits, String::new(), now);
    provider.account_fingerprint = account.fingerprint.clone();
    provider.subscription_active_until = account.subscription_active_until.clone();
    provider
}

/// The latest usage Codex CLI logged locally, shown when the usage API is unavailable.
#[derive(Debug, uniffi::Object)]
pub struct CodexRollout {
    usage: Usage,
    now: f64,
    observed_at: Option<f64>,
}

impl CodexRollout {
    pub(crate) fn new(data: &serde_json::Value, now: f64, observed_at: Option<f64>) -> Self {
        Self {
            usage: quotaline_codex::codex_usage::usage(data, now),
            now,
            observed_at,
        }
    }
}

#[uniffi::export]
impl CodexRollout {
    pub fn observed_at(&self) -> Option<f64> {
        self.observed_at
    }

    pub fn provider(&self, account: CodexAccountFacts) -> ProviderRecord {
        account_provider(self.usage.clone(), None, &account, self.now).into()
    }
}

#[uniffi::export]
pub fn codex_rollout(payload: Vec<u8>, now: f64) -> Option<Arc<CodexRollout>> {
    let document: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    let rollout = codex_usage::rollout(&document)?;
    Some(Arc::new(CodexRollout::new(&rollout.data, now, rollout.observed_at)))
}

#[uniffi::export]
pub fn codex_local_fallback(kind: crate::http::ProviderFailureKind) -> bool {
    quotaline_codex::local_fallback(kind)
}

#[cfg(test)]
#[path = "../../tests/unit/providers/codex_usage/tests.rs"]
mod tests;
