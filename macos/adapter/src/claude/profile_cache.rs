//! Fetches Claude profiles through the shared cache and native HTTP transport.

use std::sync::{Arc, Mutex};

use crate::http::{NativeHttpTransport, NativeTransport};
use quotaline_claude as claude;
use quotaline_claude::PROFILE_SUCCESS_TTL;
use quotaline_claude::ProfileCache;
use quotaline_claude::request;

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ClaudeProfileResult {
    pub value: Option<Vec<u8>>,
    pub warning: Option<String>,
}

impl From<claude::ProfileCacheRead<Vec<u8>>> for ClaudeProfileResult {
    fn from(reading: claude::ProfileCacheRead<Vec<u8>>) -> Self {
        Self {
            value: reading.value,
            warning: reading.warning,
        }
    }
}

#[derive(Default, uniffi::Object)]
pub struct ClaudeProfileCache(Mutex<ProfileCache<Vec<u8>>>);

#[uniffi::export]
impl ClaudeProfileCache {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn fetch(
        &self,
        token: String,
        fingerprint: String,
        now: f64,
        home: String,
        transport: Arc<dyn NativeHttpTransport>,
    ) -> ClaudeProfileResult {
        let cached = self.0.lock().unwrap().read(&fingerprint, now);
        if !cached.should_fetch {
            return cached.into();
        }
        let result =
            claude::fetch::fetch_profile(&NativeTransport(transport), request::PROFILE_URL, &token, Some(&home))
                .await
                .map(|profile| serde_json::to_vec(&profile).expect("a JSON value serializes"));
        self.0.lock().unwrap().record_attempt(&fingerprint, result, now).into()
    }
}

#[uniffi::export]
pub fn claude_profile_cache_interval() -> f64 {
    PROFILE_SUCCESS_TTL
}

#[cfg(test)]
#[path = "../../tests/unit/claude/profile_cache/tests.rs"]
mod tests;
