//! Keeps Cursor profile lookup state behind the native HTTP boundary.

use crate::http::{NativeHttpTransport, NativeTransport};
use futures_channel::oneshot;
use quotaline_cursor::fetch;
use quotaline_cursor::identity;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, uniffi::Record)]
pub struct CursorIdentityDurations {
    pub success: f64,
    pub failure: f64,
}

#[uniffi::export]
pub fn cursor_identity_durations() -> CursorIdentityDurations {
    CursorIdentityDurations {
        success: identity::SUCCESS_TTL_SECONDS,
        failure: identity::FAILURE_TTL_SECONDS,
    }
}

#[derive(Default)]
struct IdentityState {
    cache: identity::Cache,
    active: HashMap<[u8; 32], Vec<oneshot::Sender<Option<String>>>>,
    latest: Option<[u8; 32]>,
}

#[derive(Default, uniffi::Object)]
pub struct CursorIdentityFetcher {
    state: Mutex<IdentityState>,
}

#[uniffi::export]
impl CursorIdentityFetcher {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn email(
        &self,
        token: String,
        cookie: String,
        now: f64,
        transport: Arc<dyn NativeHttpTransport>,
    ) -> Option<String> {
        let digest = identity::token_digest(&token);
        let waiting = {
            let mut state = self.state.lock().unwrap();
            state.latest = Some(digest);
            if let Some(cached) = state.cache.get(&token, now) {
                return cached;
            }
            if let Some(waiters) = state.active.get_mut(&digest) {
                let (sender, receiver) = oneshot::channel();
                waiters.push(sender);
                Some(receiver)
            } else {
                state.active.insert(digest, vec![]);
                None
            }
        };
        if let Some(receiver) = waiting {
            return receiver.await.unwrap_or(None);
        }
        let pending = PendingIdentity { fetcher: self, digest };
        let email = fetch::fetch_email(&NativeTransport(transport), super::request::PROFILE_URL, &cookie).await;
        let waiters = {
            let mut state = self.state.lock().unwrap();
            if state.latest == Some(digest) {
                state.cache.remember(&token, email.clone(), now);
            }
            state.active.remove(&digest).unwrap()
        };
        for waiter in waiters {
            let _ = waiter.send(email.clone());
        }
        drop(pending);
        email
    }
}

struct PendingIdentity<'a> {
    fetcher: &'a CursorIdentityFetcher,
    digest: [u8; 32],
}

impl Drop for PendingIdentity<'_> {
    fn drop(&mut self) {
        if let Some(waiters) = self.fetcher.state.lock().unwrap().active.remove(&self.digest) {
            for waiter in waiters {
                let _ = waiter.send(None);
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/providers/cursor/identity/tests.rs"]
mod tests;
