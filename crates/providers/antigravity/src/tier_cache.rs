//! Holds Antigravity tier details by credential until their refresh deadline.

use sha2::{Digest, Sha256};

pub const TIER_CACHE_TTL: f64 = 3600.0;

struct Entry<T> {
    fingerprint: [u8; 32],
    value: T,
    expires_at: f64,
}

pub struct TierCache<T> {
    entry: Option<Entry<T>>,
}

impl<T> Default for TierCache<T> {
    fn default() -> Self {
        Self { entry: None }
    }
}

impl<T: Clone> TierCache<T> {
    pub fn read(&self, token: &str, now: f64) -> Option<T> {
        let fingerprint: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        self.entry
            .as_ref()
            .filter(|entry| entry.fingerprint == fingerprint && entry.expires_at > now)
            .map(|entry| entry.value.clone())
    }

    pub fn record(&mut self, token: &str, value: T, now: f64) {
        self.entry = Some(Entry {
            fingerprint: Sha256::digest(token.as_bytes()).into(),
            value,
            expires_at: now + TIER_CACHE_TTL,
        });
    }
}

#[cfg(test)]
#[path = "../tests/unit/tier_cache/tests.rs"]
mod tests;
