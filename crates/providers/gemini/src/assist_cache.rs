//! Holds Gemini assist responses until their quota-sensitive refresh deadline.

pub const ASSIST_BALANCE_TTL: f64 = 60.0;
pub const ASSIST_DEFAULT_TTL: f64 = 3600.0;

struct Entry<T> {
    fingerprint: String,
    value: T,
    expires_at: f64,
}

pub struct AssistCache<T> {
    entry: Option<Entry<T>>,
}

impl<T> Default for AssistCache<T> {
    fn default() -> Self {
        Self { entry: None }
    }
}

impl<T: Clone> AssistCache<T> {
    pub fn read(&self, fingerprint: &str, now: f64) -> Option<T> {
        self.entry
            .as_ref()
            .filter(|entry| entry.fingerprint == fingerprint && entry.expires_at > now)
            .map(|entry| entry.value.clone())
    }

    pub fn record(&mut self, fingerprint: &str, value: T, has_balances: bool, now: f64) {
        self.entry = Some(Entry {
            fingerprint: fingerprint.into(),
            value,
            expires_at: now
                + if has_balances {
                    ASSIST_BALANCE_TTL
                } else {
                    ASSIST_DEFAULT_TTL
                },
        });
    }
}

#[cfg(test)]
#[path = "../tests/unit/assist_cache/tests.rs"]
mod tests;
