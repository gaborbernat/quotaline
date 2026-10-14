//! Fetches Codex usage and reset credits, confirming apparent resets before showing them.

use crate::codex_usage::{CodexAccountFacts, account_provider};
use crate::http::{NativeHttpTransport, NativeTransport, ProviderFailure};
use crate::provider_record::ProviderRecord;
use quotaline_codex::codex_reset::Guard;
use quotaline_codex::{fetch, request};
use quotaline_quota::Window;
use std::sync::{Arc, Mutex};

mod cache;
pub use cache::{CodexResetCreditCache, codex_reset_credit_failure_ttl, codex_reset_credit_success_ttl};

#[derive(uniffi::Object)]
pub struct CodexResetGuard {
    state: Mutex<Guard<Window>>,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct CodexUsageFetchInput {
    pub token: String,
    pub account_id: Option<String>,
    pub account: CodexAccountFacts,
    pub now: f64,
    pub hint: String,
    pub home: String,
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum CodexUsageFetchOutcome {
    Provider(Box<ProviderRecord>),
    Failure(ProviderFailure),
}

#[uniffi::export]
impl CodexResetGuard {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&self) {
        self.state.lock().unwrap().clear();
    }

    /// Fetches usage, then the account's reset credits, and returns one reading with any warnings attached.
    pub async fn fetch_usage(
        &self,
        input: CodexUsageFetchInput,
        reset_credits: Arc<CodexResetCreditCache>,
        transport: Arc<dyn NativeHttpTransport>,
    ) -> CodexUsageFetchOutcome {
        let transport = NativeTransport(transport);
        let request = fetch::Account {
            base: request::BASE_URL,
            token: &input.token,
            account_id: input.account_id.as_deref(),
        };
        let document = match fetch::fetch_usage(&transport, &request).await {
            Ok(document) => document,
            Err(error) => {
                return CodexUsageFetchOutcome::Failure(error.provider_failure(&input.hint, Some(&input.home)));
            }
        };
        let usage = quotaline_codex::codex_usage::usage(&document, input.now);
        let lookup = cache::Lookup {
            account: input.account.fingerprint.as_deref().unwrap_or_default(),
            reported: usage.reset_credits.clone(),
            now: input.now,
            home: &input.home,
        };
        let (credits, warning) = reset_credits.resolve(lookup, &transport, &request).await;
        let mut provider = account_provider(usage, credits, &input.account, input.now);
        provider.windows = self.confirm(provider.windows, input.now);
        provider.warnings.extend(warning);
        CodexUsageFetchOutcome::Provider(Box::new(provider.into()))
    }
}

impl CodexResetGuard {
    pub(crate) fn confirm(&self, windows: Vec<Window>, now: f64) -> Vec<Window> {
        self.state.lock().unwrap().confirm(windows, now)
    }
}

impl Default for CodexResetGuard {
    fn default() -> Self {
        Self {
            state: Mutex::new(Guard::default()),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/providers/codex_reset/tests.rs"]
mod tests;
