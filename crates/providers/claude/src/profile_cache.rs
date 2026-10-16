//! Holds Claude profile responses and their refresh deadlines.

pub const PROFILE_SUCCESS_TTL: f64 = 6.0 * 3600.0;
pub const PROFILE_FAILURE_TTL: f64 = 300.0;

struct Entry<T> {
    fingerprint: String,
    value: Option<T>,
    warning: Option<String>,
    expires_at: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileCacheRead<T> {
    pub value: Option<T>,
    pub warning: Option<String>,
    pub should_fetch: bool,
}

pub struct ProfileCache<T> {
    entry: Option<Entry<T>>,
}

impl<T> Default for ProfileCache<T> {
    fn default() -> Self {
        Self { entry: None }
    }
}

impl<T: Clone> ProfileCache<T> {
    pub fn read(&self, fingerprint: &str, now: f64) -> ProfileCacheRead<T> {
        let entry = self.entry.as_ref().filter(|entry| entry.fingerprint == fingerprint);
        ProfileCacheRead {
            value: entry.and_then(|entry| entry.value.clone()),
            warning: entry
                .filter(|entry| entry.expires_at > now)
                .and_then(|entry| entry.warning.clone()),
            should_fetch: entry.is_none_or(|entry| entry.expires_at <= now),
        }
    }

    pub fn record(&mut self, fingerprint: &str, value: Option<T>, warning: Option<String>, now: f64) {
        let ttl = if value.is_some() && warning.is_none() {
            PROFILE_SUCCESS_TTL
        } else {
            PROFILE_FAILURE_TTL
        };
        self.entry = Some(Entry {
            fingerprint: fingerprint.into(),
            value,
            warning,
            expires_at: now + ttl,
        });
    }

    pub fn record_attempt(&mut self, fingerprint: &str, result: Result<T, String>, now: f64) -> ProfileCacheRead<T> {
        match result {
            Ok(value) => self.record(fingerprint, Some(value), None, now),
            Err(detail) => {
                let cached = self.read(fingerprint, now).value;
                self.record(fingerprint, cached, Some(format!("Profile unavailable: {detail}")), now);
            }
        }
        self.read(fingerprint, now)
    }
}

#[cfg(test)]
#[path = "../tests/unit/profile_cache/tests.rs"]
mod tests;
