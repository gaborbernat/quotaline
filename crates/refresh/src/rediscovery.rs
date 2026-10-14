//! Decides when provider discovery should run again.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RediscoveryTrigger {
    ApplicationActivated,
    UserInitiated,
}

#[derive(Clone, Debug)]
pub struct RediscoveryPolicy {
    activation_interval: f64,
    last_discovery_at: Option<f64>,
}

impl RediscoveryPolicy {
    pub const DEFAULT_ACTIVATION_INTERVAL: f64 = 60.0;

    pub fn new(activation_interval: f64, last_discovery_at: Option<f64>) -> Self {
        Self {
            activation_interval,
            last_discovery_at,
        }
    }

    pub fn begin(&mut self, trigger: RediscoveryTrigger, now: f64) -> bool {
        if trigger == RediscoveryTrigger::ApplicationActivated
            && self
                .last_discovery_at
                .is_some_and(|last| now >= last && now - last < self.activation_interval)
        {
            return false;
        }
        self.last_discovery_at = Some(now);
        true
    }

    pub fn record_discovery(&mut self, now: f64) {
        self.last_discovery_at = Some(now);
    }
}

#[cfg(test)]
#[path = "../tests/unit/rediscovery/tests.rs"]
mod tests;
