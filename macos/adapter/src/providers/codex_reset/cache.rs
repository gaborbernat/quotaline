//! Fetches account-scoped Codex reset credits through the shared cache, one request per account at a time.

use std::collections::HashMap;
use std::sync::Mutex;

use futures_channel::oneshot;
use quotaline_codex::codex_reset::{RESET_CREDIT_FAILURE_TTL, RESET_CREDIT_SUCCESS_TTL, ResetCreditCache};
use quotaline_codex::codex_usage::ResetCredits;
use quotaline_codex::fetch;

use crate::http::NativeTransport;

type Resolved = (Option<ResetCredits>, Option<String>);

#[derive(Default)]
struct CacheState {
    cache: ResetCreditCache<ResetCredits>,
    active: HashMap<String, Vec<oneshot::Sender<Resolved>>>,
    generation: u64,
}

#[derive(Default, uniffi::Object)]
pub struct CodexResetCreditCache(Mutex<CacheState>);

#[uniffi::export]
impl CodexResetCreditCache {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&self) {
        let mut state = self.0.lock().unwrap();
        state.cache.clear();
        state.active.clear();
        state.generation = state.generation.wrapping_add(1);
    }
}

pub(crate) struct Lookup<'a> {
    pub account: &'a str,
    pub reported: Option<ResetCredits>,
    pub now: f64,
    pub home: &'a str,
}

impl CodexResetCreditCache {
    /// Returns the account's reset credits and the warning to show when the lookup failed.
    pub(crate) async fn resolve(
        &self,
        lookup: Lookup<'_>,
        transport: &NativeTransport,
        request: &fetch::Account<'_>,
    ) -> Resolved {
        let (cached, waiting, generation) = {
            let mut state = self.0.lock().unwrap();
            let complete = lookup.reported.as_ref().is_some_and(ResetCredits::complete);
            let cached = state.cache.read(lookup.account, lookup.reported, complete, lookup.now);
            if !cached.should_fetch {
                return (cached.value, cached.warning);
            }
            let waiting = if let Some(waiters) = state.active.get_mut(lookup.account) {
                let (sender, receiver) = oneshot::channel();
                waiters.push(sender);
                Some(receiver)
            } else {
                state.active.insert(lookup.account.into(), vec![]);
                None
            };
            (cached, waiting, state.generation)
        };
        if let Some(receiver) = waiting {
            return receiver.await.unwrap_or((cached.value, cached.warning));
        }
        let pending = PendingResetCredits {
            cache: self,
            account: lookup.account,
            generation,
        };
        let resolved = match fetch::fetch_reset_credits(transport, request).await {
            Ok(credits) => (Some(credits), None),
            Err(error) => (
                cached.value,
                Some(fetch::reset_credits_warning(&error, Some(lookup.home))),
            ),
        };
        let waiters = {
            let mut state = self.0.lock().unwrap();
            if state.generation == generation {
                state
                    .cache
                    .record(lookup.account, resolved.0.clone(), resolved.1.clone(), lookup.now);
                state.active.remove(lookup.account).unwrap()
            } else {
                vec![]
            }
        };
        for waiter in waiters {
            let _ = waiter.send(resolved.clone());
        }
        drop(pending);
        resolved
    }
}

struct PendingResetCredits<'a> {
    cache: &'a CodexResetCreditCache,
    account: &'a str,
    generation: u64,
}

impl Drop for PendingResetCredits<'_> {
    fn drop(&mut self) {
        let mut state = self.cache.0.lock().unwrap();
        if state.generation == self.generation {
            state.active.remove(self.account);
        }
    }
}

#[uniffi::export]
pub fn codex_reset_credit_success_ttl() -> f64 {
    RESET_CREDIT_SUCCESS_TTL
}

#[uniffi::export]
pub fn codex_reset_credit_failure_ttl() -> f64 {
    RESET_CREDIT_FAILURE_TTL
}

#[cfg(test)]
#[path = "../../../tests/unit/providers/codex_reset/cache/tests.rs"]
mod tests;
