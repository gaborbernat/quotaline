//! Exposes provider rediscovery state to the macOS UI.

use std::sync::Mutex;

use quotaline_refresh::RediscoveryPolicy;
use quotaline_refresh::RediscoveryTrigger;

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum ProviderRediscoveryTrigger {
    ApplicationActivated,
    UserInitiated,
}

#[derive(uniffi::Object)]
pub struct ProviderRediscoveryState {
    policy: Mutex<RediscoveryPolicy>,
}

#[uniffi::export]
impl ProviderRediscoveryState {
    #[uniffi::constructor]
    pub fn new(activation_interval: f64, last_discovery_at: Option<f64>) -> Self {
        Self {
            policy: Mutex::new(RediscoveryPolicy::new(activation_interval, last_discovery_at)),
        }
    }

    pub fn begin(&self, trigger: ProviderRediscoveryTrigger, now: f64) -> bool {
        self.policy.lock().unwrap().begin(
            match trigger {
                ProviderRediscoveryTrigger::ApplicationActivated => RediscoveryTrigger::ApplicationActivated,
                ProviderRediscoveryTrigger::UserInitiated => RediscoveryTrigger::UserInitiated,
            },
            now,
        )
    }

    pub fn record_discovery(&self, now: f64) {
        self.policy.lock().unwrap().record_discovery(now);
    }
}

#[uniffi::export]
pub fn default_provider_rediscovery_interval() -> f64 {
    RediscoveryPolicy::DEFAULT_ACTIVATION_INTERVAL
}

#[cfg(test)]
#[path = "../../tests/unit/refresh/rediscovery/tests.rs"]
mod tests;
