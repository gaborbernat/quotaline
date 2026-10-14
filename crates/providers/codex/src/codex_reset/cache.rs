//! Holds account-scoped Codex reset-credit responses and retry deadlines.

use std::collections::BTreeMap;

pub const RESET_CREDIT_SUCCESS_TTL: f64 = 900.0;
pub const RESET_CREDIT_FAILURE_TTL: f64 = 300.0;
const MAX_ACCOUNTS: usize = 8;

struct Entry<T> {
    value: Option<T>,
    warning: Option<String>,
    expires_at: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResetCreditCacheRead<T> {
    pub value: Option<T>,
    pub warning: Option<String>,
    pub should_fetch: bool,
}

pub struct ResetCreditCache<T> {
    entries: BTreeMap<String, Entry<T>>,
}

impl<T> Default for ResetCreditCache<T> {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

impl<T: Clone> ResetCreditCache<T> {
    pub fn read(&mut self, account: &str, inline: Option<T>, complete: bool, now: f64) -> ResetCreditCacheRead<T> {
        if complete {
            let value = inline.expect("complete reset credits contain a value");
            self.record(account, Some(value.clone()), None, now);
            return ResetCreditCacheRead {
                value: Some(value),
                warning: None,
                should_fetch: false,
            };
        }
        let cached = self.entries.get(account);
        if let Some(cached) = cached.filter(|cached| cached.expires_at > now) {
            return ResetCreditCacheRead {
                value: cached.value.clone().or(inline),
                warning: cached.warning.clone(),
                should_fetch: false,
            };
        }
        ResetCreditCacheRead {
            value: cached.and_then(|cached| cached.value.clone()).or(inline),
            warning: None,
            should_fetch: true,
        }
    }

    pub fn record(&mut self, account: &str, value: Option<T>, warning: Option<String>, now: f64) {
        if self.entries.len() >= MAX_ACCOUNTS && !self.entries.contains_key(account) {
            self.entries.clear();
        }
        let ttl = if value.is_some() && warning.is_none() {
            RESET_CREDIT_SUCCESS_TTL
        } else {
            RESET_CREDIT_FAILURE_TTL
        };
        self.entries.insert(
            account.into(),
            Entry {
                value,
                warning,
                expires_at: now + ttl,
            },
        );
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
#[path = "../../tests/unit/codex_reset/cache/tests.rs"]
mod tests;
