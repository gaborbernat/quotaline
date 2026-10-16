//! Caches Cursor profile identities by token without retaining the token.

use sha2::{Digest, Sha256};

pub fn token_digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

pub const SUCCESS_TTL_SECONDS: f64 = 3_600.0;
pub const FAILURE_TTL_SECONDS: f64 = 300.0;

#[derive(Default)]
pub struct Cache {
    token_digest: Option<[u8; 32]>,
    expires_at: f64,
    email: Option<String>,
}

impl Cache {
    pub fn get(&self, token: &str, now: f64) -> Option<Option<String>> {
        if self.token_digest != Some(token_digest(token)) || now >= self.expires_at {
            return None;
        }
        Some(self.email.clone())
    }

    pub fn remember(&mut self, token: &str, email: Option<String>, now: f64) {
        self.token_digest = Some(token_digest(token));
        self.expires_at = now
            + if email.is_some() {
                SUCCESS_TTL_SECONDS
            } else {
                FAILURE_TTL_SECONDS
            };
        self.email = email;
    }
}

#[cfg(test)]
#[path = "../tests/unit/identity/tests.rs"]
mod tests;
